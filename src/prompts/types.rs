use serde_json::Value;

use crate::{
    anthropic::UserSegment, artifact::GraphArtifactRef, planner::CompressedTool,
    state::EnvironmentContext,
};

use super::config::PromptsConfig;

pub struct PromptContext<'a> {
    pub tool: Option<&'a Value>,
    pub compressed: Option<&'a CompressedTool>,
    pub user_query: &'a [UserSegment],
    pub env: Option<&'a EnvironmentContext>,
    pub artifact_refs: &'a [(GraphArtifactRef, String)],
    pub args_hint: Option<&'a Value>,
    pub prompts: &'a PromptsConfig,
}

pub enum PromptKind<'a> {
    Compress,
    IntentClassify,
    Args,
    ToolResult {
        tool_name: &'a str,
        tool_input: &'a Value,
        tool_result: &'a str,
    },
    HashSelect {
        candidates: &'a str,
    },
}
