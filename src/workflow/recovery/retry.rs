use crate::{
    bridge::ToolKind,
    validation::{ArgValidationError, RuntimeError},
};

#[derive(Debug)]
pub enum RecoveryClass {
    RetryTool,
    RetryEdit,
    Terminal,
}

pub fn classify_failure(
    kind: &ToolKind,
    arg_error: Option<&ArgValidationError>,
    runtime_error: Option<&RuntimeError>,
) -> RecoveryClass {
    if *kind == ToolKind::Edit {
        if let Some(
            ArgValidationError::MissingRequired(_)
            | ArgValidationError::EmptyValue(_)
            | ArgValidationError::SemanticViolation(_),
        ) = arg_error
        {
            return RecoveryClass::RetryEdit;
        }
        return RecoveryClass::Terminal;
    }

    if let Some(RuntimeError::Timeout | RuntimeError::Malformed | RuntimeError::Interrupted) =
        runtime_error
    {
        return RecoveryClass::RetryTool;
    }

    if let Some(RuntimeError::InvalidPath) = runtime_error {
        return RecoveryClass::Terminal;
    }

    RecoveryClass::RetryTool
}
