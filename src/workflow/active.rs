use std::collections::HashSet;
use uuid::Uuid;

use crate::{
    planner::{ExecutionGraph, IntentPlan, NodeId},
    state::PendingTool,
    workflow::{recovery::RetryState, state::WorkflowId},
};

pub struct ActiveWorkflow {
    pub id: WorkflowId,
    pub graph: ExecutionGraph,
    pub pending_tool: Option<PendingTool>,
    pub completed_nodes: HashSet<NodeId>,
    pub retry: RetryState,
    pub pending_intent: Option<IntentPlan>,
    pub context_uuid: Option<Uuid>,
}

impl ActiveWorkflow {
    pub fn sub_uuid(&self, key: &str) -> Option<Uuid> {
        self.context_uuid.map(|u| Uuid::new_v5(&u, key.as_bytes()))
    }
}
