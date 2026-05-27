use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};
use uuid::Uuid;

use crate::{
    anthropic::serialize_segments,
    constants::{CTX_KEY_PLANNER, HISTORY_LOOKUP_LIMIT},
    prompts::PromptKind,
    workflow::selection::{
        traits::{Step, StepOutcome},
        types::SelectionContext,
        utils::{build_prompt, run_search},
    },
};

pub struct Lookup;

impl Step for Lookup {
    async fn run(&self, ctx: &mut SelectionContext) -> StepOutcome {
        let hash = {
            let query_str = serialize_segments(&ctx.user_query);
            let mut h = DefaultHasher::new();
            query_str.hash(&mut h);
            let query_hash = format!("{:x}", h.finish());

            let history = ctx.state.task_history.lock().await;
            if history.is_empty() {
                query_hash
            } else {
                let candidates = history
                    .iter()
                    .rev()
                    .take(HISTORY_LOOKUP_LIMIT)
                    .map(|e| format!("hash={} query={}", e.hash, e.user_query))
                    .collect::<Vec<_>>()
                    .join("\n");
                drop(history);
                let prompt = build_prompt(
                    ctx,
                    PromptKind::HashSelect {
                        candidates: &candidates,
                    },
                );
                let session = Arc::clone(&ctx.state.session);
                let mode = ctx.ctx.mode.clone();
                let model = ctx.ctx.model.clone();
                let incognito = ctx.ctx.incognito;
                match run_search(session, mode, model, incognito, prompt).await {
                    Ok(h) => {
                        let h = h.trim().to_string();
                        tracing::debug!("[selection] matched hash={:?}", h);
                        if h.is_empty() {
                            query_hash
                        } else {
                            h
                        }
                    }
                    Err(_) => query_hash,
                }
            }
        };

        {
            let mut session = ctx.state.session.lock().await;
            let key = (hash.clone(), CTX_KEY_PLANNER.to_string());
            session
                .context_uuids
                .entry(key)
                .or_insert_with(Uuid::new_v4);
        }

        ctx.query_hash = Some(hash);
        StepOutcome::Continue
    }
}
