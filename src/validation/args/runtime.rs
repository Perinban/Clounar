#[derive(Debug)]
pub enum RuntimeError {
    Timeout,
    Malformed,
    Interrupted,
    InvalidPath,
}

impl RuntimeError {
    pub fn from_str(error: &str) -> Option<Self> {
        if error.contains("timeout") {
            return Some(Self::Timeout);
        }
        if error.contains("malformed") {
            return Some(Self::Malformed);
        }
        if error.contains("interrupted") {
            return Some(Self::Interrupted);
        }
        if error.contains("invalid path") || error.contains("canonical") {
            return Some(Self::InvalidPath);
        }
        None
    }
}
