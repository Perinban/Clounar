use std::fmt;

use crate::validation::{ArgValidationError, ExecutionGuardError};

#[derive(Debug)]
pub enum ToolError {
    GuardFailed(ExecutionGuardError),
    ArgsBuildFailed(String),
    TransformMissingOld,
    TransformInvalid(String),
    ArgValidationFailed(String),
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GuardFailed(e) => write!(f, "execution guard failed: {}", e),
            Self::ArgsBuildFailed(e) => write!(f, "failed to build args: {}", e),
            Self::TransformMissingOld => write!(
                f,
                "edit blocked: missing target string. No changes were made."
            ),
            Self::TransformInvalid(e) => write!(f, "edit blocked: {}. No changes were made.", e),
            Self::ArgValidationFailed(e) => write!(f, "argument validation failed: {}", e),
        }
    }
}

impl From<ExecutionGuardError> for ToolError {
    fn from(e: ExecutionGuardError) -> Self {
        Self::GuardFailed(e)
    }
}

impl From<ArgValidationError> for ToolError {
    fn from(e: ArgValidationError) -> Self {
        Self::ArgValidationFailed(e.to_string())
    }
}
