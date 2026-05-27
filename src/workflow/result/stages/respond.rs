use axum::extract::State;

use crate::{
    constants::CTX_KEY_RESPOND,
    prompts::{PromptContext, PromptKind},
    workflow::{
        action::WorkflowAction,
        result::{
            traits::{Stage, StageOutcome},
            types::{GraphPosition, PipelineContext, ToolResultKind},
        },
    },
};

pub struct Respond;

impl Stage for Respond {
    async fn run(&self, ctx: &PipelineContext) -> StageOutcome {
        if !matches!(ctx.position, GraphPosition::Terminal)
            || !matches!(ctx.kind, ToolResultKind::Success)
        {
            return StageOutcome::Continue;
        }

        let query = PromptContext {
            tool: None,
            compressed: None,
            user_query: &ctx.user_query,
            env: None,
            artifact_refs: &[],
            args_hint: None,
            prompts: &ctx.state.config.prompts,
        }
        .build(PromptKind::ToolResult {
            tool_name: &ctx.tool_name,
            tool_input: &ctx.tool_input,
            tool_result: &ctx.tool_result,
        });

        let context_uuid = ctx
            .context_uuid
            .map(|u| uuid::Uuid::new_v5(&u, CTX_KEY_RESPOND.as_bytes()));

        StageOutcome::Done(
            Box::pin(
                WorkflowAction::RespondWithText {
                    query,
                    context_uuid,
                }
                .execute(ctx.ctx.clone(), State(ctx.state.clone())),
            )
            .await,
        )
    }
}
