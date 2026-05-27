use axum::{extract::State, response::Response};
use uuid::Uuid;

use crate::{
    state::{AppState, RequestContext},
    workflow::responder::{
        traits::Respond,
        types::{AbortKind, ResponseKind, TextKind, ToolUseKind},
    },
};

pub struct Dispatcher {
    pub ctx: RequestContext,
    pub state: State<AppState>,
    pub msg_id: String,
}

impl Dispatcher {
    pub fn new(ctx: RequestContext, state: State<AppState>) -> Self {
        Self {
            ctx,
            state,
            msg_id: format!("msg_{}", Uuid::new_v4().simple()),
        }
    }

    pub async fn send(self, kind: ResponseKind) -> Response {
        match kind {
            ResponseKind::Text {
                query,
                context_uuid,
            } => {
                let payload = TextKind {
                    query,
                    context_uuid,
                };
                if self.ctx.stream_mode {
                    payload.stream(&self).await
                } else {
                    payload.blocking(&self).await
                }
            }
            ResponseKind::ToolUse { id, name, input } => {
                let payload = ToolUseKind { id, name, input };
                if self.ctx.stream_mode {
                    payload.stream(&self).await
                } else {
                    payload.blocking(&self).await
                }
            }
            ResponseKind::Abort { reason } => {
                let payload = AbortKind { reason };
                if self.ctx.stream_mode {
                    payload.stream(&self).await
                } else {
                    payload.blocking(&self).await
                }
            }
        }
    }
}
