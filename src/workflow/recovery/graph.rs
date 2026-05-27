use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use axum::{extract::State, response::Response};
use uuid::Uuid;

use crate::{
    anthropic::{serialize_segments, UserSegment},
    bridge::tools::ToolEntry,
    constants::{CTX_KEY_PLANNER, MAX_CLASSIFIER_RETRIES},
    perplexity::{search::search, types::SearchParams, SearchMode},
    planner::{IntentPlan, TaskDecomposition},
    prompts::{PromptContext, PromptKind},
    state::{AppState, RequestContext},
    validation::{validate_intent, ContractContext, SemanticContext, Validate},
    workflow::{
        action::WorkflowAction,
        recovery::state::RetryState,
        responder::{Dispatcher, ResponseKind},
        state::WorkflowState,
    },
};

pub async fn retry_empty_graph(
    state: State<AppState>,
    ctx: RequestContext,
    user_query: Vec<UserSegment>,
    tools: Vec<serde_json::Value>,
    cwd: String,
    retries: u8,
) -> Response {
    if !RetryState::budget_exceeded(retries, MAX_CLASSIFIER_RETRIES) {
        {
            let mut wf = state.workflow.lock().await;
            if let Some(aw) = wf.active_mut() {
                aw.retry.classifier_retries += 1;
            }
            wf.set_recovering();
        }
        tracing::warn!("[recovery] retry_classifier attempt={}", retries + 1);
        let corrective = PromptContext {
            tool: None,
            compressed: None,
            user_query: &user_query,
            env: None,
            artifact_refs: &[],
            args_hint: None,
            prompts: &state.config.prompts,
        }
        .build(PromptKind::IntentClassify);
        let reclassified = search(
            &mut *state.session.lock().await,
            &SearchParams {
                query: &corrective,
                mode: &ctx.mode,
                model: &ctx.model,
                incognito: ctx.incognito,
                search_mode: &SearchMode::Strict,
                context_uuid: None,
            },
            |_| {},
        )
        .await;
        match reclassified {
            Ok(raw) => {
                if let Some(new_intent) = IntentPlan::parse(&raw) {
                    if let Ok(v) = validate_intent(&new_intent, &cwd) {
                        let cache = state.tool_cache.read().await;
                        let registry = &cache.as_ref().unwrap().registry;
                        if v.validate(&SemanticContext { registry }).is_ok() {
                            let decomposition = TaskDecomposition::from_plan(v.original.clone());
                            let retry_graph = decomposition.build(registry, &tools, &cwd);
                            drop(cache);
                            if let Ok(retry_graph) = retry_graph {
                                if retry_graph.validate().is_ok()
                                    && Validate::validate(&retry_graph, &ContractContext).is_ok()
                                {
                                    let first_id = retry_graph.ordering[0];
                                    let retry_tool_name =
                                        retry_graph.nodes[&first_id].tool_name.clone();
                                    let retry_hint = retry_graph.nodes[&first_id].args_hint.clone();
                                    if let Some(rt) =
                                        ToolEntry::find_in_tools(&retry_tool_name, &tools)
                                    {
                                        let rt = rt.clone();
                                        let rc =
                                            ToolEntry::resolve_compressed(&state, &retry_tool_name)
                                                .await;
                                        let wf_id2 = Uuid::new_v4().to_string();
                                        let retry_context_uuid = {
                                            let mut h = DefaultHasher::new();
                                            Hash::hash(&serialize_segments(&user_query), &mut h);
                                            let hash = format!("{:x}", h.finish());
                                            let session = state.session.lock().await;
                                            session
                                                .context_uuids
                                                .get(&(hash, CTX_KEY_PLANNER.to_string()))
                                                .copied()
                                        };
                                        tracing::info!("[workflow] started workflow_id={}", wf_id2);
                                        *state.workflow.lock().await = WorkflowState::start(
                                            wf_id2,
                                            retry_graph,
                                            retry_context_uuid,
                                        );
                                        return WorkflowAction::FireTool {
                                            tool_name: retry_tool_name,
                                            tool: rt,
                                            compressed: rc,
                                            node_id: first_id,
                                            user_query,
                                            canonical_args_hint: retry_hint,
                                        }
                                        .execute(ctx, state)
                                        .await;
                                    }
                                }
                            }
                        } else {
                            drop(cache);
                        }
                    }
                }
            }
            Err(e) => tracing::error!("[recovery] classifier retry failed: {}", e),
        }
    }

    tracing::warn!("[recovery] empty graph after retries — falling back to plain text");
    state.workflow.lock().await.reset();
    let query = user_query
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
    let context_uuid = {
        let mut h = DefaultHasher::new();
        Hash::hash(&serialize_segments(&user_query), &mut h);
        let hash = format!("{:x}", h.finish());
        let session = state.session.lock().await;
        session
            .context_uuids
            .get(&(hash, CTX_KEY_PLANNER.to_string()))
            .copied()
    };
    Dispatcher::new(ctx, state)
        .send(ResponseKind::Text {
            query,
            context_uuid,
        })
        .await
}
