use axum::{extract::State, response::Response};
use uuid::Uuid;

use crate::{
    anthropic::UserSegment,
    constants::CTX_KEY_PLANNER,
    state::{AppState, RequestContext},
    workflow::{
        action::WorkflowAction,
        recovery,
        selection::{
            steps::{
                classify::Classify, decompose::Decompose, file::File, lookup::Lookup,
                warmup::Warmup,
            },
            traits::{Step, StepOutcome},
            types::{SelectionContext, SelectionOutput},
        },
        state::WorkflowState,
    },
};

enum SelectionStep {
    Warmup(Warmup),
    Lookup(Lookup),
    Classify(Classify),
    Resolve(File),
    Decompose(Decompose),
}

impl SelectionStep {
    async fn run(&self, ctx: &mut SelectionContext) -> StepOutcome {
        match self {
            Self::Warmup(s) => s.run(ctx).await,
            Self::Lookup(s) => s.run(ctx).await,
            Self::Classify(s) => s.run(ctx).await,
            Self::Resolve(s) => s.run(ctx).await,
            Self::Decompose(s) => s.run(ctx).await,
        }
    }
}

pub async fn handle_tool_selection(
    state: State<AppState>,
    ctx: RequestContext,
    user_query: Vec<UserSegment>,
    tools: Vec<serde_json::Value>,
) -> Response {
    let State(state) = state;
    let cwd = state.env.lock().await.cwd.clone();

    let mut sel_ctx = SelectionContext {
        state,
        ctx,
        user_query,
        tools,
        cwd,
        query_hash: None,
        intent: None,
        output: None,
    };

    let steps = [
        SelectionStep::Warmup(Warmup),
        SelectionStep::Lookup(Lookup),
        SelectionStep::Classify(Classify),
        SelectionStep::Resolve(File),
        SelectionStep::Decompose(Decompose),
    ];

    for step in &steps {
        if let StepOutcome::Done(r) = step.run(&mut sel_ctx).await {
            return r;
        }
    }

    let output = sel_ctx.output.take().unwrap();
    let workflow_id = Uuid::new_v4().to_string();

    match output {
        SelectionOutput::Ready { graph, selected } => {
            tracing::info!("[workflow] started workflow_id={}", workflow_id);
            let context_uuid = {
                let hash = sel_ctx.query_hash.as_deref().unwrap_or("");
                if hash.is_empty() {
                    None
                } else {
                    let session = sel_ctx.state.session.lock().await;
                    session
                        .context_uuids
                        .get(&(hash.to_string(), CTX_KEY_PLANNER.to_string()))
                        .copied()
                }
            };
            *sel_ctx.state.workflow.lock().await =
                WorkflowState::start(workflow_id, graph, context_uuid);
            WorkflowAction::FireTool {
                tool_name: selected.tool_name,
                tool: selected.tool,
                compressed: selected.compressed,
                node_id: selected.node_id,
                user_query: sel_ctx.user_query,
                canonical_args_hint: selected.args_hint,
            }
            .execute(sel_ctx.ctx, State(sel_ctx.state))
            .await
        }
        SelectionOutput::EmptyGraph { retries } => {
            recovery::graph::retry_empty_graph(
                State(sel_ctx.state),
                sel_ctx.ctx,
                sel_ctx.user_query,
                sel_ctx.tools,
                sel_ctx.cwd,
                retries,
            )
            .await
        }
        SelectionOutput::Recover(r) => r,
    }
}
