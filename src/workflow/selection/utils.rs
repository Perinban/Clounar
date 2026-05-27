use std::sync::Arc;

use crate::{
    perplexity::{search::search, types::SearchParams, SearchMode},
    prompts::{PromptContext, PromptKind},
    workflow::selection::types::SelectionContext,
};

pub fn build_prompt(ctx: &SelectionContext, kind: PromptKind) -> String {
    PromptContext {
        tool: None,
        compressed: None,
        user_query: &ctx.user_query,
        env: None,
        artifact_refs: &[],
        args_hint: None,
        prompts: &ctx.state.config.prompts,
    }
    .build(kind)
}

pub async fn run_search(
    session: Arc<tokio::sync::Mutex<crate::perplexity::PerplexitySession>>,
    mode: String,
    model: String,
    incognito: bool,
    prompt: String,
) -> anyhow::Result<String> {
    let mut guard = session.lock().await;
    search(
        &mut guard,
        &SearchParams {
            query: &prompt,
            mode: &mode,
            model: &model,
            incognito,
            search_mode: &SearchMode::Strict,
            context_uuid: None,
        },
        |_| {},
    )
    .await
}
