mod dispatcher;
mod handlers;
mod traits;
pub mod types;
mod utils;

use axum::{extract::State, response::Response};

use crate::state::{AppState, RequestContext};

pub use dispatcher::Dispatcher;
pub use types::ResponseKind;
pub use utils::error_response;

pub async fn respond(state: State<AppState>, ctx: RequestContext, query: String) -> Response {
    Dispatcher::new(ctx, state)
        .send(ResponseKind::Text {
            query,
            context_uuid: None,
        })
        .await
}

pub async fn respond_abort(
    state: State<AppState>,
    ctx: RequestContext,
    reason: String,
) -> Response {
    Dispatcher::new(ctx, state)
        .send(ResponseKind::Abort { reason })
        .await
}
