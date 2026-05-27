use strum::EnumString;

use crate::planner::{CapabilityKind, HasCapability};

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString)]
pub enum ActionClass {
    ReadFile,
    WriteFile,
    EditFile,
    ListDirectory,
    ExecuteCommand,
    MonitorTask,
    WebSearch,
    DebugCode,
}

impl ActionClass {
    pub fn from_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

impl HasCapability for ActionClass {
    fn capability(&self) -> CapabilityKind {
        match self {
            ActionClass::ReadFile => CapabilityKind::Read,
            ActionClass::WriteFile => CapabilityKind::Write,
            ActionClass::EditFile => CapabilityKind::Edit,
            ActionClass::ExecuteCommand => CapabilityKind::Execute,
            ActionClass::MonitorTask => CapabilityKind::Execute,
            ActionClass::ListDirectory => CapabilityKind::Execute,
            ActionClass::WebSearch => CapabilityKind::Search,
            ActionClass::DebugCode => CapabilityKind::Read,
        }
    }
}

#[derive(Debug, Clone, strum::EnumString)]
pub enum WriteDisposition {
    CreateNew,
    ModifyExisting,
    OverwriteExisting,
    EnsureExists,
}
