use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use axum::{extract::State, http::StatusCode, response::Response};
use serde_json::Value;

use crate::{
    anthropic::{serialize_segments, UserSegment},
    bridge::tools::ToolEntry,
    constants::{CTX_KEY_PLANNER, MAX_EDIT_RETRIES, MAX_TOOL_RETRIES},
    planner::NodeId,
    state::{AppState, RequestContext},
    workflow::{action::WorkflowAction, recovery::state::RetryState},
};

pub struct RetryToolParams {
    pub tool_name: String,
    pub tool_input: Value,
    pub _tool_result: String,
    pub next_tool_name: String,
    pub node_id: NodeId,
    pub user_query: Vec<UserSegment>,
    pub tool_retries: u8,
}

pub async fn retry_edit(
    state: State<AppState>,
    ctx: RequestContext,
    tool_input: Value,
    tool_result: String,
    node_id: NodeId,
    user_query: Vec<UserSegment>,
    edit_retries: u8,
) -> Option<Response> {
    if !RetryState::budget_exceeded(edit_retries, MAX_EDIT_RETRIES) {
        {
            let mut wf = state.workflow.lock().await;
            if let Some(aw) = wf.active_mut() {
                aw.retry.increment_edit_retries(node_id);
            }
            wf.set_recovering();
        }
        tracing::warn!(
            "[recovery] retry_edit node={} attempt={}",
            node_id,
            edit_retries + 1
        );
        let file_path = tool_input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let read_node_id = {
            let wf = state.workflow.lock().await;
            wf.active()
                .and_then(|aw| {
                    aw.graph.nodes.values().find(|n| {
                        n.id != node_id
                            && n.produces_artifacts
                                .iter()
                                .any(|s| s.ref_name == "file_content")
                    })
                })
                .map(|n| n.id)
                .unwrap_or(node_id)
        };
        let tools_snap = state.tools.read().await.clone();
        if let Some(read_tool) = ToolEntry::find_in_tools("Read", &tools_snap) {
            let read_compressed = ToolEntry::resolve_compressed(&state, "Read").await;
            return Some(
                Box::pin(
                    WorkflowAction::FireTool {
                        tool_name: "Read".to_string(),
                        tool: read_tool.clone(),
                        compressed: read_compressed,
                        node_id: read_node_id,
                        user_query,
                        canonical_args_hint: Some(serde_json::json!({ "file_path": file_path })),
                    }
                    .execute(ctx, state),
                )
                .await,
            );
        }
    }
    state.workflow.lock().await.reset();
    let context_uuid = {
        let query_str = serialize_segments(&user_query);
        let mut h = DefaultHasher::new();
        query_str.hash(&mut h);
        let hash = format!("{:x}", h.finish());
        let session = state.session.lock().await;
        session
            .context_uuids
            .get(&(hash, CTX_KEY_PLANNER.to_string()))
            .copied()
    };
    Some(
        Box::pin(
            WorkflowAction::RespondWithText {
                query: format!("Edit failed: {}", tool_result),
                context_uuid,
            }
            .execute(ctx, state),
        )
        .await,
    )
}

pub async fn retry_tool(
    state: State<AppState>,
    ctx: RequestContext,
    param: RetryToolParams,
) -> Response {
    let RetryToolParams {
        tool_name,
        tool_input,
        _tool_result,
        next_tool_name,
        node_id,
        user_query,
        tool_retries,
    } = param;
    if !RetryState::budget_exceeded(tool_retries, MAX_TOOL_RETRIES) {
        {
            let mut wf = state.workflow.lock().await;
            if let Some(aw) = wf.active_mut() {
                aw.retry.increment_tool_retries(node_id);
            }
            wf.set_recovering();
        }
        tracing::warn!(
            "[recovery] retry_tool node={} attempt={}",
            node_id,
            tool_retries + 1
        );
        let tools_snap = state.tools.read().await.clone();
        let current_hint = tool_input
            .get("file_path")
            .map(|p| serde_json::json!({ "file_path": p }));
        if let Ok(retry_tool) = ToolEntry::resolve(
            &tool_name,
            &tools_snap,
            StatusCode::BAD_GATEWAY,
            format!("Tool not found: {}", tool_name),
        ) {
            let retry_compressed = ToolEntry::resolve_compressed(&state, &tool_name).await;
            return Box::pin(
                WorkflowAction::FireTool {
                    tool_name,
                    tool: retry_tool,
                    compressed: retry_compressed,
                    node_id,
                    user_query,
                    canonical_args_hint: current_hint,
                }
                .execute(ctx, state),
            )
            .await;
        }
    }
    let abort_msg = format!(
        "{} failed — cannot proceed to {} without its output.",
        tool_name, next_tool_name
    );
    state.workflow.lock().await.reset();
    let context_uuid = {
        let query_str = serialize_segments(&user_query);
        let mut h = DefaultHasher::new();
        query_str.hash(&mut h);
        let hash = format!("{:x}", h.finish());
        let session = state.session.lock().await;
        session
            .context_uuids
            .get(&(hash, CTX_KEY_PLANNER.to_string()))
            .copied()
    };
    Box::pin(
        WorkflowAction::RespondWithText {
            query: abort_msg,
            context_uuid,
        }
        .execute(ctx, state),
    )
    .await
}
