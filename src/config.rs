use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

use crate::prompts::PromptsConfig;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub perplexity: PerplexityConfig,
    pub server: ServerConfig,
    pub prompts: PromptsConfig,
}

#[derive(Deserialize, Clone)]
pub struct PerplexityConfig {
    pub default_mode: String,
    pub default_model: String,
    #[serde(default = "default_true")]
    pub incognito: bool,
}

fn default_true() -> bool {
    true
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Cannot read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&contents).context("Invalid config.toml")?;
        config
            .prompts
            .validate()
            .context("Invalid [prompts] in config.toml")?;
        Ok(config)
    }
}
