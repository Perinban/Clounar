use serde_json::Value;
use std::collections::HashMap;

use crate::{
    artifact::{ArtifactKind, ArtifactSpec},
    bridge::ToolEntry,
    planner::{ExecutionStep, HasCapability},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CapabilityKind {
    Read,
    Write,
    Edit,
    Execute,
    Search,
    Interact,
}

impl CapabilityKind {
    pub fn matches(&self, artifact: &ArtifactKind) -> bool {
        match self {
            CapabilityKind::Read | CapabilityKind::Write | CapabilityKind::Edit => {
                matches!(artifact, ArtifactKind::FileSnapshot)
            }
            CapabilityKind::Execute => {
                matches!(artifact, ArtifactKind::ExecutionOutput)
            }
            CapabilityKind::Search => {
                matches!(
                    artifact,
                    ArtifactKind::SearchResultSet | ArtifactKind::FetchedContent
                )
            }
            CapabilityKind::Interact => {
                matches!(artifact, ArtifactKind::UserResponse)
            }
        }
    }
}

#[derive(Clone)]
pub struct ToolCapability {
    pub kind: CapabilityKind,
    pub produces: &'static [&'static str],
    pub requires: &'static [&'static str],
}

pub struct CapabilityRegistry {
    pub by_name: HashMap<String, ToolCapability>,
    pub by_kind: HashMap<CapabilityKind, Vec<String>>,
}

impl CapabilityRegistry {
    pub fn build(entries: impl Iterator<Item = (String, ToolCapability)>) -> Self {
        let mut by_name = HashMap::new();
        let mut by_kind: HashMap<CapabilityKind, Vec<String>> = HashMap::new();
        for (name, cap) in entries {
            by_kind
                .entry(cap.kind.clone())
                .or_default()
                .push(name.clone());
            by_name.insert(name, cap);
        }
        Self { by_name, by_kind }
    }

    pub fn fallback(&self, item: &dyn HasCapability, tools: &[Value]) -> Option<ExecutionStep> {
        let kind = item.capability();
        self.by_kind
            .get(&kind)?
            .iter()
            .find(|name| ToolEntry::find_in_tools(name, tools).is_some())
            .map(|name| {
                let cap = self.by_name.get(name);
                ExecutionStep {
                    tool_name: name.clone(),
                    args_hint: None,
                    requires_artifact: cap
                        .map(|c| c.requires.iter().map(|s| s.to_string()).collect())
                        .unwrap_or_default(),
                    produces_artifact: cap.and_then(|c| c.produces.first()).map(|s| ArtifactSpec {
                        ref_name: s.to_string(),
                        kind: (*s).parse().unwrap_or(ArtifactKind::ExecutionOutput),
                    }),
                }
            })
    }
}
