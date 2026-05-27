use serde_json::Value;

use crate::{
    bridge::ToolEntry,
    planner::{
        ActionClass, CapabilityRegistry, ExecutionStep, IntentPlan, PathSource, Resolvable,
        SemanticOp, WriteDisposition,
    },
    validation::{check, ResolveError},
    workflow::workspace::WorkspaceInfo,
};

pub struct TaskDecomposition {
    pub tasks: Vec<TaskIntent>,
}

pub struct TaskIntent {
    pub action: ActionClass,
    pub relative_path: Option<PathSource>,
    pub depends_on: Vec<usize>,
    pub write_disposition: Option<WriteDisposition>,
}

impl TaskDecomposition {
    pub fn from_plan(plan: IntentPlan) -> Self {
        TaskDecomposition {
            tasks: vec![TaskIntent {
                action: plan.action,
                relative_path: plan.path,
                depends_on: vec![],
                write_disposition: plan.write_disposition,
            }],
        }
    }
}

impl TaskIntent {
    pub fn resolve_steps(
        &self,
        registry: &CapabilityRegistry,
        tools: &[Value],
        cwd: &str,
        workspace: &WorkspaceInfo,
    ) -> Vec<ExecutionStep> {
        if let Some(path) = &self.relative_path {
            let fs = check(path.as_str(), cwd);
            if let Some(op) =
                SemanticOp::from_action(&self.action, self.write_disposition.as_ref(), &fs)
            {
                match op.resolve(Some(&fs)) {
                    Ok(steps) => {
                        let filtered: Vec<_> = steps
                            .into_iter()
                            .filter(|s| ToolEntry::find_in_tools(&s.tool_name, tools).is_some())
                            .collect();
                        if !filtered.is_empty() {
                            return filtered;
                        }
                    }
                    Err(ResolveError::TargetIsDirectory) => {
                        if let Ok(steps) = SemanticOp::ListDirectory.resolve(Some(&fs)) {
                            let filtered: Vec<_> = steps
                                .into_iter()
                                .filter(|s| ToolEntry::find_in_tools(&s.tool_name, tools).is_some())
                                .collect();
                            if !filtered.is_empty() {
                                return filtered;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }

        if matches!(self.action, ActionClass::ExecuteCommand) {
            if let Some(cmd) = workspace.test_command {
                if let Some(fallback) = registry.fallback(&self.action, tools) {
                    return vec![ExecutionStep {
                        args_hint: Some(serde_json::json!({ "command": cmd })),
                        ..fallback
                    }];
                }
            }
        }

        vec![]
    }
}
