use std::fmt;

#[derive(Debug)]
pub enum ContentError {
    OldStringNotFound,
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OldStringNotFound => write!(f, "old_string not found in file content"),
        }
    }
}

pub struct TransformContext<'a> {
    pub file_content: &'a str,
}
