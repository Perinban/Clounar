use anyhow::{anyhow, Result};
use serde_json::{from_str, json, Value};
use uuid::Uuid;

use super::{
    session::{PerplexitySession, ThreadState},
    types::SearchParams,
};
use crate::constants::{
    MAX_RETRIES, RETRY_DELAY_MS, SEARCH_DELAY_MS, SEARCH_FOLLOWUP_SOURCE, SEARCH_PROMPT_SOURCE,
    SEARCH_QUERY_SOURCE_FOLLOWUP, SEARCH_QUERY_SOURCE_HOME, SSE_EVENT_END, SSE_STEP_FINAL, SSE_URL,
    SSE_USAGE_ASK_TEXT,
};

pub async fn search(
    session: &mut PerplexitySession,
    params: &SearchParams<'_>,
    mut on_chunk: impl FnMut(String),
) -> Result<String> {
    let mut _last_err = anyhow!("Unknown error");

    tracing::debug!(
        "[search] query_len={} model={} mode={}",
        params.query.len(),
        params.model,
        params.mode
    );

    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
            tracing::warn!(
                "Attempt {}/{}: refreshing cookies...",
                attempt + 1,
                MAX_RETRIES
            );
            if let Err(e) = session.reconnect().await {
                tracing::error!("Reconnect failed: {}", e);
                _last_err = e;
                continue;
            }
        }

        match do_search(session, params, &mut on_chunk).await {
            Ok(result)
                if result.contains("Sign up and repeat") || result.contains("sign up to") =>
            {
                tracing::warn!(
                    "Search returned auth-wall response on attempt {}/{}",
                    attempt + 1,
                    MAX_RETRIES
                );
                _last_err = anyhow!("Session unauthenticated");
            }
            Ok(result) => return Ok(result),
            Err(e) => {
                tracing::warn!(
                    "Search failed (attempt {}/{}): {}",
                    attempt + 1,
                    MAX_RETRIES,
                    e
                );
                _last_err = e;
            }
        }
    }

    Err(_last_err)
}

