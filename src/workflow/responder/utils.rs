use std::future::Future;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tokio::sync::mpsc;

use crate::{anthropic::SseEvent, constants::SSE_CHANNEL_BUF, server::stream_response};

pub fn error_response(status: StatusCode, msg: String) -> Response {
    (
        status,
        Json(json!({ "error": { "message": msg, "type": "api_error" } })),
    )
        .into_response()
}

pub fn spawn_sse_stream<F, Fut>(msg_id: String, model_echo: String, f: F) -> Response
where
    F: FnOnce(mpsc::Sender<String>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<String>(SSE_CHANNEL_BUF);
    tokio::spawn(async move {
        let _ = tx
            .send(
                SseEvent::MessageStart {
                    id: &msg_id,
                    model: &model_echo,
                }
                .render(),
            )
            .await;
        f(tx).await;
    });
    stream_response(rx)
}
