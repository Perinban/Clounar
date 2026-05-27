use std::{collections::HashMap, path::Path};

use axum::extract::State;
use ignore::WalkBuilder;

use crate::{
    bridge::ToolCache,
    workflow::{
        responder::respond_abort,
        selection::{
            traits::{Step, StepOutcome},
            types::SelectionContext,
        },
    },
};

pub struct Warmup;

impl Step for Warmup {
    async fn run(&self, ctx: &mut SelectionContext) -> StepOutcome {
        if let Err(e) = ToolCache::init(
            &ctx.state.tool_cache,
            &ctx.state.session,
            &ctx.ctx,
            &ctx.tools,
            &ctx.state.clounar_dir,
            &ctx.state.config.prompts,
        )
        .await
        {
            tracing::error!("[warmup] tool cache init failed: {}", e);
            return StepOutcome::Done(
                respond_abort(State(ctx.state.clone()), ctx.ctx.clone(), e.to_string()).await,
            );
        }
        *ctx.state.tools.write().await = ctx.tools.clone();

        let cwd = ctx.cwd.clone();
        let clounar_dir = ctx.state.clounar_dir.clone();
        let index = build_file_index(&cwd, &clounar_dir);
        ctx.state.env.lock().await.file_index = index;

        StepOutcome::Continue
    }
}

fn build_file_index(cwd: &str, clounar_dir: &Path) -> HashMap<String, Vec<String>> {
    let has_gitignore = Path::new(cwd).join(".gitignore").exists();
    let mut builder = WalkBuilder::new(cwd);
    builder.hidden(true).git_ignore(true);
    if !has_gitignore {
        let default_ignore = clounar_dir.join(".default_ignore");
        if default_ignore.exists() {
            builder.add_ignore(&default_ignore);
        }
    }
    let mut index: HashMap<String, Vec<String>> = HashMap::new();
    for entry in builder.build().flatten() {
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            let path = entry.path();
            if let (Ok(rel), Some(name)) = (
                path.strip_prefix(cwd),
                path.file_name().map(|n| n.to_string_lossy().into_owned()),
            ) {
                index
                    .entry(name)
                    .or_default()
                    .push(rel.to_string_lossy().into_owned());
            }
        }
    }
    index
}
