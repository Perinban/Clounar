use std::sync::Arc;

use axum::{extract::State, response::Response};
use tokio::sync::Mutex;

use crate::{
    constants::TOOL_RESULT_PREVIEW_LEN,
    state::{AppState, PendingTool, RequestContext},
    workflow::result::{
        stages::{
            advance::Advance, complete::Complete, interact::Interact, record::Record,
            respond::Respond,
        },
        traits::{Stage, StageOutcome},
        types::{ExecutionSignal, GraphPosition, PipelineContext},
        utils::{detect_kind, resolve_position},
    },
};

enum PipelineStage {
    Record(Record),
    Advance(Advance),
    Complete(Complete),
    Interact(Interact),
    Respond(Respond),
}

impl PipelineStage {
    async fn run(&self, ctx: &PipelineContext) -> StageOutcome {
        match self {
            Self::Record(s) => s.run(ctx).await,
            Self::Advance(s) => s.run(ctx).await,
            Self::Complete(s) => s.run(ctx).await,
            Self::Interact(s) => s.run(ctx).await,
            Self::Respond(s) => s.run(ctx).await,
        }
    }
}

pub struct ToolResultPipeline {
    state: AppState,
    ctx: RequestContext,
    pending: PendingTool,
    tool_result: String,
}

impl ToolResultPipeline {
    pub fn new(
        state: AppState,
        ctx: RequestContext,
        pending: PendingTool,
        tool_result: String,
    ) -> Self {
        Self {
            state,
            ctx,
            pending,
            tool_result,
        }
    }

    pub async fn run(self) -> Response {
        tracing::info!(
            "[tool_result] tool={} node_id={}",
            self.pending.tool_name,
            self.pending.node_id
        );
        tracing::debug!(
            "[tool_result] result_preview={:?}",
            &self.tool_result[..self.tool_result.len().min(TOOL_RESULT_PREVIEW_LEN)]
        );

        let kind = detect_kind(&self.tool_result);
        let position = resolve_position(&self.state, self.pending.node_id).await;

        if matches!(position, GraphPosition::OutsideGraph) {
            return axum::response::IntoResponse::into_response((
                axum::http::StatusCode::OK,
                axum::Json(serde_json::json!({})),
            ));
        }

        let context_uuid = self
            .state
            .workflow
            .lock()
            .await
            .active()
            .and_then(|aw| aw.context_uuid);

        let ctx = PipelineContext {
            state: self.state,
            ctx: self.ctx,
            tool_name: self.pending.tool_name,
            tool_input: self.pending.tool_input,
            tool_result: self.tool_result,
            node_id: self.pending.node_id,
            user_query: self.pending.user_query,
            kind,
            position,
            signal: Arc::new(Mutex::new(ExecutionSignal::None)),
            context_uuid,
        };

        let stages = [
            PipelineStage::Record(Record),
            PipelineStage::Advance(Advance),
            PipelineStage::Complete(Complete),
            PipelineStage::Interact(Interact),
            PipelineStage::Respond(Respond),
        ];

        for stage in &stages {
            if let StageOutcome::Done(r) = stage.run(&ctx).await {
                return r;
            }
        }

        axum::response::IntoResponse::into_response((
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!({})),
        ))
    }

    pub async fn handle(
        State(state): State<AppState>,
        ctx: RequestContext,
        pending: PendingTool,
        tool_result: String,
    ) -> Response {
        ToolResultPipeline::new(state, ctx, pending, tool_result)
            .run()
            .await
    }
}
