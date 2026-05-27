use axum::{extract::State, http::StatusCode, response::Response};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    anthropic::UserSegment,
    artifact::GraphArtifactRef,
    bridge::tools::{ToolEntry, ToolKind, ToolStatus},
    constants::CTX_KEY_WEB_SEARCH,
    perplexity::{search::search, types::SearchParams, SearchMode},
    planner::{CompressedTool, NodeId},
    prompts::{PromptContext, PromptKind},
    state::{AppState, EnvironmentContext, PendingTool, RequestContext},
    validation::{GuardContext, ToolError, Transform, TransformCheck, TransformContext, Validate},
    workflow::{
        action::traits::ActionHandler,
        responder::{error_response, Dispatcher, ResponseKind},
        ToolResultPipeline,
    },
};

pub struct ToolHandler {
    pub tool_name: String,
    pub tool: Value,
    pub compressed: Option<CompressedTool>,
    pub node_id: NodeId,
    pub user_query: Vec<UserSegment>,
    pub canonical_args_hint: Option<Value>,
}

impl ToolHandler {
    async fn build_args(
        &self,
        state: &AppState,
        ctx: &RequestContext,
        env: &EnvironmentContext,
        artifact_refs: &[GraphArtifactRef],
    ) -> Result<Value, String> {
        let schema_props = self
            .tool
            .get("input_schema")
            .and_then(|s| s.get("properties"))
            .and_then(|p| p.as_object());

        if schema_props.map_or(true, |p| p.is_empty()) {
            tracing::debug!("[action] tool has no args, skipping resolve");
            return Ok(json!({}));
        }

        // [disabled] WebSearch skips arg resolution — uses raw user query text directly
        if matches!(
            ToolEntry::from_name(&self.tool_name).map(|e| &e.kind),
            Some(ToolKind::WebSearch)
        ) {
            let query = self
                .user_query
                .iter()
                .filter_map(|s| {
                    if let UserSegment::Text(t) = s {
                        Some(t.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            tracing::debug!(
                "[action] WebSearch bypasses arg resolution, query={:?}",
                query
            );
            return Ok(json!({ "query": query }));
        }

        let empty_compressed;
        let compressed_ref = match self.compressed.as_ref() {
            Some(c) => c,
            None => {
                empty_compressed = CompressedTool::default();
                &empty_compressed
            }
        };

        let resolved_refs: Vec<(GraphArtifactRef, String)> = if !artifact_refs.is_empty() {
            let workflow_id = state.workflow.lock().await.active().map(|w| w.id.clone());
            if let Some(wf_id) = workflow_id {
                let registry = state.artifact_registry.read().await;
                let mut refs = Vec::with_capacity(artifact_refs.len());
                for ref_name in artifact_refs {
                    match registry.resolve(&wf_id, ref_name) {
                        Some(artifact) => {
                            tracing::info!(
                                "[artifact] tool={} consuming {}",
                                self.tool_name,
                                artifact.id
                            );
                            refs.push((ref_name.clone(), artifact.content.clone()));
                        }
                        None => {
                            tracing::error!(
                                "[artifact] tool={} required ref '{}' not found in registry for workflow_id={}",
                                self.tool_name, ref_name, wf_id
                            );
                            drop(registry);
                            state.workflow.lock().await.reset();
                            return Err(format!(
                                "required artifact '{}' not found — cannot build prompt for tool '{}'",
                                ref_name, self.tool_name
                            ));
                        }
                    }
                }
                refs
            } else {
                tracing::error!(
                    "[artifact] tool={} requires refs={:?} but workflow_id is not set",
                    self.tool_name,
                    artifact_refs
                );
                return Err(format!(
                    "required artifact(s) {:?} cannot be resolved — no active workflow_id for tool '{}'",
                    artifact_refs, self.tool_name
                ));
            }
        } else {
            Vec::new()
        };

        let ctx_prompt = PromptContext {
            tool: Some(&self.tool),
            compressed: Some(compressed_ref),
            user_query: &self.user_query,
            env: Some(env),
            artifact_refs: &resolved_refs,
            args_hint: self.canonical_args_hint.as_ref(),
            prompts: &state.config.prompts,
        };

        let args_context_uuid = {
            let wf = state.workflow.lock().await;
            let node = wf.active().and_then(|aw| aw.graph.nodes.get(&self.node_id));
            let needs_search = node.is_some_and(|n| {
                n.required_artifact_refs
                    .iter()
                    .any(|r| r == "search_results")
            });
            if needs_search {
                wf.active()
                    .and_then(|aw| aw.sub_uuid(&format!("{}:WebSearch", CTX_KEY_WEB_SEARCH)))
            } else {
                None
            }
        };

        let args = match search(
            &mut *state.session.lock().await,
            &SearchParams {
                query: &ctx_prompt.build(PromptKind::Args),
                mode: &ctx.mode,
                model: &ctx.model,
                incognito: ctx.incognito,
                search_mode: &SearchMode::Strict,
                context_uuid: args_context_uuid,
            },
            |_| {},
        )
        .await
        .inspect(|_| {
            if let Some(uuid) = args_context_uuid {
                if let Ok(mut session) = state.session.try_lock() {
                    session.thread_states.remove(&uuid);
                }
            }
        }) {
            Ok(raw) => {
                let mut parsed: Value =
                    llm_json::loads(&raw, &Default::default()).unwrap_or(json!({}));
                if !parsed.is_object() {
                    return Err(format!("Args response was not a JSON object: {}", raw));
                }
                if let Some(obj) = parsed.as_object_mut() {
                    if let Some(allowed) = schema_props {
                        obj.retain(|k, _| allowed.contains_key(k));
                    }
                }
                tracing::debug!("[action] args={}", parsed);
                parsed
            }
            Err(e) => return Err(e.to_string()),
        };

        Ok(args)
    }
}

impl ActionHandler for ToolHandler {
    async fn execute(self, ctx: RequestContext, state: State<AppState>) -> Response {
        let node_id = self.node_id;
        match self.run(ctx, state.clone()).await {
            Ok(response) => response,
            Err(e) => {
                tracing::error!("[action] node={} failed: {}", node_id, e);
                state.workflow.lock().await.reset();
                let status = match &e {
                    ToolError::TransformMissingOld | ToolError::TransformInvalid(_) => {
                        state.0.artifact_registry.write().await.clear();
                        StatusCode::UNPROCESSABLE_ENTITY
                    }
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                };
                error_response(status, e.to_string())
            }
        }
    }
}

impl ToolHandler {
    async fn run(self, ctx: RequestContext, state: State<AppState>) -> Result<Response, ToolError> {
        let env = state.env.lock().await.clone();
        let (artifact_refs, guard_snapshot) = {
            let wf = state.workflow.lock().await;
            let refs = wf
                .active()
                .and_then(|w| w.graph.nodes.get(&self.node_id))
                .map(|n| n.required_artifact_refs.clone())
                .unwrap_or_default();
            let snapshot = wf
                .active()
                .map(|aw| (aw.graph.clone(), aw.completed_nodes.clone()));
            (refs, snapshot)
        };

        // Guard validation
        if let Some((graph, completed_nodes)) = guard_snapshot {
            if let Some(node) = graph.nodes.get(&self.node_id) {
                let cache = state.tool_cache.read().await;
                if let Some(cache) = cache.as_ref() {
                    node.validate(&GuardContext {
                        graph: &graph,
                        completed: &completed_nodes,
                        registry: &cache.registry,
                    })
                    .map_err(|e| {
                        tracing::error!("[guard] node={} guard failed: {}", self.node_id, e);
                        ToolError::GuardFailed(e)
                    })?;
                }
            }
        }

        // Build tool args
        let mut input = self
            .build_args(&state, &ctx, &env, &artifact_refs)
            .await
            .map_err(ToolError::ArgsBuildFailed)?;

        // Transform check (edit safety)
        let wf_id = state
            .workflow
            .lock()
            .await
            .active()
            .map(|w| w.id.clone())
            .unwrap();
        let content = {
            let registry = state.artifact_registry.read().await;
            registry
                .resolve(&wf_id, "file_content")
                .map(|a| a.content.clone())
        };

        match TransformCheck::from_args(&input, &artifact_refs, content.as_deref()) {
            TransformCheck::NotApplicable => {}
            TransformCheck::MissingOld => return Err(ToolError::TransformMissingOld),
            TransformCheck::Required { old, content } => {
                (Transform { old_string: &old })
                    .validate(&TransformContext {
                        file_content: &content,
                    })
                    .map_err(|e| ToolError::TransformInvalid(e.to_string()))?;
            }
        }

        // Merge canonical args hint
        if let Some(hint) = &self.canonical_args_hint {
            if let Some(obj) = hint.as_object() {
                for (key, val) in obj {
                    match key.as_str() {
                        "file_path" => {
                            input[key] = val.clone();
                        }
                        _ => {
                            if input.get(key).is_none() || input[key].is_null() {
                                input[key] = val.clone();
                            }
                        }
                    }
                }
            }
        }

        // Arg validation and normalization
        if let Some(entry) = ToolEntry::from_name(&self.tool_name) {
            entry.validate_args(&input).map_err(|e| {
                tracing::error!("[arg_validation] tool={} {}", self.tool_name, e);
                ToolError::ArgValidationFailed(e.to_string())
            })?;
            entry.normalize_args(&mut input);
        }

        tracing::info!(
            "[action] firing tool={} node_id={}",
            self.tool_name,
            self.node_id
        );
        tracing::debug!("[action] tool_input={}", input);

        // Bridge tool (WebSearch)
        let is_bridge_tool = ToolEntry::from_name(&self.tool_name)
            .map(|e| e.status == ToolStatus::Override)
            .unwrap_or(false);

        if is_bridge_tool {
            let query = input
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            tracing::info!(
                "[action] bridge executing tool={} query={:?}",
                self.tool_name,
                query
            );

            let pending = PendingTool {
                tool_name: self.tool_name.clone(),
                tool_input: input.clone(),
                node_id: self.node_id,
                user_query: self.user_query,
            };

            let search_prompt = state.config.prompts.web_search.replace("{query}", &query);

            let websearch_uuid =
                state.workflow.lock().await.active().and_then(|aw| {
                    aw.sub_uuid(&format!("{}:{}", CTX_KEY_WEB_SEARCH, self.tool_name))
                });

            let result = search(
                &mut *state.session.lock().await,
                &SearchParams {
                    query: &search_prompt,
                    mode: &ctx.mode,
                    model: &ctx.model,
                    incognito: ctx.incognito,
                    search_mode: &SearchMode::Web,
                    context_uuid: websearch_uuid,
                },
                |_| {},
            )
            .await;

            let tool_result = match result {
                Ok(content) => {
                    format!("Web search results for query: \"{}\"\n\n{}", query, content)
                }
                Err(e) => {
                    tracing::error!(
                        "[action] bridge tool={} search failed: {}",
                        self.tool_name,
                        e
                    );
                    format!("Web search failed for query \"{}\": {}", query, e)
                }
            };

            return Ok(
                Box::pin(ToolResultPipeline::handle(state, ctx, pending, tool_result)).await,
            );
        }

        // Native tool — emit ToolUse to client
        let tool_id = format!("tool_{}", Uuid::new_v4().simple());

        {
            let mut wf = state.workflow.lock().await;
            if let Some(aw) = wf.active_mut() {
                aw.pending_tool = Some(PendingTool {
                    tool_name: self.tool_name.clone(),
                    tool_input: input.clone(),
                    node_id: self.node_id,
                    user_query: self.user_query,
                });
            }
        }

        Ok(Dispatcher::new(ctx, state)
            .send(ResponseKind::ToolUse {
                id: tool_id,
                name: self.tool_name,
                input,
            })
            .await)
    }
}
