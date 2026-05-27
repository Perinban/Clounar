use axum::{extract::State, response::Response};

use crate::state::{AppState, RequestContext};

pub trait ActionHandler {
    async fn execute(self, ctx: RequestContext, state: State<AppState>) -> Response;
}