async fn do_search(
    session: &mut PerplexitySession,
    params: &SearchParams<'_>,
    on_chunk: &mut impl FnMut(String),
) -> Result<String> {
    let profile = params.search_mode.profile();
    let sources = json!(profile.sources);
    tokio::time::sleep(tokio::time::Duration::from_millis(SEARCH_DELAY_MS)).await;
    let frontend_uuid = Uuid::new_v4().to_string();

    let p = if let Some(ctx_uuid) = params.context_uuid {
        if let Some(thread) = session.thread_states.get(&ctx_uuid) {
            // Follow-up on existing thread
            json!({
                "attachments": [],
                "search_focus": profile.search_focus,
                "frontend_uuid": frontend_uuid,
                "last_backend_uuid": thread.last_backend_uuid,
                "read_write_token": thread.read_write_token,
                "rum_session_id": session.rum_session_id.to_string(),
                "followup_source": SEARCH_FOLLOWUP_SOURCE,
                "query_source": SEARCH_QUERY_SOURCE_FOLLOWUP,
                "prompt_source": SEARCH_PROMPT_SOURCE,
                "mode": params.mode,
                "model_preference": params.model,
                "is_incognito": params.incognito,
                "sources": sources,
                "source": profile.source,
                "use_schematized_api": true,
                "skip_search_enabled": profile.skip_search_enabled,
                "always_search_override": profile.always_search_override,
                "supported_block_use_cases": profile.supported_block_use_cases,
                "supported_features": profile.supported_features,
                "is_related_query": false,
                "expect_search_results": profile.expect_search_results,
            })
        } else {
            // New thread with known context_uuid
            json!({
                "attachments": [],
                "search_focus": profile.search_focus,
                "frontend_uuid": frontend_uuid,
                "frontend_context_uuid": ctx_uuid.to_string(),
                "rum_session_id": session.rum_session_id.to_string(),
                "prompt_source": SEARCH_PROMPT_SOURCE,
                "query_source": SEARCH_QUERY_SOURCE_HOME,
                "mode": params.mode,
                "model_preference": params.model,
                "is_incognito": params.incognito,
                "sources": sources,
                "source": profile.source,
                "use_schematized_api": true,
                "skip_search_enabled": profile.skip_search_enabled,
                "always_search_override": profile.always_search_override,
                "supported_block_use_cases": profile.supported_block_use_cases,
                "supported_features": profile.supported_features,
                "is_related_query": false,
                "expect_search_results": profile.expect_search_results,
            })
        }
    } else {
        // Fresh thread, no context
        json!({
            "attachments": [],
            "search_focus": profile.search_focus,
            "frontend_uuid": frontend_uuid,
            "rum_session_id": session.rum_session_id.to_string(),
            "prompt_source": SEARCH_PROMPT_SOURCE,
            "query_source": SEARCH_QUERY_SOURCE_HOME,
            "mode": params.mode,
            "model_preference": params.model,
            "is_incognito": params.incognito,
            "sources": sources,
            "source": profile.source,
            "use_schematized_api": true,
            "skip_search_enabled": profile.skip_search_enabled,
            "always_search_override": profile.always_search_override,
            "supported_block_use_cases": profile.supported_block_use_cases,
            "supported_features": profile.supported_features,
            "is_related_query": false,
            "expect_search_results": profile.expect_search_results,
        })
    };

    let body = json!({
        "params": p,
        "query_str": params.query,
        "dsl_query": params.query
    });

    let mut resp = session
        .client
        .post(SSE_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .header("x-csrf-token", session.csrf_token.as_str())
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("HTTP {}: {}", status, text));
    }

    let mut full_answer = String::new();
    let mut buf = String::new();
    let mut last_event = String::new();
    let mut last_backend_uuid: Option<String> = None;
    let mut read_write_token: Option<String> = None;

    while let Some(bytes) = resp
        .chunk()
        .await
        .map_err(|e| anyhow!("Stream error: {}", e))?
    {
        buf.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf = buf[pos + 1..].to_string();

            if let Some(stripped) = line.strip_prefix("event:") {
                last_event = stripped.trim().to_string();
                if last_event == SSE_EVENT_END {
                    if let (Some(buuid), Some(rwt), Some(ctx_uuid)) =
                        (last_backend_uuid, read_write_token, params.context_uuid)
                    {
                        session.thread_states.insert(
                            ctx_uuid,
                            ThreadState {
                                last_backend_uuid: buuid,
                                read_write_token: rwt,
                            },
                        );
                    }
                    return Ok(full_answer);
                }
                continue;
            }

            if !line.starts_with("data:") {
                continue;
            }

            let data = line["data:".len()..].trim();

            if last_event == SSE_EVENT_END {
                return Ok(full_answer);
            }

            let msg: Value = match from_str(data) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Capture backend_uuid and read_write_token from any message
            if let Some(uuid) = msg["backend_uuid"].as_str() {
                last_backend_uuid = Some(uuid.to_string());
            }
            if let Some(token) = msg["read_write_token"].as_str() {
                read_write_token = Some(token.to_string());
            }

            let is_final = msg["step_type"].as_str() == Some(SSE_STEP_FINAL)
                || msg["final"].as_bool() == Some(true);

            if let Some(blocks) = msg["blocks"].as_array() {
                for block in blocks {
                    if block["intended_usage"].as_str() != Some(SSE_USAGE_ASK_TEXT) {
                        continue;
                    }
                    if let Some(chunks) = block["markdown_block"]["chunks"].as_array() {
                        for chunk in chunks {
                            if let Some(s) = chunk.as_str() {
                                on_chunk(s.to_string());
                                full_answer.push_str(s);
                            }
                        }
                    }
                }
            }

            if is_final {
                if let (Some(buuid), Some(rwt), Some(ctx_uuid)) =
                    (last_backend_uuid, read_write_token, params.context_uuid)
                {
                    session.thread_states.insert(
                        ctx_uuid,
                        ThreadState {
                            last_backend_uuid: buuid,
                            read_write_token: rwt,
                        },
                    );
                }
                return Ok(full_answer);
            }
        }
    }

    if full_answer.is_empty() {
        tracing::warn!(
            "[search] stream closed without content: model={} mode={}",
            params.model,
            params.mode
        );
        return Err(anyhow!(
            "Stream closed without content or end_of_stream event"
        ));
    }
    Ok(full_answer)
}
