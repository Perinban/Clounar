use itertools::Itertools;
use serde_json::Value;

use crate::anthropic::{ContentBlock, MessageContent, MessagesRequest, Role, UserSegment};

pub enum QueryKind {
    ToolSelection {
        user_query: Vec<UserSegment>,
        tools: Vec<Value>,
    },
    Plain(String),
}

pub fn classify_request(req: &MessagesRequest) -> QueryKind {
    if req.tools.as_ref().is_some_and(|t| !t.is_empty()) {
        let user_query = extract_user_query(req);
        let tools = req.tools.clone().unwrap_or_default();
        tracing::debug!("[classify] ToolSelection tools={}", tools.len());
        return QueryKind::ToolSelection { user_query, tools };
    }

    let system_text = req.system.iter().filter_map(|system| {
        let text = match system {
            Value::String(s) => s.clone(),
            Value::Array(blocks) => blocks
                .iter()
                .filter_map(|b| b.get("text")?.as_str().map(|s| s.to_string()))
                .join("\n"),
            _ => String::new(),
        };
        (!text.is_empty()).then_some(text)
    });

    let message_texts = req.messages.iter().filter_map(|msg| {
        let text = match &msg.content {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.clone()),
                    ContentBlock::ToolResult { content, .. } => match content {
                        Value::String(s) => Some(s.clone()),
                        Value::Array(arr) => Some(
                            arr.iter()
                                .filter_map(|b| b.get("text")?.as_str().map(|s| s.to_string()))
                                .join("\n"),
                        ),
                        _ => Some(content.to_string()),
                    },
                    _ => None,
                })
                .join("\n"),
        };
        (!text.is_empty()).then_some(text)
    });

    let result = system_text.chain(message_texts).join("\n\n");
    tracing::debug!("[classify] Plain query_len={}", result.len());
    QueryKind::Plain(result)
}

fn strip_system_tags(text: &str) -> String {
    let mut result = text.to_string();
    for tag in &["system-reminder", "task-notification", "tool-use-id"] {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);
        let mut buf = String::new();
        let mut rest = result.as_str();
        loop {
            match rest.find(open.as_str()) {
                None => {
                    buf.push_str(rest);
                    break;
                }
                Some(start) => {
                    buf.push_str(&rest[..start]);
                    match rest.find(close.as_str()) {
                        Some(end) => rest = &rest[end + close.len()..],
                        None => {
                            break;
                        }
                    }
                }
            }
        }
        result = buf;
    }
    result
}

fn extract_user_query(req: &MessagesRequest) -> Vec<UserSegment> {
    req.messages
        .iter()
        .filter(|m| m.role == Role::User)
        .filter_map(|m| match &m.content {
            MessageContent::Text(s) => {
                let s = strip_system_tags(s);
                if s.trim().is_empty() {
                    None
                } else {
                    Some(vec![UserSegment::Text(s)])
                }
            }
            MessageContent::Blocks(blocks) => {
                let segments: Vec<UserSegment> = blocks
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text { text } => {
                            let t = strip_system_tags(text);
                            let t = t.trim().to_string();
                            if t.is_empty() {
                                return None;
                            }
                            tracing::debug!("[classify] block text={:?}", &t[..t.len().min(200)]);
                            if t.starts_with("```") {
                                Some(UserSegment::Code(t))
                            } else {
                                Some(UserSegment::Text(t))
                            }
                        }
                        ContentBlock::ToolResult { content, .. } => {
                            let s = match content {
                                Value::String(s) => s.clone(),
                                Value::Array(arr) => arr
                                    .iter()
                                    .filter_map(|b| b.get("text")?.as_str().map(|s| s.to_string()))
                                    .join("\n"),
                                _ => content.to_string(),
                            };
                            if s.is_empty() || s.contains("<task-notification>") {
                                return None;
                            }
                            Some(UserSegment::ToolResult(s))
                        }
                        _ => None,
                    })
                    .collect();
                if segments.is_empty() {
                    None
                } else {
                    Some(segments)
                }
            }
        })
        .next_back()
        .unwrap_or_default()
}
