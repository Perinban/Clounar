use std::{collections::HashSet, mem};
use uuid::Uuid;

use crate::{
    planner::{ExecutionGraph, NodeId},
    workflow::{active::ActiveWorkflow, recovery::RetryState},
};

pub type WorkflowId = String;

#[derive(Default)]
pub enum WorkflowState {
    #[default]
    Idle,
    Running(ActiveWorkflow),
    Recovering(ActiveWorkflow),
    #[allow(dead_code)]
    AwaitingInteraction(ActiveWorkflow),
}

impl WorkflowState {
    pub fn start(id: WorkflowId, graph: ExecutionGraph, context_uuid: Option<Uuid>) -> Self {
        Self::Running(ActiveWorkflow {
            id,
            graph,
            pending_tool: None,
            completed_nodes: HashSet::new(),
            retry: RetryState::default(),
            pending_intent: None,
            context_uuid,
        })
    }

    pub fn reset(&mut self) {
        *self = Self::Idle;
    }

    pub fn active(&self) -> Option<&ActiveWorkflow> {
        match self {
            Self::Running(w) | Self::Recovering(w) => Some(w),
            _ => None,
        }
    }

    pub fn active_mut(&mut self) -> Option<&mut ActiveWorkflow> {
        match self {
            Self::Running(w) | Self::Recovering(w) => Some(w),
            _ => None,
        }
    }

    pub fn pause_at(&mut self, _node_id: NodeId) {
        match mem::replace(self, Self::Idle) {
            Self::Running(aw) | Self::Recovering(aw) => {
                *self = Self::AwaitingInteraction(aw);
            }
            other => *self = other,
        }
    }

    pub fn is_paused(&self) -> bool {
        matches!(self, Self::AwaitingInteraction(_))
    }

    pub fn resume(&mut self) -> Option<&mut ActiveWorkflow> {
        if matches!(self, Self::AwaitingInteraction(_)) {
            if let Self::AwaitingInteraction(aw) = mem::replace(self, Self::Idle) {
                *self = Self::Running(aw);
            }
        }
        self.active_mut()
    }

    pub fn set_recovering(&mut self) {
        if let Self::Running(aw) = mem::replace(self, Self::Idle) {
            *self = Self::Recovering(aw);
        }
    }
}
