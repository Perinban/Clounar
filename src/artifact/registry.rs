use std::{collections::HashMap, time::SystemTime};

use crate::{bridge::tools::TOOL_MAP, planner::NodeId};

use super::types::{Artifact, ArtifactId, ArtifactKind, ArtifactMetadata, GraphArtifactRef};

pub struct NodeEmit<'a> {
    pub specs: &'a [(String, &'static str)],
    pub node_id: NodeId,
    pub tool_name: &'a str,
    pub artifact_name: &'a str,
    pub content: &'a str,
    pub workflow_id: &'a str,
    pub tool_input: &'a serde_json::Value,
}

#[derive(Default)]
pub struct ArtifactRegistry {
    artifacts: HashMap<ArtifactId, Artifact>,
    ref_bindings: HashMap<GraphArtifactRef, Vec<ArtifactId>>,
}

impl ArtifactRegistry {
    pub fn emit(
        &mut self,
        ref_name: GraphArtifactRef,
        kind: ArtifactKind,
        name: String,
        content: String,
        metadata: ArtifactMetadata,
    ) {
        let versions = self.ref_bindings.entry(ref_name).or_default();
        let version = versions.len() + 1;
        let id = format!("artifact:{}:{}:v{}", kind.as_ref_name(), name, version);
        versions.push(id.clone());
        self.artifacts.insert(
            id.clone(),
            Artifact {
                id,
                kind,
                name,
                content,
                metadata,
            },
        );
    }

    pub fn emit_for_node(&mut self, p: NodeEmit<'_>) {
        let (specs, node_id, tool_name, artifact_name, content, workflow_id, tool_input) = (
            p.specs,
            p.node_id,
            p.tool_name,
            p.artifact_name,
            p.content,
            p.workflow_id,
            p.tool_input,
        );
        let file_path = TOOL_MAP
            .iter()
            .find(|e| e.kind.as_ref() == tool_name)
            .and_then(|e| e.file_key)
            .and_then(|key| tool_input.get(key))
            .and_then(|v| v.as_str());
        for (ref_name, kind_name) in specs {
            let kind = kind_name.parse().unwrap_or(ArtifactKind::ExecutionOutput);
            let kind_ref = kind.as_ref_name();
            self.emit(
                ref_name.clone(),
                kind,
                artifact_name.to_string(),
                content.to_string(),
                ArtifactMetadata {
                    produced_by_node: node_id,
                    produced_by_tool: tool_name.to_string(),
                    timestamp: SystemTime::now(),
                    workflow_id: workflow_id.to_string(),
                    file_path: file_path.map(|s| s.to_string()),
                },
            );
            let version = self.ref_bindings_len(ref_name);
            tracing::info!(
                "[artifact] node={} tool={} produced artifact:{}:{}:v{}",
                node_id,
                tool_name,
                kind_ref,
                artifact_name,
                version
            );
        }
    }

    pub fn clear(&mut self) {
        self.artifacts.clear();
        self.ref_bindings.clear();
    }

    pub fn ref_bindings_len(&self, ref_name: &str) -> usize {
        self.ref_bindings
            .get(ref_name)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn rebind(&mut self, ref_name: &str, file_path: &str, workflow_id: &str) -> bool {
        let id = self
            .ref_bindings
            .get(ref_name)
            .and_then(|ids| {
                ids.iter().rev().find(|id| {
                    self.artifacts
                        .get(*id)
                        .and_then(|a| a.metadata.file_path.as_deref())
                        == Some(file_path)
                })
            })
            .cloned();

        if let Some(id) = id {
            if let Some(a) = self.artifacts.get_mut(&id) {
                a.metadata.workflow_id = workflow_id.to_string();
            }
            self.ref_bindings
                .entry(ref_name.to_string())
                .or_default()
                .push(id);
            true
        } else {
            false
        }
    }

    pub fn resolve(&self, workflow_id: &str, ref_name: &str) -> Option<&Artifact> {
        let id = self.ref_bindings.get(ref_name)?.last()?;
        let artifact = self.artifacts.get(id)?;
        if artifact.metadata.workflow_id == workflow_id {
            Some(artifact)
        } else {
            None
        }
    }
}
