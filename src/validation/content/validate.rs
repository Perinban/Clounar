use crate::validation::Validate;

use super::types::{ContentError, TransformContext};

pub struct Transform<'a> {
    pub old_string: &'a str,
}

impl<'ctx> Validate<'ctx> for Transform<'_> {
    type Context = TransformContext<'ctx>;
    type Error = ContentError;

    fn validate(&self, ctx: &'ctx Self::Context) -> Result<(), Self::Error> {
        if ctx.file_content.contains(self.old_string) {
            Ok(())
        } else {
            Err(ContentError::OldStringNotFound)
        }
    }
}
