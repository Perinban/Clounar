use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use crate::{
    anthropic::serialize_segments,
    constants::TOOL_RESULT_SNIPPET_LEN,
    state::TaskEntry,
    workflow::result::{
        traits::{Stage, StageOutcome},
        types::PipelineContext,
    },
};

pub struct Record;

impl Stage for Record {
    async fn run(&self, ctx: &PipelineContext) -> StageOutcome {
        let query_str = serialize_segments(&ctx.user_query);
        let mut h = DefaultHasher::new();
        query_str.hash(&mut h);
        let hash = format!("{:x}", h.finish());
        let snippet =
            ctx.tool_result[..ctx.tool_result.len().min(TOOL_RESULT_SNIPPET_LEN)].to_string();
        ctx.state.task_history.lock().await.push(TaskEntry {
            hash,
            user_query: query_str,
            tool_name: ctx.tool_name.clone(),
            tool_result: snippet,
        });
        StageOutcome::Continue
    }
}
