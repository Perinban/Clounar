use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fs,
    hash::{Hash, Hasher},
    path::Path,
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};

use crate::{
    anthropic::UserSegment,
    bridge::tools::{ToolEntry, ToolStatus},
    constants::CACHE_FILE,
    perplexity::{search::search, session::PerplexitySession, types::SearchParams, SearchMode},
    planner::{CapabilityKind, CapabilityRegistry, CompressedTool, ToolCapability},
    prompts::{PromptContext, PromptKind, PromptsConfig},
    state::RequestContext,
};

#[derive(Serialize, Deserialize, Clone)]
struct CachedEntry {
    hash: String,
    compressed: CompressedTool,
}

enum CacheStatus {
    Hit(CompressedTool),
    Miss,
}

#[derive(Clone)]
pub struct CachedToolEntry {
    pub compressed: CompressedTool,
    pub capability: Option<ToolCapability>,
}

pub struct ToolCache {
    pub map: HashMap<String, CachedToolEntry>,
    pub registry: CapabilityRegistry,
}

impl ToolCache {
    pub async fn init(
        tool_cache: &Arc<RwLock<Option<ToolCache>>>,
        session: &Arc<Mutex<PerplexitySession>>,
        ctx: &RequestContext,
        tools: &[Value],
        clounar_dir: &Path,
        prompts: &PromptsConfig,
    ) -> anyhow::Result<()> {
        if tool_cache.read().await.is_some() {
            return Ok(());
        }

        let cache_path = clounar_dir.join(CACHE_FILE);
        let mut stored: HashMap<String, CachedEntry> = fs::read_to_string(&cache_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let mut map = HashMap::new();
        let session = session.clone();
        let mode = ctx.mode.clone();
        let model = ctx.model.clone();
        let incognito = ctx.incognito;

        for tool in tools.iter().filter(|t| {
            t.get("name")
                .and_then(|v| v.as_str())
                .and_then(|n| ToolEntry::from_name(n))
                .map(|e| e.status != ToolStatus::Excluded)
                .unwrap_or(true)
        }) {
            let Some(name) = tool.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            let name = name.to_string();

            let hash = {
                let mut h = DefaultHasher::new();
                tool.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .hash(&mut h);
                let mut prop_names: Vec<&str> = tool
                    .get("input_schema")
                    .and_then(|s| s.get("properties"))
                    .and_then(|p| p.as_object())
                    .map(|m| m.keys().map(|k| k.as_str()).collect())
                    .unwrap_or_default();
                prop_names.sort_unstable();
                prop_names.hash(&mut h);
                format!("{:x}", h.finish())
            };

            let status = stored
                .get(&name)
                .filter(|e| e.hash == hash)
                .map(|e| CacheStatus::Hit(e.compressed.clone()))
                .unwrap_or(CacheStatus::Miss);

            let capability = ToolCapability::try_from(tool).ok();

            match status {
                CacheStatus::Hit(compressed) => {
                    tracing::debug!("[cache] hit tool={}", name);
                    map.insert(
                        name,
                        CachedToolEntry {
                            compressed,
                            capability,
                        },
                    );
                }
                CacheStatus::Miss => {
                    tracing::debug!("[cache] miss tool={} — compressing", name);
                    let prompt = PromptContext {
                        tool: Some(tool),
                        compressed: None,
                        user_query: &[] as &[UserSegment],
                        env: None,
                        artifact_refs: &[],
                        args_hint: None,
                        prompts,
                    }
                    .build(PromptKind::Compress);
                    let session = session.clone();
                    let mode = mode.clone();
                    let model = model.clone();
                    match search(
                        &mut *session.lock().await,
                        &SearchParams {
                            query: &prompt,
                            mode: &mode,
                            model: &model,
                            incognito,
                            search_mode: &SearchMode::Strict,
                            context_uuid: None,
                        },
                        |_| {},
                    )
                    .await
                    {
                        Ok(raw) => {
                            let compressed: CompressedTool = serde_json::from_value(
                                llm_json::loads(&raw, &Default::default()).unwrap_or_default(),
                            )
                            .unwrap_or_default();
                            tracing::debug!(
                                "[cache] compressed tool={} capabilities={}",
                                name,
                                compressed.capabilities.len(),
                            );
                            stored.insert(
                                name.clone(),
                                CachedEntry {
                                    hash,
                                    compressed: compressed.clone(),
                                },
                            );
                            map.insert(
                                name,
                                CachedToolEntry {
                                    compressed,
                                    capability,
                                },
                            );
                        }
                        Err(e) => {
                            tracing::error!("Compression failed for {}: {}", name, e);
                            map.insert(
                                name,
                                CachedToolEntry {
                                    compressed: CompressedTool::default(),
                                    capability,
                                },
                            );
                        }
                    };
                }
            }
        }

        if let Ok(json) = serde_json::to_string_pretty(&stored) {
            if let Err(e) = fs::write(&cache_path, json) {
                tracing::warn!("[cache] failed to persist cache to disk: {}", e);
            }
        }

        let registry = CapabilityRegistry::build(
            map.iter()
                .filter_map(|(name, entry)| entry.capability.clone().map(|c| (name.clone(), c))),
        );

        let matched = map.values().filter(|e| e.capability.is_some()).count();
        tracing::info!(
            "[cache] registry built: total={} matched={} unmatched={} read={} write={} execute={} interact={}",
            map.len(),
            matched,
            map.len() - matched,
            registry.by_kind.get(&CapabilityKind::Read).map_or(0, |v| v.len()),
            registry.by_kind.get(&CapabilityKind::Write).map_or(0, |v| v.len()),
            registry.by_kind.get(&CapabilityKind::Execute).map_or(0, |v| v.len()),
            registry.by_kind.get(&CapabilityKind::Interact).map_or(0, |v| v.len()),
        );
        for (name, entry) in &map {
            if entry.capability.is_none() {
                tracing::debug!(
                    "[cache] tool has no capability mapping, excluded from registry: tool={}",
                    name
                );
            }
        }

        let mut cache = tool_cache.write().await;
        if cache.is_none() {
            *cache = Some(Self { map, registry });
        }
        Ok(())
    }
}
