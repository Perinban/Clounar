use crate::planner::{utils::sanitize, ActionClass, WriteDisposition};

#[derive(Debug, Clone, PartialEq)]
pub enum TaskKind {
    Knowledge,
    Procedural,
    Transform,
}

#[derive(Debug, Clone)]
pub enum PathSource {
    Explicit(String),
    LastDir(String),
    Cwd,
}

impl PathSource {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Explicit(s) | Self::LastDir(s) => s.as_str(),
            Self::Cwd => ".",
        }
    }
}

#[derive(Debug, Clone)]
pub struct IntentPlan {
    pub action: ActionClass,
    pub task_kind: TaskKind,
    pub path: Option<PathSource>,
    pub write_disposition: Option<WriteDisposition>,
}

impl IntentPlan {
    pub fn parse(raw: &str) -> Option<Self> {
        let stripped = raw.trim();
        let stripped = stripped.strip_prefix("```json").unwrap_or(stripped);
        let stripped = stripped.strip_prefix("```").unwrap_or(stripped);
        let stripped = stripped.strip_suffix("```").unwrap_or(stripped);
        let stripped = stripped.trim();

        let v: serde_json::Value = serde_json::from_str(stripped).ok()?;
        let action_str = v.get("action")?.as_str()?;
        let action = ActionClass::from_str(action_str)?;
        let task_kind = match v.get("task_kind").and_then(|s| s.as_str()) {
            Some("Knowledge") => TaskKind::Knowledge,
            Some("Transform") => TaskKind::Transform,
            _ => TaskKind::Procedural,
        };
        let path = v
            .get("relative_path")
            .and_then(|s| s.as_str())
            .map(|s| PathSource::Explicit(sanitize(s)));
        let write_disposition = match &action {
            ActionClass::EditFile => Some(WriteDisposition::ModifyExisting),
            ActionClass::WriteFile => Some(
                v.get("write_disposition")
                    .and_then(|s| s.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(WriteDisposition::EnsureExists),
            ),
            _ => None,
        };
        Some(IntentPlan {
            action,
            task_kind,
            path,
            write_disposition,
        })
    }
}
