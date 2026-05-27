use std::fmt;

#[derive(Debug)]
pub enum FsRequirement {
    BeFile,
    NotBeDir,
    BeDir,
}

#[derive(Debug)]
pub enum ResolveError {
    TargetNotFound,
    TargetIsDirectory,
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TargetNotFound => write!(f, "target not found"),
            Self::TargetIsDirectory => write!(f, "target is a directory"),
        }
    }
}
