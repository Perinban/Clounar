use std::fmt;

#[derive(Debug)]
pub enum ArgValidationError {
    MissingRequired(String),
    WrongType {
        field: String,
        expected: &'static str,
        got: &'static str,
    },
    EmptyValue(String),
    SemanticViolation(String),
}

impl fmt::Display for ArgValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequired(field) => write!(f, "required argument '{}' is missing", field),
            Self::WrongType {
                field,
                expected,
                got,
            } => write!(f, "argument '{}' expected {}, got {}", field, expected, got),
            Self::EmptyValue(field) => write!(f, "argument '{}' must not be empty", field),
            Self::SemanticViolation(msg) => write!(f, "semantic violation: {}", msg),
        }
    }
}
