use axum::{extract::State, response::Response};

use crate::{
    state::{AppState, RequestContext},
    workflow::action::{
        handlers::{text::TextHandler, tool::ToolHandler},
        traits::ActionHandler,
        types::WorkflowAction,
    },
};

impl WorkflowAction {
    pub async fn execute(self, ctx: RequestContext, state: State<AppState>) -> Response {
        match self {
            WorkflowAction::FireTool {
                tool_name,
                tool,
                compressed,
                node_id,
                user_query,
                canonical_args_hint,
            } => {
                ToolHandler {
                    tool_name,
                    tool,
                    compressed,
                    node_id,
                    user_query,
                    canonical_args_hint,
                }
                .execute(ctx, state)
                .await
            }
            WorkflowAction::RespondWithText {
                query,
                context_uuid,
            } => {
                TextHandler {
                    query,
                    context_uuid,
                }
                .execute(ctx, state)
                .await
            }
        }
    }
}
