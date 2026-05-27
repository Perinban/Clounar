use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug)]
pub enum UserSegment {
    Text(String),
    Code(String),
    ToolResult(String),
}

#[derive(Deserialize, Debug)]
pub struct MessagesRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub system: Option<Value>,
    #[allow(dead_code)]
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
    pub tools: Option<Vec<Value>>,
    #[allow(dead_code)]
    pub tool_choice: Option<Value>,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Deserialize, Debug)]
pub struct Message {
    pub role: Role,
    pub content: MessageContent,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        name: String,
        #[allow(dead_code)]
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        #[allow(dead_code)]
        tool_use_id: String,
        content: Value,
    },
}

#[derive(Serialize)]
pub struct UsageInfo {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Serialize)]
pub struct MessagesResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub role: String,
    pub content: Vec<ResponseContent>,
    pub model: String,
    #[serde(rename = "stop_reason")]
    pub stopreason: String,
    pub usage: UsageInfo,
}

#[derive(Serialize)]
pub struct ResponseContent {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: String,
}
