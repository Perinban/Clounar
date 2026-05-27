mod anthropic;
mod artifact;
mod bridge;
mod config;
mod constants;
mod perplexity;
mod planner;
mod prompts;
mod server;
mod state;
mod validation;
mod workflow;

use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use std::{env, fs, io, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::artifact::ArtifactRegistry;
use crate::{
    config::Config,
    constants::{
        CLAUDE_DIR, CLAUDE_SETTINGS, CLAUDE_SETTINGS_FILE, CLOUNAR_DIR, CONFIG_FILE,
        DEFAULT_CONFIG, DEFAULT_IGNORE, DEFAULT_IGNORE_FILE, ROUTE_MESSAGES, ROUTE_MODELS,
    },
    perplexity::session::{extract_cookies, PerplexitySession},
    state::{AppState, EnvironmentContext},
    workflow::state::WorkflowState,
};

#[tokio::main]
async fn main() -> Result<()> {
    let home_dir = dirs::home_dir().context("Cannot determine home directory")?;
    let clounar_dir = home_dir.join(CLOUNAR_DIR);
    fs::create_dir_all(&clounar_dir)?;

    let config_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| clounar_dir.join(CONFIG_FILE));

    if !config_path.exists() {
        fs::write(&config_path, DEFAULT_CONFIG).context("Failed to write default config.toml")?;
        tracing::info!("Created default config at {}", config_path.display());
    }

    let default_ignore_path = clounar_dir.join(DEFAULT_IGNORE_FILE);
    if !default_ignore_path.exists() {
        fs::write(&default_ignore_path, DEFAULT_IGNORE)
            .context("Failed to write .default_ignore")?;
    }

    let config = Config::load(&config_path)?;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(format!("clounar={}", config.server.log_level).parse()?),
        )
        .init();

    let claude_settings_path = Some(home_dir.join(CLAUDE_DIR).join(CLAUDE_SETTINGS_FILE));
    if let Some(ref path) = claude_settings_path {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            match fs::write(path, CLAUDE_SETTINGS) {
                Ok(_) => tracing::info!("Created ~/.claude/settings.json"),
                Err(e) => tracing::warn!("Failed to create ~/.claude/settings.json: {}", e),
            }
        }
    }

    tracing::info!("Extracting cookies from browser...");
    let tokens = extract_cookies().await?;

    tracing::info!("Connecting to Perplexity...");
    let session = PerplexitySession::connect(&tokens, config.perplexity.incognito).await?;
    tracing::info!("Connected");

    match session.fetch_models().await {
        Ok(config_json) => {
            let tier = &session.subscription_tier;
            tracing::info!("Supported models (tier={:?}):", tier);
            if let Some(entries) = config_json["config"].as_array() {
                for entry in entries {
                    let entry_tier = entry["subscription_tier"].as_str().unwrap_or("free");
                    if *tier < entry_tier.parse().unwrap_or_default() {
                        continue;
                    }
                    for key in &["non_reasoning_model", "reasoning_model"] {
                        if let Some(model_id) = entry[key].as_str() {
                            let mode = config_json["models"][model_id]["mode"]
                                .as_str()
                                .unwrap_or("");
                            if mode != "search" {
                                continue;
                            }
                            tracing::info!(
                                "  {} — {}",
                                model_id,
                                entry["label"].as_str().unwrap_or("")
                            );
                        }
                    }
                }
            }
        }
        Err(e) => tracing::warn!("[models] failed to fetch: {}", e),
    }

    let state = AppState {
        config: config.clone(),
        clounar_dir,
        session: Arc::new(Mutex::new(session)),
        workflow: Arc::new(Mutex::new(WorkflowState::default())),
        tool_cache: Arc::new(RwLock::new(None)),
        tools: Arc::new(RwLock::new(vec![])),
        env: Arc::new(Mutex::new(EnvironmentContext::default())),
        task_history: Arc::new(Mutex::new(vec![])),
        artifact_registry: Arc::new(RwLock::new(ArtifactRegistry::default())),
    };

    let app = Router::new()
        .route(ROUTE_MESSAGES, post(server::messages::messages))
        .route(ROUTE_MODELS, get(perplexity::models::list))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(_) => {
            let fallback = tokio::net::TcpListener::bind(format!("{}:0", config.server.host))
                .await
                .context("Failed to bind to any port")?;
            let port = fallback.local_addr()?.port();
            tracing::warn!(
                "Port {} is busy. OS assigned port {}. Update ~/.claude/settings.json? [y/N]: ",
                config.server.port,
                port
            );
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if input.trim().eq_ignore_ascii_case("y") {
                if let Some(ref path) = claude_settings_path {
                    if let Ok(raw) = fs::read_to_string(path) {
                        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&raw) {
                            json["env"]["ANTHROPIC_BASE_URL"] = serde_json::json!(format!(
                                "http://{}:{}",
                                config.server.host, port
                            ));
                            if let Ok(updated) = serde_json::to_string_pretty(&json) {
                                let _ = fs::write(path, updated);
                                tracing::info!(
                                    "Updated ~/.claude/settings.json with port {}",
                                    port
                                );
                            }
                        }
                    }
                }
            } else {
                anyhow::bail!("Start aborted. Set a free port in config.toml");
            }
            fallback
        }
    };
    let bound_addr = listener.local_addr()?;
    tracing::info!("Listening on http://{}", bound_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
