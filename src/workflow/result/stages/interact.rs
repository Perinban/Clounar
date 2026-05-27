use axum::{extract::State, http::StatusCode};
use uuid::Uuid;

use crate::{
    anthropic::UserSegment,
    planner::{IntentPlan, PathSource},
    workflow::{
        action::WorkflowAction,
        handle_tool_selection,
        responder::error_response,
        result::{
            traits::{Stage, StageOutcome},
            types::{ExecutionSignal, PipelineContext},
        },
        selection::{Decompose, File, SelectionContext, SelectionOutput, Step, StepOutcome},
        state::WorkflowState,
    },
};

pub struct Interact;

enum ResumeStep {
    File,
    Decompose,
}

impl Stage for Interact {
    async fn run(&self, ctx: &PipelineContext) -> StageOutcome {
        let (node_id, description) = match ctx.signal.lock().await.clone() {
            ExecutionSignal::PauseForInteraction {
                node_id,
                description,
            } => (node_id, description),
            ExecutionSignal::None => return StageOutcome::Continue,
        };

        tracing::info!(
            "[interact] resuming node_id={} resolved={:?}",
            node_id,
            description
        );

        let intent = {
            let mut wf = ctx.state.workflow.lock().await;
            wf.resume().and_then(|aw| aw.pending_intent.take())
        };

        let intent = match intent {
            Some(i) => IntentPlan {
                path: Some(PathSource::Explicit(description.clone())),
                ..i
            },
            None => {
                tracing::warn!("[interact] no pending_intent — falling back to re-selection");
                let tools = ctx.state.tools.read().await.clone();
                let user_query = vec![UserSegment::Text(description)];
                ctx.state.workflow.lock().await.reset();
                return StageOutcome::Done(
                    Box::pin(handle_tool_selection(
                        State(ctx.state.clone()),
                        ctx.ctx.clone(),
                        user_query,
                        tools,
                    ))
                    .await,
                );
            }
        };

        let cwd = ctx.state.env.lock().await.cwd.clone();
        let tools = ctx.state.tools.read().await.clone();

        let mut sel_ctx = SelectionContext {
            state: ctx.state.clone(),
            ctx: ctx.ctx.clone(),
            user_query: ctx.user_query.clone(),
            tools,
            cwd,
            query_hash: None,
            intent: Some(intent),
            output: None,
        };

        for step in [ResumeStep::File, ResumeStep::Decompose] {
            let outcome = match step {
                ResumeStep::File => File.run(&mut sel_ctx).await,
                ResumeStep::Decompose => Decompose.run(&mut sel_ctx).await,
            };
            if let StepOutcome::Done(r) = outcome {
                return StageOutcome::Done(r);
            }
        }

        match sel_ctx.output.take() {
            Some(SelectionOutput::Ready { graph, selected }) => {
                let workflow_id = Uuid::new_v4().to_string();
                tracing::info!("[interact] started workflow_id={}", workflow_id);
                *ctx.state.workflow.lock().await = WorkflowState::start(workflow_id, graph, None);
                StageOutcome::Done(
                    Box::pin(
                        WorkflowAction::FireTool {
                            tool_name: selected.tool_name,
                            tool: selected.tool,
                            compressed: selected.compressed,
                            node_id: selected.node_id,
                            user_query: ctx.user_query.clone(),
                            canonical_args_hint: selected.args_hint,
                        }
                        .execute(ctx.ctx.clone(), State(ctx.state.clone())),
                    )
                    .await,
                )
            }
            Some(SelectionOutput::EmptyGraph { .. }) => {
                tracing::warn!("[interact] empty graph after decompose");
                ctx.state.workflow.lock().await.reset();
                StageOutcome::Done(error_response(
                    StatusCode::BAD_GATEWAY,
                    "Could not build workflow for resolved file".to_string(),
                ))
            }
            Some(SelectionOutput::Recover(r)) => StageOutcome::Done(r),
            None => {
                tracing::error!("[interact] no output after decompose");
                ctx.state.workflow.lock().await.reset();
                StageOutcome::Done(error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "No output after decompose".to_string(),
                ))
            }
        }
    }
}
