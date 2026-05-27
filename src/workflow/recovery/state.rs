use std::collections::HashMap;

use crate::planner::NodeId;

#[derive(Debug, Clone, Default)]
pub struct RetryState {
    pub classifier_retries: u8,
    pub tool_retries: HashMap<NodeId, u8>,
    pub edit_retries: HashMap<NodeId, u8>,
}

impl RetryState {
    pub fn get_edit_retries(&self, node_id: NodeId) -> u8 {
        self.edit_retries.get(&node_id).copied().unwrap_or(0)
    }

    pub fn get_tool_retries(&self, node_id: NodeId) -> u8 {
        self.tool_retries.get(&node_id).copied().unwrap_or(0)
    }

    pub fn increment_edit_retries(&mut self, node_id: NodeId) {
        *self.edit_retries.entry(node_id).or_insert(0) += 1;
    }

    pub fn increment_tool_retries(&mut self, node_id: NodeId) {
        *self.tool_retries.entry(node_id).or_insert(0) += 1;
    }

    pub fn budget_exceeded(retries: u8, max: u8) -> bool {
        retries >= max
    }
}
