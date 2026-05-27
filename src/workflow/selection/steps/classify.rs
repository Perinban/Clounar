use std::sync::Arc;

use axum::extract::State;

use crate::{
    planner::{action::TaskKind, ActionClass, IntentPlan, PathSource},
    prompts::PromptKind,
    workflow::{
        responder::respond_abort,
        selection::{
            traits::{Step, StepOutcome},
            types::SelectionContext,
            utils::{build_prompt, run_search},
        },
    },
};

pub struct Classify;

impl Step for Classify {
    async fn run(&self, ctx: &mut SelectionContext) -> StepOutcome {
        let prompt = build_prompt(ctx, PromptKind::IntentClassify);
        let session = Arc::clone(&ctx.state.session);
        let mode = ctx.ctx.mode.clone();
        let model = ctx.ctx.model.clone();
        let incognito = ctx.ctx.incognito;
        match run_search(session, mode, model, incognito, prompt).await {
            Ok(raw) => {
                let fallback = IntentPlan {
                    action: ActionClass::WebSearch,
                    task_kind: TaskKind::Knowledge,
                    path: None,
                    write_disposition: None,
                };
                let classified = IntentPlan::parse(&raw).unwrap_or(fallback);

                let resolved = if classified.task_kind == TaskKind::Knowledge
                    && !matches!(classified.action, ActionClass::DebugCode)
                {
                    if !matches!(classified.action, ActionClass::WebSearch) {
                        IntentPlan {
                            action: ActionClass::WebSearch,
                            ..classified
                        }
                    } else {
                        classified
                    }
                } else {
                    classified
                };

                tracing::debug!(
                    "[selection] classifier intent action={:?} task_kind={:?} write_disposition={:?}",
                    resolved.action,
                    resolved.task_kind,
                    resolved.write_disposition
                );
                let resolved =
                    if resolved.action == ActionClass::ListDirectory && resolved.path.is_none() {
                        let path = match ctx.state.env.lock().await.last_resolved_dir.clone() {
                            Some(dir) => PathSource::LastDir(dir),
                            None => PathSource::Cwd,
                        };
                        IntentPlan {
                            path: Some(path),
                            ..resolved
                        }
                    } else {
                        resolved
                    };
                ctx.intent = Some(resolved);
                StepOutcome::Continue
            }
            Err(e) => {
                tracing::error!("[selection] classifier failed: {}", e);
                StepOutcome::Done(
                    respond_abort(State(ctx.state.clone()), ctx.ctx.clone(), e.to_string()).await,
                )
            }
        }
    }
}
