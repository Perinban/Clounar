use serde_json::Value;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, RwLock};

use crate::{
    anthropic::UserSegment, artifact::ArtifactRegistry, bridge::ToolCache, config::Config,
    perplexity::PerplexitySession, planner::NodeId, workflow::state::WorkflowState,
};

#[derive(Clone, Default)]
pub struct EnvironmentContext {
    pub cwd: String,
    pub platform: String,
    pub shell: String,
    pub last_resolved_dir: Option<String>,
    pub file_index: HashMap<String, Vec<String>>,
}

#[derive(Clone)]
pub struct PendingTool {
    pub tool_name: String,
    pub tool_input: Value,
    pub node_id: NodeId,
    pub user_query: Vec<UserSegment>,
}

#[derive(Clone)]
pub struct RequestContext {
    pub stream_mode: bool,
    pub model_echo: String,
    pub mode: String,
    pub model: String,
    pub incognito: bool,
}

#[derive(Clone)]
pub struct TaskEntry {
    pub hash: String,
    pub user_query: String,
    #[allow(dead_code)]
    pub tool_name: String,
    #[allow(dead_code)]
    pub tool_result: String,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub clounar_dir: PathBuf,
    pub session: Arc<Mutex<PerplexitySession>>,
    pub workflow: Arc<Mutex<WorkflowState>>,
    pub tool_cache: Arc<RwLock<Option<ToolCache>>>,
    pub tools: Arc<RwLock<Vec<Value>>>,
    pub env: Arc<Mutex<EnvironmentContext>>,
    pub task_history: Arc<Mutex<Vec<TaskEntry>>>,
    pub artifact_registry: Arc<RwLock<ArtifactRegistry>>,
}
