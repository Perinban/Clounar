use axum::{http::StatusCode, response::Response};
use serde_json::Value;

use crate::{
    planner::{traits::HasCapability, CapabilityKind, CompressedTool},
    state::AppState,
    validation::{validate_args, ArgValidationError},
    workflow::responder::error_response,
};

#[derive(Clone, Copy)]
pub enum ArgType {
    String,
    NonEmptyString,
    Number,
    Bool,
    NonEmptyArray,
}

#[derive(Clone, Copy)]
pub enum ArgField {
    Required(&'static str, ArgType),
    Optional(&'static str, ArgType),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolStatus {
    Active,
    Excluded,
    Override,
}

#[derive(Debug, Clone, PartialEq, strum::AsRefStr, strum::EnumString)]
#[strum(ascii_case_insensitive)]
pub enum ToolKind {
    Read,
    Write,
    Edit,
    Bash,
    WebSearch,
    Monitor,
    WebFetch,
    Agent,
    AskUserQuestion,
    CronCreate,
    CronDelete,
    CronList,
    EnterPlanMode,
    ExitPlanMode,
    EnterWorktree,
    ExitWorktree,
    NotebookEdit,
    PushNotification,
    RemoteTrigger,
    ScheduleWakeup,
    Skill,
    TaskCreate,
    TaskGet,
    TaskList,
    TaskUpdate,
    TaskOutput,
    TaskStop,
}

pub struct ToolEntry {
    pub kind: ToolKind,
    pub status: ToolStatus,
    pub capability: Option<CapabilityKind>,
    pub produces: Option<(&'static str, &'static str)>,
    pub requires: Option<&'static str>,
    pub file_key: Option<&'static str>,
    pub args: &'static [ArgField],
}

pub static TOOL_MAP: &[ToolEntry] = &[
    ToolEntry {
        kind: ToolKind::Read,
        status: ToolStatus::Active,
        capability: Some(CapabilityKind::Read),
        produces: Some(("file_content", "file_content")),
        requires: None,
        file_key: Some("file_path"),
        args: &[
            ArgField::Required("file_path", ArgType::NonEmptyString),
            ArgField::Optional("offset", ArgType::Number),
            ArgField::Optional("limit", ArgType::Number),
            ArgField::Optional("pages", ArgType::String),
        ],
    },
    ToolEntry {
        kind: ToolKind::Write,
        status: ToolStatus::Active,
        capability: Some(CapabilityKind::Write),
        produces: None,
        requires: None,
        file_key: Some("file_path"),
        args: &[
            ArgField::Required("file_path", ArgType::NonEmptyString),
            ArgField::Required("content", ArgType::String),
        ],
    },
    ToolEntry {
        kind: ToolKind::Edit,
        status: ToolStatus::Active,
        capability: Some(CapabilityKind::Edit),
        produces: Some(("file_content", "file_content")),
        requires: Some("file_content"),
        file_key: Some("file_path"),
        args: &[
            ArgField::Required("file_path", ArgType::NonEmptyString),
            ArgField::Required("old_string", ArgType::String),
            ArgField::Required("new_string", ArgType::String),
            ArgField::Optional("replace_all", ArgType::Bool),
        ],
    },
    ToolEntry {
        kind: ToolKind::Bash,
        status: ToolStatus::Active,
        capability: Some(CapabilityKind::Execute),
        produces: Some(("command_output", "command_output")),
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("command", ArgType::NonEmptyString),
            ArgField::Optional("timeout", ArgType::Number),
            ArgField::Optional("description", ArgType::String),
            ArgField::Optional("run_in_background", ArgType::Bool),
            ArgField::Optional("dangerouslyDisableSandbox", ArgType::Bool),
        ],
    },
    ToolEntry {
        kind: ToolKind::WebSearch,
        status: ToolStatus::Override,
        capability: Some(CapabilityKind::Search),
        produces: Some(("search_results", "search_results")),
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("query", ArgType::NonEmptyString),
            ArgField::Optional("allowed_domains", ArgType::NonEmptyArray),
            ArgField::Optional("blocked_domains", ArgType::NonEmptyArray),
        ],
    },
    ToolEntry {
        kind: ToolKind::Monitor,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("description", ArgType::NonEmptyString),
            ArgField::Required("command", ArgType::NonEmptyString),
            ArgField::Required("timeout_ms", ArgType::Number),
            ArgField::Required("persistent", ArgType::Bool),
        ],
    },
    ToolEntry {
        kind: ToolKind::WebFetch,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("url", ArgType::NonEmptyString),
            ArgField::Required("prompt", ArgType::String),
        ],
    },
    ToolEntry {
        kind: ToolKind::Agent,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("description", ArgType::NonEmptyString),
            ArgField::Required("prompt", ArgType::NonEmptyString),
            ArgField::Optional("subagent_type", ArgType::String),
            ArgField::Optional("run_in_background", ArgType::Bool),
        ],
    },
    ToolEntry {
        kind: ToolKind::AskUserQuestion,
        status: ToolStatus::Active,
        capability: Some(CapabilityKind::Interact),
        produces: None,
        requires: None,
        file_key: None,
        args: &[ArgField::Required("questions", ArgType::NonEmptyArray)],
    },
    ToolEntry {
        kind: ToolKind::CronCreate,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("cron", ArgType::NonEmptyString),
            ArgField::Required("prompt", ArgType::NonEmptyString),
            ArgField::Optional("recurring", ArgType::Bool),
            ArgField::Optional("durable", ArgType::Bool),
        ],
    },
    ToolEntry {
        kind: ToolKind::CronDelete,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[ArgField::Required("id", ArgType::NonEmptyString)],
    },
    ToolEntry {
        kind: ToolKind::CronList,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[],
    },
    ToolEntry {
        kind: ToolKind::EnterPlanMode,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[],
    },
    ToolEntry {
        kind: ToolKind::ExitPlanMode,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[],
    },
    ToolEntry {
        kind: ToolKind::EnterWorktree,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[],
    },
    ToolEntry {
        kind: ToolKind::ExitWorktree,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("action", ArgType::NonEmptyString),
            ArgField::Optional("discard_changes", ArgType::Bool),
        ],
    },
    ToolEntry {
        kind: ToolKind::NotebookEdit,
        status: ToolStatus::Active,
        capability: Some(CapabilityKind::Edit),
        produces: Some(("file_content", "file_content")),
        requires: Some("file_content"),
        file_key: Some("notebook_path"),
        args: &[
            ArgField::Required("notebook_path", ArgType::NonEmptyString),
            ArgField::Required("new_source", ArgType::String),
            ArgField::Required("cell_id", ArgType::NonEmptyString),
            ArgField::Optional("cell_type", ArgType::String),
            ArgField::Optional("edit_mode", ArgType::String),
        ],
    },
    ToolEntry {
        kind: ToolKind::PushNotification,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("message", ArgType::NonEmptyString),
            ArgField::Required("status", ArgType::NonEmptyString),
        ],
    },
    ToolEntry {
        kind: ToolKind::RemoteTrigger,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("action", ArgType::NonEmptyString),
            ArgField::Optional("trigger_id", ArgType::String),
        ],
    },
    ToolEntry {
        kind: ToolKind::ScheduleWakeup,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("delaySeconds", ArgType::Number),
            ArgField::Required("reason", ArgType::NonEmptyString),
            ArgField::Required("prompt", ArgType::NonEmptyString),
        ],
    },
    ToolEntry {
        kind: ToolKind::Skill,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("skill", ArgType::NonEmptyString),
            ArgField::Optional("args", ArgType::String),
        ],
    },
    ToolEntry {
        kind: ToolKind::TaskCreate,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("subject", ArgType::NonEmptyString),
            ArgField::Required("description", ArgType::String),
            ArgField::Optional("activeForm", ArgType::String),
        ],
    },
    ToolEntry {
        kind: ToolKind::TaskGet,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Optional("taskId", ArgType::String),
            ArgField::Optional("task_id", ArgType::String),
            ArgField::Optional("shell_id", ArgType::String),
        ],
    },
    ToolEntry {
        kind: ToolKind::TaskList,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[],
    },
    ToolEntry {
        kind: ToolKind::TaskUpdate,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("taskId", ArgType::NonEmptyString),
            ArgField::Optional("subject", ArgType::String),
            ArgField::Optional("description", ArgType::String),
            ArgField::Optional("activeForm", ArgType::String),
            ArgField::Optional("owner", ArgType::String),
        ],
    },
    ToolEntry {
        kind: ToolKind::TaskOutput,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Required("task_id", ArgType::NonEmptyString),
            ArgField::Required("block", ArgType::Bool),
            ArgField::Required("timeout", ArgType::Number),
        ],
    },
    ToolEntry {
        kind: ToolKind::TaskStop,
        status: ToolStatus::Excluded,
        capability: None,
        produces: None,
        requires: None,
        file_key: None,
        args: &[
            ArgField::Optional("taskId", ArgType::String),
            ArgField::Optional("task_id", ArgType::String),
            ArgField::Optional("shell_id", ArgType::String),
        ],
    },
];

