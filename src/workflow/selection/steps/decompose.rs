use axum::extract::State;

use crate::{
    bridge::tools::ToolEntry,
    planner::TaskDecomposition,
    validation::{
        validate_intent, ContractContext, ExecutionContractError, SemanticContext, Validate,
    },
    workflow::{
        responder::respond_abort,
        selection::{
            traits::{Step, StepOutcome},
            types::{SelectedTool, SelectionContext, SelectionOutput},
        },
    },
};

pub struct Decompose;

impl Step for Decompose {
    async fn run(&self, ctx: &mut SelectionContext) -> StepOutcome {
        let intent = ctx.intent.as_ref().unwrap();

        let validated = match validate_intent(intent, &ctx.cwd) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("[selection] intent validation failed: {}", e);
                ctx.state.workflow.lock().await.reset();
                return StepOutcome::Done(
                    respond_abort(
                        State(ctx.state.clone()),
                        ctx.ctx.clone(),
                        format!("Intent validation failed: {}", e),
                    )
                    .await,
                );
            }
        };

        {
            let cache = ctx.state.tool_cache.read().await;
            let registry = &cache.as_ref().unwrap().registry;
            if let Err(e) = validated.validate(&SemanticContext { registry }) {
                tracing::warn!("[selection] semantic validation failed: {}", e);
                ctx.state.workflow.lock().await.reset();
                return StepOutcome::Done(
                    respond_abort(
                        State(ctx.state.clone()),
                        ctx.ctx.clone(),
                        format!("Semantic validation failed: {}", e),
                    )
                    .await,
                );
            }
        }

        let graph = {
            let cache = ctx.state.tool_cache.read().await;
            let mut base_intent = intent.clone();
            base_intent.path = validated
                .canonical_subject
                .as_ref()
                .map(|s| crate::planner::PathSource::Explicit(s.clone()));
            let decomposition = TaskDecomposition::from_plan(base_intent);
            tracing::debug!(
                "[selection] decomposition tasks={}",
                decomposition.tasks.len()
            );
            let registry = &cache.as_ref().unwrap().registry;
            match decomposition.build(registry, &ctx.tools, &ctx.cwd) {
                Ok(g) => g,
                Err(e) => {
                    tracing::warn!("[selection] build failed: {}", e);
                    let retries = ctx
                        .state
                        .workflow
                        .lock()
                        .await
                        .active()
                        .map(|w| w.retry.classifier_retries)
                        .unwrap_or(0);
                    ctx.output = Some(SelectionOutput::EmptyGraph { retries });
                    return StepOutcome::Continue;
                }
            }
        };

        if let Err(e) = graph.validate() {
            tracing::error!("[selection] graph validation failed: {}", e);
            ctx.state.workflow.lock().await.reset();
            ctx.output = Some(SelectionOutput::Recover(
                respond_abort(
                    State(ctx.state.clone()),
                    ctx.ctx.clone(),
                    format!("Invalid graph: {}", e),
                )
                .await,
            ));
            return StepOutcome::Continue;
        }

        match Validate::validate(&graph, &ContractContext) {
            Ok(()) => {}
            Err(ExecutionContractError::EmptyGraph) => {
                let retries = ctx
                    .state
                    .workflow
                    .lock()
                    .await
                    .active()
                    .map(|w| w.retry.classifier_retries)
                    .unwrap_or(0);
                ctx.output = Some(SelectionOutput::EmptyGraph { retries });
                return StepOutcome::Continue;
            }
            Err(e) => {
                tracing::error!("[selection] contract validation failed: {}", e);
                ctx.state.workflow.lock().await.reset();
                ctx.output = Some(SelectionOutput::Recover(
                    respond_abort(
                        State(ctx.state.clone()),
                        ctx.ctx.clone(),
                        format!("Contract validation failed: {}", e),
                    )
                    .await,
                ));
                return StepOutcome::Continue;
            }
        }

        let first_node_id = graph.ordering[0];
        let tool_name = graph.nodes[&first_node_id].tool_name.clone();
        let args_hint = graph.nodes[&first_node_id].args_hint.clone();

        let tool = match ToolEntry::find_in_tools(&tool_name, &ctx.tools) {
            Some(t) => t.clone(),
            None => {
                ctx.state.workflow.lock().await.reset();
                ctx.output = Some(SelectionOutput::Recover(
                    respond_abort(
                        State(ctx.state.clone()),
                        ctx.ctx.clone(),
                        format!("Invalid tool selection: {}", tool_name),
                    )
                    .await,
                ));
                return StepOutcome::Continue;
            }
        };

        let compressed = ToolEntry::resolve_compressed(&State(ctx.state.clone()), &tool_name).await;

        ctx.output = Some(SelectionOutput::Ready {
            graph,
            selected: SelectedTool {
                tool_name,
                tool,
                compressed,
                node_id: first_node_id,
                args_hint,
            },
        });
        StepOutcome::Continue
    }
}
