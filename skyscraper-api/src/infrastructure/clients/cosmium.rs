use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

use crate::domain::entity::{ScrapeJobRequest, ScrapeJobResult, WorkflowStep};
use crate::domain::error::DomainError;
use crate::domain::port::ScrapeClient;

pub struct CosmiumClient {
    client: Client,
    base_url: String,
    profile_cache: Arc<RwLock<Option<CachedProfiles>>>,
}

struct CachedProfiles {
    profiles: Vec<String>,
    fetched_at: std::time::Instant,
}

const PROFILE_CACHE_TTL: std::time::Duration = std::time::Duration::from_mins(1);

#[derive(Serialize)]
struct CosmiumScrapeRequest<'a> {
    url: &'a str,
    profile: &'a str,
    screenshot: bool,
    headful: bool,
    wait_ms: u32,
    extract: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    script: &'a Option<String>,
    #[serde(skip_serializing_if = "<[WorkflowStep]>::is_empty")]
    workflow: &'a [WorkflowStep],
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy: Option<&'a str>,
    include_html: bool,
    retries: u32,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    geo_sync: bool,
}

#[derive(Deserialize)]
struct CosmiumScrapeResponse {
    final_url: String,
    #[serde(default)]
    status: i32,
    #[serde(default)]
    html_length: usize,
    html: Option<String>,
    #[serde(default)]
    extracted: serde_json::Value,
    screenshot_base64: Option<String>,
    #[serde(default)]
    blocked: bool,
    proxy_used: Option<String>,
    #[serde(default)]
    profile_used: String,
    #[serde(default)]
    elapsed_ms: u64,
    #[serde(default)]
    attempts: i32,
}

impl CosmiumClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            profile_cache: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait]
impl ScrapeClient for CosmiumClient {
    async fn scrape(
        &self,
        url: &str,
        profile: &str,
        request: &ScrapeJobRequest,
        proxy_url: Option<&str>,
    ) -> Result<ScrapeJobResult, DomainError> {
        let body = CosmiumScrapeRequest {
            url,
            profile,
            screenshot: request.screenshot,
            headful: request.headful,
            wait_ms: request.wait_ms,
            extract: &request.extract,
            script: &request.script,
            workflow: &request.workflow,
            proxy: proxy_url,
            include_html: request.include_html,
            retries: 0,
            geo_sync: proxy_url.is_some(),
        };

        let resp = self
            .client
            .post(format!("{}/api/scrape", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::Upstream(format!("cosmium: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(DomainError::Upstream(format!(
                "cosmium returned {status}: {text}"
            )));
        }

        let result: CosmiumScrapeResponse = resp
            .json()
            .await
            .map_err(|e| DomainError::Upstream(format!("cosmium decode: {e}")))?;

        if result.blocked {
            return Err(DomainError::ScrapeBlocked);
        }

        Ok(ScrapeJobResult {
            final_url: result.final_url,
            http_status: result.status,
            html_length: result.html_length,
            html: result.html,
            extracted: result.extracted,
            screenshot_base64: result.screenshot_base64,
            blocked: result.blocked,
            proxy_used: result.proxy_used,
            profile_used: result.profile_used,
            elapsed_ms: result.elapsed_ms,
            attempts: result.attempts,
        })
    }

    async fn list_profiles(&self) -> Result<Vec<String>, DomainError> {
        {
            let cache = self.profile_cache.read().await;
            if let Some(cached) = cache.as_ref() {
                if cached.fetched_at.elapsed() < PROFILE_CACHE_TTL {
                    return Ok(cached.profiles.clone());
                }
            }
        }

        match self.fetch_profiles().await {
            Ok(profiles) => {
                let mut cache = self.profile_cache.write().await;
                *cache = Some(CachedProfiles {
                    profiles: profiles.clone(),
                    fetched_at: std::time::Instant::now(),
                });
                Ok(profiles)
            }
            Err(e) => {
                warn!("failed to refresh profiles, using stale cache: {e}");
                let cache = self.profile_cache.read().await;
                if let Some(cached) = cache.as_ref() {
                    return Ok(cached.profiles.clone());
                }
                Err(e)
            }
        }
    }
}

impl CosmiumClient {
    async fn fetch_profiles(&self) -> Result<Vec<String>, DomainError> {
        let resp = self
            .client
            .get(format!("{}/api/profiles", self.base_url))
            .send()
            .await
            .map_err(|e| DomainError::Upstream(format!("cosmium profiles: {e}")))?;

        if !resp.status().is_success() {
            return Err(DomainError::Upstream(format!(
                "cosmium profiles returned {}",
                resp.status()
            )));
        }

        resp.json()
            .await
            .map_err(|e| DomainError::Upstream(format!("cosmium profiles decode: {e}")))
    }
}
