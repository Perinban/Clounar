use std::{collections::HashMap, path::Path};

use axum::extract::State;
use uuid::Uuid;

use crate::{
    anthropic::UserSegment,
    bridge::ToolEntry,
    planner::{
        CapabilityKind, ExecutionGraph, ExecutionNode, IntentPlan, NodeId, NodeKind, PathSource,
    },
    workflow::{
        action::WorkflowAction,
        selection::{
            traits::{Step, StepOutcome},
            types::SelectionContext,
        },
        state::WorkflowState,
    },
};

pub struct File;

enum FileResolution<'a> {
    Exact(&'a str),
    Cwd(&'a str),
    LastDir(&'a str),
    Ambiguous,
}

impl Step for File {
    async fn run(&self, ctx: &mut SelectionContext) -> StepOutcome {
        let base = ctx.intent.as_ref().unwrap();
        let rel = match &base.path {
            Some(PathSource::Explicit(r)) if !r.contains('/') => r.as_str(),
            _ => return StepOutcome::Continue,
        };

        let env = ctx.state.env.lock().await;
        let matches = match env.file_index.get(rel) {
            Some(m) => m.clone(),
            None => return StepOutcome::Continue,
        };

        let resolution = match matches.len() {
            1 => FileResolution::Exact(&matches[0]),
            _ => match matches.iter().find(|p| {
                Path::new(p)
                    .parent()
                    .map(|d| d.to_string_lossy() == env.cwd.as_str())
                    .unwrap_or(false)
            }) {
                Some(p) => FileResolution::Cwd(p),
                None => match env.last_resolved_dir.as_deref().and_then(|dir| {
                    matches.iter().find(|p| {
                        Path::new(p)
                            .parent()
                            .map(|d| d.to_string_lossy() == dir)
                            .unwrap_or(false)
                    })
                }) {
                    Some(p) => FileResolution::LastDir(p),
                    None => FileResolution::Ambiguous,
                },
            },
        };

        match resolution {
            FileResolution::Exact(p) | FileResolution::Cwd(p) | FileResolution::LastDir(p) => {
                let resolved = IntentPlan {
                    path: Some(PathSource::Explicit(p.to_string())),
                    ..base.clone()
                };
                drop(env);
                ctx.intent = Some(resolved);
                return StepOutcome::Continue;
            }
            FileResolution::Ambiguous => {}
        }
        drop(env);

        // Ambiguous — ask user
        let query = ctx
            .user_query
            .iter()
            .filter_map(|s| match s {
                UserSegment::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        let options: Vec<serde_json::Value> = matches
            .iter()
            .map(|p| {
                let desc = match &base.path {
                    Some(PathSource::Explicit(name)) => query.replace(name.as_str(), p),
                    _ => query.clone(),
                };
                serde_json::json!({ "label": p, "description": desc })
            })
            .collect();

        let args_hint = serde_json::json!({
            "questions": [{
                "question": "Multiple files match — which one did you mean?",
                "header": "Which file?",
                "options": options,
                "multiSelect": false
            }]
        });

        let cache = ctx.state.tool_cache.read().await;
        let tool_name = cache
            .as_ref()
            .unwrap()
            .registry
            .by_kind
            .get(&CapabilityKind::Interact)
            .and_then(|names| names.first())
            .cloned();
        drop(cache);

        let tool_name = match tool_name {
            Some(n) => n,
            None => return StepOutcome::Continue,
        };

        let tool = match ToolEntry::find_in_tools(&tool_name, &ctx.tools) {
            Some(t) => t.clone(),
            None => return StepOutcome::Continue,
        };

        let compressed =
            ToolEntry::resolve_compressed(&axum::extract::State(ctx.state.clone()), &tool_name)
                .await;

        let node_id: NodeId = 1;
        let mut nodes = HashMap::new();
        nodes.insert(
            node_id,
            ExecutionNode {
                id: node_id,
                kind: NodeKind::Interaction,
                tool_name: tool_name.clone(),
                required_artifact_refs: vec![],
                produces_artifacts: vec![],
                args_hint: Some(args_hint.clone()),
            },
        );

        let graph = ExecutionGraph {
            nodes,
            ordering: vec![node_id],
            dependencies: HashMap::new(),
        };

        tracing::info!(
            "[file] ambiguous rel={} matches={} — emitting AskUserQuestion",
            rel,
            matches.len()
        );

        let workflow_id = Uuid::new_v4().to_string();
        let mut wf = WorkflowState::start(workflow_id, graph, None);
        if let Some(aw) = wf.active_mut() {
            aw.pending_intent = Some(base.clone());
        }
        *ctx.state.workflow.lock().await = wf;

        StepOutcome::Done(
            Box::pin(
                WorkflowAction::FireTool {
                    tool_name,
                    tool,
                    compressed,
                    node_id,
                    user_query: ctx.user_query.clone(),
                    canonical_args_hint: Some(args_hint),
                }
                .execute(ctx.ctx.clone(), State(ctx.state.clone())),
            )
            .await,
        )
    }
}
