use serde_json::json;

pub enum SseEvent<'a> {
    MessageStart { id: &'a str, model: &'a str },
    ContentBlockStart,
    ToolUseBlockStart { id: &'a str, name: &'a str },
    TextDelta(&'a str),
    ToolUseDelta(&'a str),
    ContentBlockStop,
    MessageDelta,
    MessageDeltaTool,
    MessageStop,
}

impl<'a> SseEvent<'a> {
    pub fn render(&self) -> String {
        let (kind, data) = match self {
            Self::MessageStart { id, model } => (
                "message_start",
                json!({
                    "type": "message_start",
                    "message": {
                        "id": id, "type": "message", "role": "assistant",
                        "content": [], "model": model, "stop_reason": null,
                        "usage": { "input_tokens": 0 }
                    }
                }),
            ),
            Self::ContentBlockStart => (
                "content_block_start",
                json!({
                    "type": "content_block_start", "index": 0,
                    "content_block": { "type": "text", "text": "" }
                }),
            ),
            Self::ToolUseBlockStart { id, name } => (
                "content_block_start",
                json!({
                    "type": "content_block_start", "index": 0,
                    "content_block": { "type": "tool_use", "id": id, "name": name, "input": {} }
                }),
            ),
            Self::TextDelta(text) => (
                "content_block_delta",
                json!({
                    "type": "content_block_delta", "index": 0,
                    "delta": { "type": "text_delta", "text": text }
                }),
            ),
            Self::ToolUseDelta(partial_json) => (
                "content_block_delta",
                json!({
                    "type": "content_block_delta", "index": 0,
                    "delta": { "type": "input_json_delta", "partial_json": partial_json }
                }),
            ),
            Self::ContentBlockStop => (
                "content_block_stop",
                json!({
                    "type": "content_block_stop", "index": 0
                }),
            ),
            Self::MessageDelta => (
                "message_delta",
                json!({
                    "type": "message_delta",
                    "delta": { "stop_reason": "end_turn", "stop_sequence": null },
                    "usage": { "output_tokens": 0 }
                }),
            ),
            Self::MessageDeltaTool => (
                "message_delta",
                json!({
                    "type": "message_delta",
                    "delta": { "stop_reason": "tool_use", "stop_sequence": null },
                    "usage": { "output_tokens": 0 }
                }),
            ),
            Self::MessageStop => ("message_stop", json!({ "type": "message_stop" })),
        };
        format!("event: {}\ndata: {}\n\n", kind, data)
    }
}
