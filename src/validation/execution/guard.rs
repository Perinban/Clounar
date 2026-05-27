use std::collections::HashSet;

use crate::{
    planner::{CapabilityRegistry, ExecutionGraph, ExecutionNode, NodeId},
    validation::{ExecutionGuardError, Validate},
};

pub struct GuardContext<'a> {
    pub graph: &'a ExecutionGraph,
    pub completed: &'a HashSet<NodeId>,
    pub registry: &'a CapabilityRegistry,
}

impl<'ctx> Validate<'ctx> for ExecutionNode {
    type Context = GuardContext<'ctx>;
    type Error = ExecutionGuardError;

    fn validate(&self, ctx: &'ctx Self::Context) -> Result<(), Self::Error> {
        if let Some(deps) = ctx.graph.dependencies.get(&self.id) {
            for &dep_id in deps {
                if !ctx.completed.contains(&dep_id) {
                    tracing::warn!(
                        "[guard] node={} dependency={} not completed",
                        self.id,
                        dep_id
                    );
                    return Err(ExecutionGuardError::DependencyNotCompleted(dep_id));
                }
            }
        }

        let capability = match ctx.registry.by_name.get(&self.tool_name) {
            Some(c) => c,
            None => {
                tracing::warn!(
                    "[guard] node={} tool='{}' not in registry",
                    self.id,
                    self.tool_name
                );
                return Err(ExecutionGuardError::UnknownTool(self.tool_name.clone()));
            }
        };

        for spec in &self.produces_artifacts {
            if !capability.kind.matches(&spec.kind) {
                tracing::warn!(
                    "[guard] node={} tool='{}' capability={:?} incompatible with artifact ref={}",
                    self.id,
                    self.tool_name,
                    capability.kind,
                    spec.ref_name
                );
                return Err(ExecutionGuardError::CapabilityMismatch(format!(
                    "tool '{}' cannot produce artifact ref '{}'",
                    self.tool_name, spec.ref_name
                )));
            }
        }

        tracing::debug!(
            "[guard] node={} tool='{}' validated dependencies={:?}",
            self.id,
            self.tool_name,
            ctx.graph
                .dependencies
                .get(&self.id)
                .map(|d| d.as_slice())
                .unwrap_or(&[])
        );
        Ok(())
    }
}
