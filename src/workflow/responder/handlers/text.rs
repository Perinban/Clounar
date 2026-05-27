use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::{
    anthropic::{MessagesResponse, ResponseContent, SseEvent, UsageInfo},
    perplexity::{search::search, types::SearchParams, SearchMode},
    workflow::responder::{
        dispatcher::Dispatcher,
        traits::Respond,
        types::TextKind,
        utils::{error_response, spawn_sse_stream},
    },
};

impl Respond for TextKind {
    async fn stream(self, d: &Dispatcher) -> Response {
        let session = d.state.session.clone();
        let msg_id = d.msg_id.clone();
        let ctx = d.ctx.clone();
        spawn_sse_stream(msg_id, ctx.model_echo.clone(), move |tx| async move {
            let _ = tx.send(SseEvent::ContentBlockStart.render()).await;
            let tx_chunk = tx.clone();
            let result = search(
                &mut *session.lock().await,
                &SearchParams {
                    query: &self.query,
                    mode: &ctx.mode,
                    model: &ctx.model,
                    incognito: ctx.incognito,
                    search_mode: &SearchMode::Writing,
                    context_uuid: self.context_uuid,
                },
                |delta| {
                    let _ = tx_chunk.try_send(SseEvent::TextDelta(&delta).render());
                },
            )
            .await;
            if let Err(e) = result {
                tracing::error!("[responder] search failed (stream): {}", e);
                let _ = tx
                    .send(SseEvent::TextDelta(&format!("[Error: {}]", e)).render())
                    .await;
            }
            let _ = tx.send(SseEvent::ContentBlockStop.render()).await;
            let _ = tx.send(SseEvent::MessageDelta.render()).await;
            let _ = tx.send(SseEvent::MessageStop.render()).await;
        })
    }

    async fn blocking(self, d: &Dispatcher) -> Response {
        match search(
            &mut *d.state.session.lock().await,
            &SearchParams {
                query: &self.query,
                mode: &d.ctx.mode,
                model: &d.ctx.model,
                incognito: d.ctx.incognito,
                search_mode: &SearchMode::Writing,
                context_uuid: self.context_uuid,
            },
            |_| {},
        )
        .await
        {
            Ok(answer) => Json(MessagesResponse {
                id: d.msg_id.clone(),
                kind: "message".to_string(),
                role: "assistant".to_string(),
                content: vec![ResponseContent {
                    kind: "text".to_string(),
                    text: answer,
                }],
                model: d.ctx.model_echo.clone(),
                stopreason: "end_turn".to_string(),
                usage: UsageInfo {
                    input_tokens: 0,
                    output_tokens: 0,
                },
            })
            .into_response(),
            Err(e) => {
                tracing::error!("[responder] search failed (blocking): {}", e);
                error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        }
    }
}
