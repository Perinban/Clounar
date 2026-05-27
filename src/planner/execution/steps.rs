use serde_json::Value;

use crate::{
    artifact::{ArtifactKind, ArtifactSpec},
    bridge::TOOL_MAP,
    planner::{ActionClass, CapabilityKind, HasCapability, Resolvable, WriteDisposition},
    validation::{FileSystemState, FsRequirement, ResolveError, Validate},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticOp {
    CreateFile,
    ModifyFile,
    NotebookModifyFile,
    OverwriteFile,
    ReadFile,
    ListDirectory,
    DebugCode,
}

pub struct ExecutionStep {
    pub tool_name: String,
    pub args_hint: Option<Value>,
    pub requires_artifact: Vec<String>,
    pub produces_artifact: Option<ArtifactSpec>,
}

fn step(tool_name: &'static str, args_hint: Option<Value>) -> ExecutionStep {
    let entry = TOOL_MAP.iter().find(|e| e.kind.as_ref() == tool_name);
    ExecutionStep {
        tool_name: tool_name.into(),
        args_hint,
        requires_artifact: entry
            .and_then(|e| e.requires)
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        produces_artifact: entry.and_then(|e| e.produces).map(|(ref_name, kind_str)| {
            ArtifactSpec {
                ref_name: ref_name.into(),
                kind: kind_str.parse().unwrap_or(ArtifactKind::ExecutionOutput),
            }
        }),
    }
}

impl ExecutionStep {
    fn with_requires(mut self, requires: &[&'static str]) -> Self {
        self.requires_artifact = requires.iter().map(|s| s.to_string()).collect();
        self
    }
}

impl SemanticOp {
    pub fn fs_requirement(&self) -> FsRequirement {
        match self {
            SemanticOp::ReadFile
            | SemanticOp::ModifyFile
            | SemanticOp::NotebookModifyFile
            | SemanticOp::DebugCode => FsRequirement::BeFile,
            SemanticOp::CreateFile | SemanticOp::OverwriteFile => FsRequirement::NotBeDir,
            SemanticOp::ListDirectory => FsRequirement::BeDir,
        }
    }

    pub fn from_action(
        action: &ActionClass,
        write_disposition: Option<&WriteDisposition>,
        fs: &FileSystemState,
    ) -> Option<Self> {
        match action {
            ActionClass::ReadFile => Some(SemanticOp::ReadFile),
            ActionClass::ListDirectory => Some(SemanticOp::ListDirectory),
            ActionClass::EditFile => {
                let is_notebook = fs
                    .abs_path
                    .extension()
                    .map(|e| e.eq_ignore_ascii_case("ipynb"))
                    .unwrap_or(false);
                if is_notebook {
                    Some(SemanticOp::NotebookModifyFile)
                } else {
                    Some(SemanticOp::ModifyFile)
                }
            }
            ActionClass::DebugCode => Some(SemanticOp::DebugCode),
            ActionClass::WriteFile => Some(match write_disposition {
                Some(WriteDisposition::CreateNew) => SemanticOp::CreateFile,
                Some(WriteDisposition::ModifyExisting) => SemanticOp::ModifyFile,
                Some(WriteDisposition::OverwriteExisting) => SemanticOp::OverwriteFile,
                Some(WriteDisposition::EnsureExists) | None => {
                    if fs.exists {
                        SemanticOp::ModifyFile
                    } else {
                        SemanticOp::CreateFile
                    }
                }
            }),
            _ if fs.is_dir => Some(SemanticOp::ListDirectory),
            _ => None,
        }
    }
}

impl HasCapability for SemanticOp {
    fn capability(&self) -> CapabilityKind {
        match self {
            SemanticOp::ReadFile => CapabilityKind::Read,
            SemanticOp::CreateFile
            | SemanticOp::ModifyFile
            | SemanticOp::NotebookModifyFile
            | SemanticOp::OverwriteFile => CapabilityKind::Edit,
            SemanticOp::ListDirectory => CapabilityKind::Execute,
            SemanticOp::DebugCode => CapabilityKind::Search,
        }
    }
}

impl Resolvable for SemanticOp {
    fn resolve(
        &self,
        fs_state: Option<&FileSystemState>,
    ) -> Result<Vec<ExecutionStep>, ResolveError> {
        let fs = fs_state.unwrap();
        fs.validate(&self.fs_requirement())?;
        match self {
            SemanticOp::CreateFile => {
                tracing::debug!(
                    "[resolver] op=CreateFile canonical_file_path={}",
                    fs.abs_path.display()
                );
                let path_hint =
                    Some(serde_json::json!({ "file_path": fs.abs_path.to_string_lossy() }));
                if fs.exists {
                    Ok(vec![
                        step("Read", path_hint.clone()),
                        step("Write", path_hint),
                    ])
                } else {
                    Ok(vec![step("Write", path_hint)])
                }
            }

            SemanticOp::ModifyFile => {
                tracing::debug!(
                    "[resolver] op=ModifyFile canonical_file_path={}",
                    fs.abs_path.display()
                );
                let path_hint =
                    Some(serde_json::json!({ "file_path": fs.abs_path.to_string_lossy() }));
                Ok(vec![
                    step("Read", path_hint.clone()),
                    step("Edit", path_hint),
                ])
            }

            SemanticOp::ReadFile => {
                tracing::debug!(
                    "[resolver] op=ReadFile canonical_file_path={}",
                    fs.abs_path.display()
                );
                Ok(vec![step(
                    "Read",
                    Some(serde_json::json!({ "file_path": fs.abs_path.to_string_lossy() })),
                )])
            }

            SemanticOp::NotebookModifyFile => {
                tracing::debug!(
                    "[resolver] op=NotebookModifyFile canonical_file_path={}",
                    fs.abs_path.display()
                );
                let read_hint =
                    Some(serde_json::json!({ "file_path": fs.abs_path.to_string_lossy() }));
                let edit_hint =
                    Some(serde_json::json!({ "notebook_path": fs.abs_path.to_string_lossy() }));
                Ok(vec![
                    step("Read", read_hint),
                    step("NotebookEdit", edit_hint),
                ])
            }

            SemanticOp::OverwriteFile => {
                tracing::debug!(
                    "[resolver] op=OverwriteFile canonical_file_path={}",
                    fs.abs_path.display()
                );
                Ok(vec![step(
                    "Write",
                    Some(serde_json::json!({ "file_path": fs.abs_path.to_string_lossy() })),
                )])
            }

            SemanticOp::ListDirectory => {
                tracing::debug!("[resolver] op=ListDirectory path={}", fs.abs_path.display());
                Ok(vec![step(
                    "Bash",
                    Some(
                        serde_json::json!({ "command": format!("ls -la {}", fs.abs_path.display()) }),
                    ),
                )])
            }

            SemanticOp::DebugCode => {
                tracing::debug!(
                    "[resolver] op=DebugCode canonical_file_path={}",
                    fs.abs_path.display()
                );
                let path_hint =
                    Some(serde_json::json!({ "file_path": fs.abs_path.to_string_lossy() }));
                Ok(vec![
                    step("WebSearch", None),
                    step("Read", path_hint.clone()),
                    step("Edit", path_hint).with_requires(&["file_content", "search_results"]),
                ])
            }
        }
    }
}
