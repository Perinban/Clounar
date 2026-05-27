use crate::validation::{FileSystemState, Validate};

use super::types::{FsRequirement, ResolveError};

impl<'ctx> Validate<'ctx> for FileSystemState {
    type Context = FsRequirement;
    type Error = ResolveError;

    fn validate(&self, ctx: &'ctx FsRequirement) -> Result<(), ResolveError> {
        match ctx {
            FsRequirement::BeFile => {
                if !self.exists {
                    return Err(ResolveError::TargetNotFound);
                }
                if self.is_dir {
                    return Err(ResolveError::TargetIsDirectory);
                }
                Ok(())
            }
            FsRequirement::NotBeDir => {
                if self.exists && self.is_dir {
                    return Err(ResolveError::TargetIsDirectory);
                }
                Ok(())
            }
            FsRequirement::BeDir => {
                if self.exists && !self.is_dir {
                    return Err(ResolveError::TargetNotFound);
                }
                Ok(())
            }
        }
    }
}
