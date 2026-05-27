use axum::response::Response;

use crate::workflow::result::types::PipelineContext;

pub enum StageOutcome {
    Continue,
    Done(Response),
}

pub trait Stage {
    async fn run(&self, ctx: &PipelineContext) -> StageOutcome;
}
