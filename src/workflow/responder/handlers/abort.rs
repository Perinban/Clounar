use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{
    anthropic::SseEvent,
    workflow::responder::{
        dispatcher::Dispatcher, traits::Respond, types::AbortKind, utils::spawn_sse_stream,
    },
};

impl Respond for AbortKind {
    async fn stream(self, d: &Dispatcher) -> Response {
        let msg_id = d.msg_id.clone();
        let model = d.ctx.model_echo.clone();
        let reason = self.reason;
        spawn_sse_stream(msg_id, model, move |tx| async move {
            let _ = tx.send(SseEvent::ContentBlockStart.render()).await;
            let _ = tx.send(SseEvent::TextDelta(&reason).render()).await;
            let _ = tx.send(SseEvent::ContentBlockStop.render()).await;
            let _ = tx.send(SseEvent::MessageDelta.render()).await;
            let _ = tx.send(SseEvent::MessageStop.render()).await;
        })
    }

    async fn blocking(self, d: &Dispatcher) -> Response {
        Json(json!({
            "id": d.msg_id,
            "type": "message",
            "role": "assistant",
            "model": d.ctx.model_echo,
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 0, "output_tokens": 0 },
            "content": [{ "type": "text", "text": self.reason }]
        }))
        .into_response()
    }
}
