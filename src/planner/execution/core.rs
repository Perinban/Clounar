use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::artifact::{ArtifactSpec, GraphArtifactRef};

pub type NodeId = u32;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CompressedTool {
    pub capabilities: Vec<String>,
    pub limits: Vec<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Clone, PartialEq)]
pub enum NodeKind {
    Execution,
    Interaction,
}

#[derive(Clone)]
pub struct ExecutionNode {
    #[allow(dead_code)]
    pub id: NodeId,
    pub kind: NodeKind,
    pub tool_name: String,
    pub required_artifact_refs: Vec<GraphArtifactRef>,
    pub produces_artifacts: Vec<ArtifactSpec>,
    pub args_hint: Option<Value>,
}

#[derive(Clone, Default)]
pub struct ExecutionGraph {
    pub nodes: HashMap<NodeId, ExecutionNode>,
    pub dependencies: HashMap<NodeId, Vec<NodeId>>,
    pub ordering: Vec<NodeId>,
}

impl ExecutionGraph {
    pub fn validate(&self) -> Result<(), String> {
        for id in &self.ordering {
            if !self.nodes.contains_key(id) {
                return Err(format!("ordering references unknown node id={}", id));
            }
        }
        for (node_id, deps) in &self.dependencies {
            for dep_id in deps {
                if !self.nodes.contains_key(dep_id) {
                    return Err(format!(
                        "node {} depends on unknown node {}",
                        node_id, dep_id
                    ));
                }
            }
        }
        for id in &self.ordering {
            let node = &self.nodes[id];
            if node.tool_name.is_empty() {
                return Err(format!("node {} has no tool_name", id));
            }
        }
        Ok(())
    }
}
