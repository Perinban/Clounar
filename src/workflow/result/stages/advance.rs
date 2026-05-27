use axum::{extract::State, http::StatusCode};

use crate::{
    artifact::registry::NodeEmit,
    bridge::tools::ToolEntry,
    workflow::{
        action::WorkflowAction,
        recovery::{self, retry::RecoveryClass},
        result::{
            traits::{Stage, StageOutcome},
            types::{GraphPosition, PipelineContext, ToolResultKind},
            utils::{artifact_name, recovery_class},
        },
    },
};

pub struct Advance;

enum ErrorPath {
    RetryEdit {
        edit_retries: u8,
    },
    RetryTool {
        tool_retries: u8,
        next_tool_name: String,
    },
    Advance,
}

impl Stage for Advance {
    async fn run(&self, ctx: &PipelineContext) -> StageOutcome {
        let next_node_id = match ctx.position {
            GraphPosition::MidGraph { next_node_id } => next_node_id,
            _ => return StageOutcome::Continue,
        };

        if matches!(ctx.kind, ToolResultKind::Cancelled) {
            tracing::info!(
                "[tool_result] cancelled mid-graph: tool={} node_id={} — resetting",
                ctx.tool_name,
                ctx.node_id
            );
            ctx.state.workflow.lock().await.reset();
            ctx.state.artifact_registry.write().await.clear();
            return StageOutcome::Done(axum::response::IntoResponse::into_response((
                StatusCode::OK,
                axum::Json(serde_json::json!({})),
            )));
        }

        if matches!(ctx.kind, ToolResultKind::Error) {
            let error_path = {
                let wf = ctx.state.workflow.lock().await;
                let is_edit_class = wf
                    .active()
                    .and_then(|aw| aw.graph.nodes.get(&ctx.node_id))
                    .map(|n| n.required_artifact_refs.iter().any(|r| r == "file_content"))
                    .unwrap_or(false);
                if is_edit_class {
                    ErrorPath::RetryEdit {
                        edit_retries: wf
                            .active()
                            .map(|aw| aw.retry.get_edit_retries(ctx.node_id))
                            .unwrap_or(u8::MAX),
                    }
                } else {
                    match wf.active().and_then(|aw| {
                        let current = aw.graph.nodes.get(&ctx.node_id)?;
                        let next = aw.graph.nodes.get(&next_node_id)?;
                        let blocking = next.required_artifact_refs.iter().any(|req| {
                            current
                                .produces_artifacts
                                .iter()
                                .any(|a| a.ref_name == *req)
                        });
                        if blocking {
                            Some((
                                aw.retry.get_tool_retries(ctx.node_id),
                                aw.graph.nodes[&next_node_id].tool_name.clone(),
                            ))
                        } else {
                            None
                        }
                    }) {
                        Some((tool_retries, next_tool_name)) => ErrorPath::RetryTool {
                            tool_retries,
                            next_tool_name,
                        },
                        None => ErrorPath::Advance,
                    }
                }
            };

            let rc = recovery_class(ctx);
            match error_path {
                ErrorPath::RetryEdit { edit_retries } => {
                    tracing::warn!("[tool_result] Edit failed: {}", ctx.tool_result);
                    let resp = recovery::tool::retry_edit(
                        State(ctx.state.clone()),
                        ctx.ctx.clone(),
                        ctx.tool_input.clone(),
                        ctx.tool_result.clone(),
                        ctx.node_id,
                        ctx.user_query.clone(),
                        if matches!(rc, RecoveryClass::RetryEdit) {
                            edit_retries
                        } else {
                            u8::MAX
                        },
                    )
                    .await
                    .unwrap();
                    return StageOutcome::Done(resp);
                }
                ErrorPath::RetryTool {
                    tool_retries,
                    next_tool_name,
                } => {
                    tracing::warn!(
                        "[tool_result] aborting graph: tool={} failed and next tool={} requires its output",
                        ctx.tool_name,
                        next_tool_name,
                    );
                    return StageOutcome::Done(
                        recovery::tool::retry_tool(
                            State(ctx.state.clone()),
                            ctx.ctx.clone(),
                            recovery::tool::RetryToolParams {
                                tool_name: ctx.tool_name.clone(),
                                tool_input: ctx.tool_input.clone(),
                                _tool_result: ctx.tool_result.clone(),
                                next_tool_name,
                                node_id: ctx.node_id,
                                user_query: ctx.user_query.clone(),
                                tool_retries: if matches!(rc, RecoveryClass::RetryTool) {
                                    tool_retries
                                } else {
                                    u8::MAX
                                },
                            },
                        )
                        .await,
                    );
                }
                ErrorPath::Advance => {}
            }
        }

        let (next_tool_name, next_canonical_args_hint, specs, workflow_id) = {
            let wf = ctx.state.workflow.lock().await;
            let aw = match wf.active() {
                Some(a) => a,
                None => return StageOutcome::Continue,
            };
            (
                aw.graph.nodes[&next_node_id].tool_name.clone(),
                aw.graph
                    .nodes
                    .get(&next_node_id)
                    .and_then(|n| n.args_hint.clone()),
                aw.graph
                    .nodes
                    .get(&ctx.node_id)
                    .map(|n| {
                        n.produces_artifacts
                            .iter()
                            .map(|s| (s.ref_name.clone(), s.kind.as_ref_name()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
                aw.id.clone(),
            )
        };

        if !specs.is_empty() {
            let rebound = if matches!(ctx.kind, ToolResultKind::CacheHit) {
                if let Some(fp) = ctx.tool_input.get("file_path").and_then(|v| v.as_str()) {
                    let mut reg = ctx.state.artifact_registry.write().await;
                    specs
                        .iter()
                        .all(|(ref_name, _)| reg.rebind(ref_name, fp, &workflow_id))
                } else {
                    false
                }
            } else {
                false
            };
            if !rebound {
                tracing::debug!(
                    "[advance] emitting artifact for node={} tool={} content_preview={:?}",
                    ctx.node_id,
                    ctx.tool_name,
                    &ctx.tool_result[..ctx.tool_result.len().min(120)]
                );
                ctx.state
                    .artifact_registry
                    .write()
                    .await
                    .emit_for_node(NodeEmit {
                        specs: &specs,
                        node_id: ctx.node_id,
                        tool_name: &ctx.tool_name,
                        artifact_name: &artifact_name(ctx),
                        content: &ctx.tool_result,
                        workflow_id: &workflow_id,
                        tool_input: &ctx.tool_input,
                    });
            }
        }
        {
            let mut wf = ctx.state.workflow.lock().await;
            if let Some(aw) = wf.active_mut() {
                aw.completed_nodes.insert(ctx.node_id);
            }
        }

        let tools = ctx.state.tools.read().await.clone();
        let next_tool = match ToolEntry::resolve(
            &next_tool_name,
            &tools,
            StatusCode::BAD_GATEWAY,
            format!("Tool not found: {}", next_tool_name),
        ) {
            Ok(t) => t,
            Err(r) => {
                ctx.state.workflow.lock().await.reset();
                return StageOutcome::Done(*r);
            }
        };
        let next_compressed = ToolEntry::resolve_compressed(&ctx.state, &next_tool_name).await;

        tracing::info!(
            "[tool_result] advancing graph: next_node_id={} tool={}",
            next_node_id,
            next_tool_name
        );

        StageOutcome::Done(
            Box::pin(
                WorkflowAction::FireTool {
                    tool_name: next_tool_name,
                    tool: next_tool,
                    compressed: next_compressed,
                    node_id: next_node_id,
                    user_query: ctx.user_query.clone(),
                    canonical_args_hint: next_canonical_args_hint,
                }
                .execute(ctx.ctx.clone(), State(ctx.state.clone())),
            )
            .await,
        )
    }
}
