use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use crate::state::AppState;

pub async fn list(State(state): State<AppState>) -> impl IntoResponse {
    let session = state.session.lock().await;

    match session.fetch_models().await {
        Ok(config_json) => {
            let models_obj = &config_json["models"];
            let mut data: Vec<serde_json::Value> = Vec::new();

            if let Some(entries) = config_json["config"].as_array() {
                for entry in entries {
                    let entry_tier = entry["subscription_tier"].as_str().unwrap_or("free");
                    if session.subscription_tier < entry_tier.parse().unwrap_or_default() {
                        continue;
                    }
                    for key in &["non_reasoning_model", "reasoning_model"] {
                        if let Some(model_id) = entry[key].as_str() {
                            let v = &models_obj[model_id];
                            if v["mode"].as_str().unwrap_or("") != "search" {
                                continue;
                            }
                            data.push(json!({
                                "id": model_id,
                                "object": "model",
                                "owned_by": v["provider"].as_str().unwrap_or("perplexity").to_lowercase(),
                                "description": entry["description"],
                            }));
                        }
                    }
                }
            }

            tracing::debug!(
                "[models] returning {} models for tier={:?}",
                data.len(),
                session.subscription_tier
            );
            Json(json!({ "object": "list", "data": data }))
        }
        Err(e) => {
            tracing::error!("[models] fetch failed: {}", e);
            Json(json!({ "object": "list", "data": [] }))
        }
    }
}
