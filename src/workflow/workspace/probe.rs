use std::path::Path;

use crate::workflow::workspace::types::{ProjectKind, WorkspaceInfo};

pub trait WorkspaceProbe {
    fn detect(root: &Path) -> Option<WorkspaceInfo>;
}

pub struct Workspace;

impl WorkspaceProbe for Workspace {
    fn detect(root: &Path) -> Option<WorkspaceInfo> {
        ProjectKind::all()
            .iter()
            .find(|kind| kind.markers().iter().any(|m| root.join(m).exists()))
            .map(|kind| WorkspaceInfo {
                test_command: kind.test_command(),
            })
    }
}

impl Workspace {
    pub fn probe(cwd: &str) -> WorkspaceInfo {
        Self::detect(Path::new(cwd)).unwrap_or(WorkspaceInfo { test_command: None })
    }
}
