use crate::{
    planner::{CapabilityKind, ExecutionStep},
    validation::{FileSystemState, ResolveError},
};

pub trait HasCapability {
    fn capability(&self) -> CapabilityKind;
}

pub trait Resolvable {
    fn resolve(
        &self,
        fs_state: Option<&FileSystemState>,
    ) -> Result<Vec<ExecutionStep>, ResolveError>;
}
