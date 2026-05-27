use axum::response::Response;
use serde_json::Value;

use crate::{
    anthropic::UserSegment,
    planner::{CompressedTool, ExecutionGraph, IntentPlan, NodeId},
    state::{AppState, RequestContext},
};

pub struct SelectionContext {
    pub state: AppState,
    pub ctx: RequestContext,
    pub user_query: Vec<UserSegment>,
    pub tools: Vec<Value>,
    pub cwd: String,
    pub query_hash: Option<String>,
    pub intent: Option<IntentPlan>,
    pub output: Option<SelectionOutput>,
}

pub enum SelectionOutput {
    Ready {
        graph: ExecutionGraph,
        selected: SelectedTool,
    },
    EmptyGraph {
        retries: u8,
    },
    Recover(Response),
}

pub struct SelectedTool {
    pub tool_name: String,
    pub tool: Value,
    pub compressed: Option<CompressedTool>,
    pub node_id: NodeId,
    pub args_hint: Option<Value>,
}
