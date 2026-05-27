use serde_json::Value;

use crate::{
    anthropic::UserSegment,
    planner::{CompressedTool, NodeId},
};

pub enum WorkflowAction {
    FireTool {
        tool_name: String,
        tool: Value,
        compressed: Option<CompressedTool>,
        node_id: NodeId,
        user_query: Vec<UserSegment>,
        canonical_args_hint: Option<Value>,
    },
    RespondWithText {
        query: String,
        context_uuid: Option<uuid::Uuid>,
    },
}
