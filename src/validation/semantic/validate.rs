use std::path::Path;

use crate::{
    planner::{ActionClass, CapabilityRegistry, HasCapability},
    validation::{Validate, ValidatedIntent},
};

use super::types::SemanticValidationError;

pub struct SemanticContext<'a> {
    pub registry: &'a CapabilityRegistry,
}

impl<'ctx> Validate<'ctx> for ValidatedIntent {
    type Context = SemanticContext<'ctx>;
    type Error = SemanticValidationError;

    fn validate(&self, ctx: &'ctx Self::Context) -> Result<(), Self::Error> {
        let kind = self.original.action.capability();

        if ctx
            .registry
            .by_kind
            .get(&kind)
            .map_or(true, |v: &Vec<String>| v.is_empty())
        {
            tracing::warn!(
                "[validation] action={:?} result=Invalid(NoCompatibleCapability)",
                self.original.action
            );
            return Err(SemanticValidationError::NoCompatibleCapability);
        }

        if matches!(self.original.action, ActionClass::WriteFile) {
            if let Some(canonical) = &self.canonical_subject {
                if let Some(parent) = Path::new(canonical).parent() {
                    if !parent.exists() {
                        tracing::warn!("[validation] action=WriteFile parent={:?} result=Invalid(ParentDirectoryMissing)", parent);
                        return Err(SemanticValidationError::ParentDirectoryMissing);
                    }
                }
            }
        }

        Ok(())
    }
}
