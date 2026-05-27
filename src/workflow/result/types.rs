use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::{
    anthropic::UserSegment,
    planner::NodeId,
    state::{AppState, RequestContext},
};

pub enum ToolResultKind {
    Success,
    CacheHit,
    Cancelled,
    Error,
}

pub enum GraphPosition {
    MidGraph { next_node_id: NodeId },
    Terminal,
    OutsideGraph,
}

#[derive(Clone)]
pub enum ExecutionSignal {
    None,
    #[allow(dead_code)]
    PauseForInteraction {
        node_id: NodeId,
        description: String,
    },
}

pub struct PipelineContext {
    pub state: AppState,
    pub ctx: RequestContext,
    pub tool_name: String,
    pub tool_input: Value,
    pub tool_result: String,
    pub node_id: NodeId,
    pub user_query: Vec<UserSegment>,
    pub kind: ToolResultKind,
    pub position: GraphPosition,
    pub signal: Arc<tokio::sync::Mutex<ExecutionSignal>>,
    pub context_uuid: Option<Uuid>,
}
