use std::path::Path;

use crate::{
    bridge::ToolEntry,
    constants::ARTIFACT_NAME_SNIPPET_LEN,
    planner::NodeId,
    state::AppState,
    validation::RuntimeError,
    workflow::{
        recovery::{classify_failure, RecoveryClass},
        result::types::{GraphPosition, PipelineContext, ToolResultKind},
    },
};

pub fn detect_kind(result: &str) -> ToolResultKind {
    if result.starts_with("Wasted call") {
        ToolResultKind::CacheHit
    } else if result.contains("The tool use was rejected") {
        ToolResultKind::Cancelled
    } else if result.contains("<tool_use_error>") {
        ToolResultKind::Error
    } else {
        ToolResultKind::Success
    }
}

pub async fn resolve_position(state: &AppState, node_id: NodeId) -> GraphPosition {
    let wf = state.workflow.lock().await;
    let aw = match wf.active() {
        Some(a) => a,
        None => return GraphPosition::OutsideGraph,
    };
    if !aw.graph.nodes.contains_key(&node_id) {
        return GraphPosition::OutsideGraph;
    }
    let pos = aw.graph.ordering.iter().position(|id| *id == node_id);
    match pos.and_then(|p| aw.graph.ordering.get(p + 1)) {
        Some(&next_node_id) => GraphPosition::MidGraph { next_node_id },
        None => GraphPosition::Terminal,
    }
}

pub fn artifact_name(ctx: &PipelineContext) -> String {
    ctx.tool_input
        .get("file_path")
        .and_then(|v| v.as_str())
        .map(|p| {
            Path::new(p)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(p)
                .to_string()
        })
        .unwrap_or_else(|| {
            format!(
                "{}:{}",
                ctx.tool_name,
                ctx.tool_result
                    .chars()
                    .take(ARTIFACT_NAME_SNIPPET_LEN)
                    .collect::<String>()
            )
            .chars()
            .take(ARTIFACT_NAME_SNIPPET_LEN)
            .collect()
        })
}

pub fn recovery_class(ctx: &PipelineContext) -> RecoveryClass {
    ToolEntry::from_name(&ctx.tool_name)
        .map(|e| {
            let arg_err = e.validate_args(&ctx.tool_input).err();
            let runtime_err = RuntimeError::from_str(&ctx.tool_result);
            classify_failure(&e.kind, arg_err.as_ref(), runtime_err.as_ref())
        })
        .unwrap_or(RecoveryClass::Terminal)
}
