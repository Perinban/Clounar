use axum::{extract::State, response::Response};

use crate::{
    state::{AppState, RequestContext},
    workflow::{
        action::traits::ActionHandler,
        responder::{Dispatcher, ResponseKind},
    },
};

pub struct TextHandler {
    pub query: String,
    pub context_uuid: Option<uuid::Uuid>,
}

impl ActionHandler for TextHandler {
    async fn execute(self, ctx: RequestContext, state: State<AppState>) -> Response {
        Dispatcher::new(ctx, state)
            .send(ResponseKind::Text {
                query: self.query,
                context_uuid: self.context_uuid,
            })
            .await
    }
}
