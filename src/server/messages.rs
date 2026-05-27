use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use itertools::Itertools;
use serde_json::{from_str, json, Value};

use crate::{
    anthropic::{ContentBlock, MessageContent, MessagesRequest},
    constants::{SYS_PREFIX_CWD, SYS_PREFIX_PLATFORM, SYS_PREFIX_SHELL, TITLE_TRUNCATE_LEN},
    server::{classify_request, QueryKind},
    state::{AppState, PendingTool, RequestContext},
    workflow::{handle_tool_selection, respond, ToolResultPipeline},
};

enum Dispatch {
    ToolResult {
        pending: PendingTool,
        result: String,
    },
    Echo,
    Select,
}

struct MessageClassifier {
    system_contains: Option<&'static str>,
    message_contains: Option<&'static str>,
}

impl MessageClassifier {
    const TITLE: Self = Self {
        system_contains: Some("Generate a concise, sentence-case title"),
        message_contains: None,
    };
    const SUGGESTION: Self = Self {
        system_contains: None,
        message_contains: Some("[SUGGESTION MODE:"),
    };

    fn matches(&self, system: &Option<String>, message: &Option<String>) -> bool {
        let check = |pat: Option<&str>, val: &Option<String>| {
            pat.map_or(true, |p| val.as_deref().is_some_and(|s| s.contains(p)))
        };
        check(self.system_contains, system) && check(self.message_contains, message)
    }
}

enum ShortCircuit {
    Title(String),
    Suggestion,
    Echo,
}

impl ShortCircuit {
    fn respond(self, model: &str) -> Response {
        match self {
            ShortCircuit::Echo => (StatusCode::OK, Json(json!({}))).into_response(),
            ShortCircuit::Title(text) => Json(json!({
                "id": "msg_title",
                "type": "message",
                "role": "assistant",
                "model": model,
                "stop_reason": "end_turn",
                "usage": { "input_tokens": 0, "output_tokens": 0 },
                "content": [{ "type": "text", "text": format!("{{\"title\":\"{}\"}}", text) }]
            }))
            .into_response(),
            ShortCircuit::Suggestion => Json(json!({
                "id": "msg_suggestion",
                "type": "message",
                "role": "assistant",
                "model": model,
                "stop_reason": "end_turn",
                "usage": { "input_tokens": 0, "output_tokens": 0 },
                "content": [{ "type": "text", "text": "" }]
            }))
            .into_response(),
        }
    }
}

pub async fn messages(State(state): State<AppState>, body: axum::body::Bytes) -> Response {
    let body_str = String::from_utf8_lossy(&body);
    let req: MessagesRequest = match from_str(&body_str) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to parse request: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": { "message": e.to_string(), "type": "invalid_request" } })),
            )
                .into_response();
        }
    };

    let model = req.model.clone();
    let system_str = req.system.as_ref().map(|s| {
        s.as_str().map(|s| s.to_string()).unwrap_or_else(|| {
            s.as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|b| {
                            b.get("text")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                        .join("\n")
                })
                .unwrap_or_default()
        })
    });
    let message_text = req.messages.last().and_then(|m| match &m.content {
        MessageContent::Text(s) => Some(s.clone()),
        MessageContent::Blocks(blocks) => blocks.iter().find_map(|b| match b {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        }),
    });

    if MessageClassifier::TITLE.matches(&system_str, &message_text) {
        let text = message_text
            .as_deref()
            .unwrap_or("Coding session")
            .chars()
            .take(TITLE_TRUNCATE_LEN)
            .collect();
        return ShortCircuit::Title(text).respond(&model);
    }

    if MessageClassifier::SUGGESTION.matches(&system_str, &message_text) {
        return ShortCircuit::Suggestion.respond(&model);
    }

    let stream_mode = req.stream.unwrap_or(false);
    let mode = state.config.perplexity.default_mode.clone();
    let perplexity_model = state.config.perplexity.default_model.clone();

    if let Some(system) = &req.system {
        if let Some(arr) = system.as_array() {
            let mut env = state.env.lock().await;
            for block in arr {
                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                    for line in text.lines() {
                        let line = line.trim().trim_start_matches("- ");
                        if let Some(v) = line.strip_prefix(SYS_PREFIX_CWD) {
                            env.cwd = v.trim().to_string();
                        } else if let Some(v) = line.strip_prefix(SYS_PREFIX_PLATFORM) {
                            env.platform = v.trim().to_string();
                        } else if let Some(v) = line.strip_prefix(SYS_PREFIX_SHELL) {
                            env.shell = v.trim().to_string();
                        }
                    }
                }
            }
        }
    }

    let ctx = RequestContext {
        stream_mode,
        model_echo: model.clone(),
        mode,
        model: perplexity_model,
        incognito: state.config.perplexity.incognito,
    };

    let inbound_tool_result = req.messages.last().and_then(|last| match &last.content {
        MessageContent::Blocks(blocks) => blocks.iter().find_map(|b| match b {
            ContentBlock::ToolResult { content, .. } => match content {
                Value::String(s) => Some(s.clone()),
                Value::Array(arr) => Some(
                    arr.iter()
                        .filter_map(|b: &Value| {
                            b.get("text")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                        .join("\n"),
                ),
                _ => Some(content.to_string()),
            },
            _ => None,
        }),
        _ => None,
    });

    let dispatch = {
        let mut wf = state.workflow.lock().await;
        tracing::debug!(
            "[messages] inbound_tool_result={} wf_active={} wf_paused={}",
            inbound_tool_result.is_some(),
            wf.active().is_some(),
            wf.is_paused()
        );
        match inbound_tool_result {
            Some(result) => match wf.active_mut().and_then(|aw| aw.pending_tool.take()) {
                Some(pending) => Dispatch::ToolResult { pending, result },
                None => Dispatch::Select,
            },
            None => match wf {
                ref w if w.is_paused() => Dispatch::Echo,
                ref w if w.active().and_then(|aw| aw.pending_tool.as_ref()).is_some() => {
                    Dispatch::Echo
                }
                _ => Dispatch::Select,
            },
        }
    };

    match dispatch {
        Dispatch::ToolResult { pending, result } => {
            tracing::info!(
                "[messages] resolved ToolResult from pending: tool={} node_id={}",
                pending.tool_name,
                pending.node_id,
            );
            ToolResultPipeline::handle(State(state), ctx, pending, result).await
        }
        Dispatch::Echo => {
            tracing::warn!(
                "[messages] tool_result arrived with no active workflow to receive it — echoing empty response"
            );
            ShortCircuit::Echo.respond(&model)
        }
        Dispatch::Select => match classify_request(&req) {
            QueryKind::ToolSelection { user_query, tools } => {
                state.workflow.lock().await.reset();
                tracing::info!("[messages] dispatch=tool_selection tools={}", tools.len());
                handle_tool_selection(State(state), ctx, user_query, tools).await
            }
            QueryKind::Plain(query) => {
                tracing::info!("[messages] dispatch=plain");
                respond(State(state), ctx, query).await
            }
        },
    }
}
