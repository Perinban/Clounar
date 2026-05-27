use std::collections::{HashMap, HashSet};

use crate::{
    planner::{ExecutionGraph, NodeId},
    validation::{ExecutionContractError, Validate},
};

pub struct ContractContext;

impl<'ctx> Validate<'ctx> for ExecutionGraph {
    type Context = ContractContext;
    type Error = ExecutionContractError;

    fn validate(&self, _ctx: &'ctx Self::Context) -> Result<(), Self::Error> {
        if self.ordering.is_empty() {
            return Err(ExecutionContractError::EmptyGraph);
        }

        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut stack: HashSet<NodeId> = HashSet::new();
        for &id in &self.ordering {
            if !visited.contains(&id) {
                dfs_cycle_check(id, &self.dependencies, &mut visited, &mut stack)?;
            }
        }

        let order_pos: HashMap<NodeId, usize> = self
            .ordering
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect();

        let mut producers: HashMap<String, (NodeId, usize)> = HashMap::new();
        for &node_id in &self.ordering {
            let node = &self.nodes[&node_id];
            let pos = order_pos[&node_id];
            for spec in &node.produces_artifacts {
                producers
                    .entry(spec.ref_name.clone())
                    .or_insert((node_id, pos));
            }
        }

        for &node_id in &self.ordering {
            let node = &self.nodes[&node_id];
            let consumer_pos = order_pos[&node_id];
            for ref_name in &node.required_artifact_refs {
                match producers.get(ref_name) {
                    None => {
                        tracing::warn!(
                            "[contracts] no producer for ref={} consumer_node={}",
                            ref_name,
                            node_id
                        );
                        return Err(ExecutionContractError::MissingArtifactProducer(
                            ref_name.clone(),
                        ));
                    }
                    Some((_, producer_pos)) => {
                        if producer_pos >= &consumer_pos {
                            tracing::warn!(
                                "[contracts] producer after consumer for ref={}",
                                ref_name
                            );
                            return Err(ExecutionContractError::ArtifactProducerAfterConsumer(
                                ref_name.clone(),
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn dfs_cycle_check(
    node: NodeId,
    dependencies: &HashMap<NodeId, Vec<NodeId>>,
    visited: &mut HashSet<NodeId>,
    stack: &mut HashSet<NodeId>,
) -> Result<(), ExecutionContractError> {
    visited.insert(node);
    stack.insert(node);
    if let Some(deps) = dependencies.get(&node) {
        for &dep in deps {
            if !visited.contains(&dep) {
                dfs_cycle_check(dep, dependencies, visited, stack)?;
            } else if stack.contains(&dep) {
                tracing::warn!("[contracts] cycle detected at node={}", dep);
                return Err(ExecutionContractError::CyclicDependency);
            }
        }
    }
    stack.remove(&node);
    Ok(())
}
