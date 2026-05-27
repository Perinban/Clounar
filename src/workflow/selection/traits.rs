use axum::response::Response;

use crate::workflow::selection::types::SelectionContext;

pub enum StepOutcome {
    Continue,
    Done(Response),
}

pub trait Step {
    async fn run(&self, ctx: &mut SelectionContext) -> StepOutcome;
}
