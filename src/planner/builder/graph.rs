use serde_json::Value;
use std::collections::HashMap;

use crate::{
    bridge::ToolEntry,
    planner::{
        CapabilityKind, CapabilityRegistry, ExecutionGraph, ExecutionNode, NodeId, NodeKind,
        TaskDecomposition,
    },
    validation::ExecutionContractError,
    workflow::workspace::Workspace,
};

impl TaskDecomposition {
    pub fn build(
        &self,
        registry: &CapabilityRegistry,
        tools: &[Value],
        cwd: &str,
    ) -> Result<ExecutionGraph, ExecutionContractError> {
        let workspace = Workspace::probe(cwd);
        let mut nodes: HashMap<NodeId, ExecutionNode> = HashMap::new();
        let mut dependencies: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut ordering: Vec<NodeId> = Vec::new();
        let mut task_index_to_node_ids: HashMap<usize, Vec<NodeId>> = HashMap::new();
        let mut next_node_id: NodeId = 1;

        'tasks: for (task_idx, task) in self.tasks.iter().enumerate() {
            let steps = task.resolve_steps(registry, tools, cwd, &workspace);
            if steps.is_empty() {
                return Err(ExecutionContractError::NoToolForAction(task.action.clone()));
            }

            let dep_node_ids: Vec<NodeId> = task
                .depends_on
                .iter()
                .flat_map(|&dep_idx| {
                    task_index_to_node_ids
                        .get(&dep_idx)
                        .cloned()
                        .unwrap_or_default()
                })
                .collect();

            let mut prev_node_in_task: Option<NodeId> = None;

            for step in steps {
                let node_id = next_node_id;
                next_node_id += 1;

                let mut node_deps: Vec<NodeId> = Vec::new();
                if let Some(prev) = prev_node_in_task {
                    node_deps.push(prev);
                } else if !dep_node_ids.is_empty() {
                    node_deps.extend_from_slice(&dep_node_ids);
                }

                if !node_deps.is_empty() {
                    dependencies.insert(node_id, node_deps);
                }

                let kind = ToolEntry::from_name(&step.tool_name)
                    .and_then(|e| e.capability.as_ref())
                    .map(|c| {
                        if matches!(c, CapabilityKind::Interact) {
                            NodeKind::Interaction
                        } else {
                            NodeKind::Execution
                        }
                    })
                    .unwrap_or(NodeKind::Execution);

                nodes.insert(
                    node_id,
                    ExecutionNode {
                        id: node_id,
                        kind: kind.clone(),
                        tool_name: step.tool_name,
                        required_artifact_refs: step.requires_artifact,
                        produces_artifacts: step
                            .produces_artifact
                            .map(|s| vec![s])
                            .unwrap_or_default(),
                        args_hint: step.args_hint,
                    },
                );

                ordering.push(node_id);
                prev_node_in_task = Some(node_id);

                if matches!(kind, NodeKind::Interaction) {
                    break 'tasks;
                }
            }

            if let Some(last) = prev_node_in_task {
                task_index_to_node_ids
                    .entry(task_idx)
                    .or_default()
                    .push(last);
            }
        }

        Ok(ExecutionGraph {
            nodes,
            ordering,
            dependencies,
        })
    }
}
