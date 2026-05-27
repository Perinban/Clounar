use serde_json::Value;
use uuid::Uuid;

pub enum ResponseKind {
    Text {
        query: String,
        context_uuid: Option<Uuid>,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    Abort {
        reason: String,
    },
}

pub struct TextKind {
    pub query: String,
    pub context_uuid: Option<Uuid>,
}

pub struct ToolUseKind {
    pub id: String,
    pub name: String,
    pub input: Value,
}

pub struct AbortKind {
    pub reason: String,
}
