use crate::{
    planner::{ActionClass, IntentPlan},
    validation::{check, IntentValidationError, ValidatedIntent},
};

pub fn validate_intent(
    intent: &IntentPlan,
    cwd: &str,
) -> Result<ValidatedIntent, IntentValidationError> {
    match intent.action {
        ActionClass::ExecuteCommand | ActionClass::WebSearch | ActionClass::DebugCode => {
            return Ok(ValidatedIntent {
                original: intent.clone(),
                canonical_subject: intent.path.as_ref().map(|p| p.as_str().to_string()),
            });
        }
        _ => {}
    }

    let trimmed = match &intent.path {
        None => {
            tracing::warn!(
                "[validation] action={:?} subject=None result=Invalid(MissingSubject)",
                intent.action
            );
            return Err(IntentValidationError::MissingSubject);
        }
        Some(p) => p.as_str(),
    };

    let trimmed = trimmed.trim();
    if trimmed.is_empty() || trimmed == "null" {
        tracing::warn!(
            "[validation] action={:?} subject={:?} result=Invalid(EmptySubject)",
            intent.action,
            trimmed
        );
        return Err(IntentValidationError::EmptySubject);
    }

    let fs = check(trimmed, cwd);
    let canonical = fs.abs_path.to_string_lossy().into_owned();

    match intent.action {
        ActionClass::ReadFile | ActionClass::EditFile if fs.exists && fs.is_dir => {
            tracing::warn!(
                "[validation] action={:?} subject={:?} result=Invalid(InvalidPath)",
                intent.action,
                trimmed
            );
            return Err(IntentValidationError::InvalidPath);
        }
        ActionClass::ListDirectory if fs.exists && !fs.is_dir => {
            tracing::warn!(
                "[validation] action={:?} subject={:?} result=Invalid(InvalidPath)",
                intent.action,
                trimmed
            );
            return Err(IntentValidationError::InvalidPath);
        }
        _ => {}
    }

    tracing::debug!(
        "[validation] action={:?} subject={:?} canonical={:?} result=Valid",
        intent.action,
        trimmed,
        canonical
    );
    Ok(ValidatedIntent {
        original: intent.clone(),
        canonical_subject: Some(canonical),
    })
}
