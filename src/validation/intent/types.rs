use std::fmt;

use crate::planner::IntentPlan;

pub struct ValidatedIntent {
    pub original: IntentPlan,
    pub canonical_subject: Option<String>,
}

#[derive(Debug)]
pub enum IntentValidationError {
    MissingSubject,
    EmptySubject,
    InvalidPath,
}

impl fmt::Display for IntentValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSubject => write!(f, "subject required but not provided"),
            Self::EmptySubject => write!(f, "subject is empty or null"),
            Self::InvalidPath => write!(f, "path incompatible with action"),
        }
    }
}