impl HasCapability for ToolEntry {
    fn capability(&self) -> CapabilityKind {
        self.capability.clone().unwrap_or(CapabilityKind::Execute)
    }
}

impl ToolEntry {
    pub fn from_name(name: &str) -> Option<&'static ToolEntry> {
        let n = Self::normalize(name);
        TOOL_MAP
            .iter()
            .find(|e| e.kind.as_ref().eq_ignore_ascii_case(&n))
    }

    pub fn find_in_tools<'a>(name: &str, tools: &'a [Value]) -> Option<&'a Value> {
        let n = Self::normalize(name);
        tools.iter().find(|t| {
            t.get("name")
                .and_then(|v| v.as_str())
                .map(|tn| tn.eq_ignore_ascii_case(&n))
                .unwrap_or(false)
        })
    }

    pub fn validate_args(&self, args: &Value) -> Result<(), ArgValidationError> {
        validate_args(&self.kind, self.args, args)
    }

    pub fn normalize_args(&self, args: &mut Value) {
        for field in self.args {
            if let ArgField::Required(name, ArgType::String)
            | ArgField::Optional(name, ArgType::String) = field
            {
                if let Some(Value::String(s)) = args.get_mut(*name) {
                    *s = s
                        .replace("\\n", "\n")
                        .replace("\\t", "\t")
                        .replace("\\r", "\r");
                }
            }
        }
    }

    pub fn resolve(
        name: &str,
        tools: &[Value],
        status: StatusCode,
        msg: String,
    ) -> Result<Value, Box<Response>> {
        match Self::find_in_tools(name, tools) {
            Some(t) => Ok(t.clone()),
            None => Err(Box::new(error_response(status, msg))),
        }
    }

    pub async fn resolve_compressed(state: &AppState, tool_name: &str) -> Option<CompressedTool> {
        state
            .tool_cache
            .read()
            .await
            .as_ref()
            .and_then(|c| c.map.get(tool_name))
            .map(|e| e.compressed.clone())
    }

    fn normalize(raw: &str) -> String {
        let mut s = raw.trim();
        if let Some(line) = s.lines().find(|l| !l.trim().is_empty()) {
            s = line.trim();
        }
        s = s
            .trim_matches('*')
            .trim_matches('`')
            .trim_matches('"')
            .trim_matches('\'');
        s.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .find(|part| !part.is_empty())
            .unwrap_or("")
            .to_string()
    }
}
