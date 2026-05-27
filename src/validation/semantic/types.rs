use std::fmt;

#[derive(Debug)]
pub enum SemanticValidationError {
    NoCompatibleCapability,
    ParentDirectoryMissing,
}

impl fmt::Display for SemanticValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCompatibleCapability => {
                write!(f, "no compatible capability registered for action")
            }
            Self::ParentDirectoryMissing => write!(f, "parent directory does not exist"),
        }
    }
}
