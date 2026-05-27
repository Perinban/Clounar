use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{
    anthropic::SseEvent,
    workflow::responder::{
        dispatcher::Dispatcher, traits::Respond, types::ToolUseKind, utils::spawn_sse_stream,
    },
};

impl Respond for ToolUseKind {
    async fn stream(self, d: &Dispatcher) -> Response {
        let msg_id = d.msg_id.clone();
        let ctx = d.ctx.clone();
        let tool_id = self.id;
        let tool_name = self.name;
        let input = self.input;
        spawn_sse_stream(msg_id, ctx.model_echo.clone(), move |tx| async move {
            let _ = tx
                .send(
                    SseEvent::ToolUseBlockStart {
                        id: &tool_id,
                        name: &tool_name,
                    }
                    .render(),
                )
                .await;
            let _ = tx
                .send(SseEvent::ToolUseDelta(&input.to_string()).render())
                .await;
            let _ = tx.send(SseEvent::ContentBlockStop.render()).await;
            let _ = tx.send(SseEvent::MessageDeltaTool.render()).await;
            let _ = tx.send(SseEvent::MessageStop.render()).await;
        })
    }

    async fn blocking(self, d: &Dispatcher) -> Response {
        Json(json!({
            "id": d.msg_id,
            "type": "message",
            "role": "assistant",
            "model": d.ctx.model_echo,
            "stop_reason": "tool_use",
            "usage": { "input_tokens": 0, "output_tokens": 0 },
            "content": [{ "type": "tool_use", "id": self.id, "name": self.name, "input": self.input }]
        }))
        .into_response()
    }
}
