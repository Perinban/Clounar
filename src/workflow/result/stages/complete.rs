use std::path::Path;

use axum::extract::State;

use crate::{
    artifact::{registry::NodeEmit, ArtifactKind},
    planner::NodeKind,
    workflow::{
        recovery::{self, RecoveryClass},
        result::{
            traits::{Stage, StageOutcome},
            types::{ExecutionSignal, GraphPosition, PipelineContext, ToolResultKind},
            utils::{artifact_name, recovery_class},
        },
    },
};

pub struct Complete;

impl Stage for Complete {
    async fn run(&self, ctx: &PipelineContext) -> StageOutcome {
        if !matches!(ctx.position, GraphPosition::Terminal) {
            return StageOutcome::Continue;
        }

        if matches!(ctx.kind, ToolResultKind::Error) {
            let (edit_retries, is_edit_class) = {
                let wf = ctx.state.workflow.lock().await;
                let is_edit = wf
                    .active()
                    .and_then(|aw| aw.graph.nodes.get(&ctx.node_id))
                    .map(|n| n.required_artifact_refs.iter().any(|r| r == "file_content"))
                    .unwrap_or(false);
                let retries = wf
                    .active()
                    .map(|aw| aw.retry.get_edit_retries(ctx.node_id))
                    .unwrap_or(u8::MAX);
                (retries, is_edit)
            };
            if is_edit_class {
                tracing::warn!(
                    "[tool_result] Edit failed at terminal node: tool={} node_id={}",
                    ctx.tool_name,
                    ctx.node_id
                );
                let rc = recovery_class(ctx);
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
        }

        if matches!(ctx.kind, ToolResultKind::Cancelled) {
            tracing::info!(
                "[tool_result] cancelled at last node: tool={} node_id={} — resetting",
                ctx.tool_name,
                ctx.node_id
            );
            ctx.state.workflow.lock().await.reset();
            ctx.state.artifact_registry.write().await.clear();
            return StageOutcome::Done(axum::response::IntoResponse::into_response((
                axum::http::StatusCode::OK,
                axum::Json(serde_json::json!({})),
            )));
        }

        let mut file_snapshot_ref: Option<String> = None;
        let mut completed_workflow_id: Option<String> = None;
        {
            let wf = ctx.state.workflow.lock().await;
            if let Some(aw) = wf.active() {
                if let Some(node) = aw.graph.nodes.get(&ctx.node_id) {
                    if matches!(node.kind, NodeKind::Interaction)
                        && matches!(ctx.kind, ToolResultKind::Success)
                    {
                        let description = Self::resolve_desc(&ctx.tool_result);
                        drop(wf);
                        ctx.state.workflow.lock().await.pause_at(ctx.node_id);
                        *ctx.signal.lock().await = ExecutionSignal::PauseForInteraction {
                            node_id: ctx.node_id,
                            description,
                        };
                        return StageOutcome::Continue;
                    }
                }
                let specs = aw
                    .graph
                    .nodes
                    .get(&ctx.node_id)
                    .map(|n| {
                        n.produces_artifacts
                            .iter()
                            .map(|s| (s.ref_name.clone(), s.kind.as_ref_name()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                file_snapshot_ref = aw.graph.nodes.get(&ctx.node_id).and_then(|n| {
                    n.produces_artifacts
                        .iter()
                        .find(|s| s.kind == ArtifactKind::FileSnapshot)
                        .map(|s| s.ref_name.clone())
                });
                completed_workflow_id = Some(aw.id.clone());
                drop(wf);
                if !specs.is_empty() {
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
                            workflow_id: completed_workflow_id.as_deref().unwrap(),
                            tool_input: &ctx.tool_input,
                        });
                }
            }
        }
        ctx.state.workflow.lock().await.reset();

        if let (Some(ref_name), Some(wf_id)) = (file_snapshot_ref, completed_workflow_id) {
            let registry = ctx.state.artifact_registry.read().await;
            if let Some(abs_path) = registry
                .resolve(&wf_id, &ref_name)
                .and_then(|a| a.metadata.file_path.as_deref())
            {
                let mut env = ctx.state.env.lock().await;
                let cwd = env.cwd.clone();
                let abs = Path::new(abs_path);
                let rel = abs.strip_prefix(&cwd).unwrap_or(abs);
                env.last_resolved_dir = rel
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .filter(|s| !s.is_empty());
                if let Some(name) = rel.file_name().map(|n| n.to_string_lossy().into_owned()) {
                    let rel_str = rel.to_string_lossy().into_owned();
                    let entries = env.file_index.entry(name).or_default();
                    if !entries.contains(&rel_str) {
                        entries.push(rel_str);
                    }
                }
            }
        }

        tracing::info!("[tool_result] graph complete: returning text response");
        ctx.state.artifact_registry.write().await.clear();
        StageOutcome::Continue
    }
}

impl Complete {
    fn extract_label(tool_result: &str) -> &str {
        let bytes = tool_result.as_bytes();
        let mut pos = None;
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] == b'=' && bytes[i + 1] == b'"' {
                pos = Some(i + 2);
            }
            i += 1;
        }
        if let Some(start) = pos {
            let rest = &tool_result[start..];
            if let Some(end) = rest.find('"') {
                let label = &rest[..end];
                if !label.is_empty() {
                    return label;
                }
            }
        }
        tool_result.trim()
    }

    fn resolve_desc(tool_result: &str) -> String {
        Self::extract_label(tool_result).to_string()
    }
}
