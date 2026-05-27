use std::collections::HashMap;

use anyhow::{Context, Result};
use rquest::header::{HeaderValue, COOKIE};
use rquest_util::Emulation;
use serde::Deserialize;
use tokio::process::Command;
use uuid::Uuid;

use crate::constants::{
    APP_CLIENT, AUTH_SESSION_URL, BASE_URL, EXTRACT_COOKIES_PY, MODELS_CONFIG_SCHEMA,
    MODELS_CONFIG_SOURCE, MODELS_CONFIG_URL, MODELS_REQUEST_REASON, PERPLEXITY_VERSION, PYTHON_BIN,
};

#[derive(Deserialize)]
pub struct CookieTokens {
    pub session_token: String,
    pub cf_clearance: String,
    pub csrf_token: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum SubscriptionTier {
    #[default]
    Free,
    Pro,
    Max,
}

pub async fn extract_cookies() -> Result<CookieTokens> {
    let output = Command::new(PYTHON_BIN)
        .arg("-c")
        .arg(EXTRACT_COOKIES_PY)
        .output()
        .await
        .context("Failed to run extract_cookies.py")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Cookie extraction failed: {}", err);
    }

    serde_json::from_slice(&output.stdout).context("Failed to parse cookie JSON")
}

pub struct ThreadState {
    pub last_backend_uuid: String,
    pub read_write_token: String,
}

pub struct PerplexitySession {
    pub client: rquest::Client,
    pub cookie: String,
    pub csrf_token: String,
    pub subscription_tier: SubscriptionTier,
    pub incognito: bool,
    pub context_uuids: HashMap<(String, String), Uuid>,
    pub rum_session_id: Uuid,
    pub thread_states: HashMap<Uuid, ThreadState>,
}

impl PerplexitySession {
    pub async fn connect(tokens: &CookieTokens, incognito: bool) -> Result<Self> {
        let cookie = make_cookie(&tokens.session_token, &tokens.cf_clearance, incognito);
        let client = build_client(&cookie)?;

        let resp = client
            .get(AUTH_SESSION_URL)
            .header(
                COOKIE,
                HeaderValue::from_str(&cookie).context("Invalid cookie header")?,
            )
            .send()
            .await
            .context("Failed to reach auth/session")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "Auth session failed ({}): {}",
                status,
                &body[..body.len().min(200)]
            );
        }

        let session_json: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse auth session JSON")?;
        let subscription_tier = session_json["user"]["subscription_tier"]
            .as_str()
            .unwrap_or("")
            .parse()
            .unwrap_or_default();

        tracing::info!("Auth session OK ({}) tier={:?}", status, subscription_tier);

        Ok(Self {
            client,
            cookie,
            csrf_token: tokens.csrf_token.clone(),
            subscription_tier,
            incognito,
            context_uuids: HashMap::new(),
            rum_session_id: Uuid::new_v4(),
            thread_states: HashMap::new(),
        })
    }

    pub async fn fetch_models(&self) -> Result<serde_json::Value> {
        tracing::debug!("[models] fetching from /rest/models/config...");

        let resp = self
            .client
            .get(MODELS_CONFIG_URL)
            .query(&[
                ("config_schema", MODELS_CONFIG_SCHEMA),
                ("version", PERPLEXITY_VERSION),
                ("source", MODELS_CONFIG_SOURCE),
            ])
            .header("X-App-Apiclient", APP_CLIENT)
            .header("X-App-Apiversion", PERPLEXITY_VERSION)
            .header("X-Perplexity-Request-Reason", MODELS_REQUEST_REASON)
            .send()
            .await
            .context("Failed to fetch models config")?;

        let status = resp.status();
        tracing::debug!("[models] response status: {}", status);

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!("[models] error body: {}", &body[..body.len().min(500)]);
            anyhow::bail!(
                "Models config failed ({}): {}",
                status,
                &body[..body.len().min(200)]
            );
        }

        let json = resp
            .json::<serde_json::Value>()
            .await
            .context("Failed to parse models config JSON")?;
        tracing::debug!(
            "[models] got {} top-level keys",
            json.as_object().map(|o| o.len()).unwrap_or(0)
        );
        Ok(json)
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        let tokens = extract_cookies().await?;
        self.cookie = make_cookie(&tokens.session_token, &tokens.cf_clearance, self.incognito);
        self.csrf_token = tokens.csrf_token;
        self.client = build_client(&self.cookie)?;
        self.thread_states.clear();
        self.context_uuids.clear();
        tracing::info!("Session refreshed");
        Ok(())
    }
}

fn make_cookie(session_token: &str, cf_clearance: &str, incognito: bool) -> String {
    format!(
        "__Secure-next-auth.session-token={}; cf_clearance={}; pplx.is-incognito={}",
        session_token, cf_clearance, incognito
    )
}

fn build_client(cookie: &str) -> Result<rquest::Client> {
    let mut headers = rquest::header::HeaderMap::new();
    headers.insert(
        COOKIE,
        HeaderValue::from_str(cookie).context("Invalid cookie")?,
    );
    headers.insert("Origin", HeaderValue::from_static(BASE_URL));
    headers.insert("Referer", HeaderValue::from_static(BASE_URL));

    rquest::Client::builder()
        .default_headers(headers)
        .emulation(Emulation::Chrome120)
        .cookie_store(true)
        .build()
        .context("Failed to build HTTP client")
}
