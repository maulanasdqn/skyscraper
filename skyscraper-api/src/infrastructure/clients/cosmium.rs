use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::warn;

use super::cosmium_types::{CosmiumScrapeRequest, CosmiumScrapeResponse};
use crate::domain::entity::{ScrapeJobRequest, ScrapeJobResult};
use crate::domain::error::DomainError;
use crate::domain::port::ScrapeClient;
use crate::infrastructure::circuit_breaker::CircuitBreaker;

pub struct CosmiumClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    profile_cache: Arc<RwLock<Option<CachedProfiles>>>,
    circuit_breaker: CircuitBreaker,
}

struct CachedProfiles {
    profiles: Vec<String>,
    fetched_at: std::time::Instant,
}

const PROFILE_CACHE_TTL: Duration = Duration::from_mins(1);

impl CosmiumClient {
    pub fn new(base_url: &str, api_key: Option<String>) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(15))
            .build()
            .expect("failed to build cosmium http client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            profile_cache: Arc::new(RwLock::new(None)),
            circuit_breaker: CircuitBreaker::new("cosmium", 5, Duration::from_secs(30)),
        }
    }

    fn request_builder(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(key) => req.header("X-Api-Key", key),
            None => req,
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
        if !self.circuit_breaker.is_call_permitted() {
            return Err(DomainError::Upstream(format!(
                "circuit breaker '{}' is open — cosmium appears down",
                self.circuit_breaker.name()
            )));
        }

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
            .request_builder(
                self.client
                    .post(format!("{}/api/v1/scrape", self.base_url))
                    .json(&body),
            )
            .send()
            .await
            .map_err(|e| {
                self.circuit_breaker.record_failure();
                DomainError::Upstream(format!("cosmium: {e}"))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if status.is_server_error() {
                self.circuit_breaker.record_failure();
            }
            return Err(DomainError::Upstream(format!(
                "cosmium returned {status}: {text}"
            )));
        }

        let result: CosmiumScrapeResponse = resp
            .json()
            .await
            .map_err(|e| DomainError::Upstream(format!("cosmium decode: {e}")))?;

        if result.blocked {
            self.circuit_breaker.record_success();
            return Err(DomainError::ScrapeBlocked);
        }

        self.circuit_breaker.record_success();

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
            .request_builder(
                self.client.get(format!("{}/api/v1/profiles", self.base_url)),
            )
            .send()
            .await
            .map_err(|e| DomainError::Upstream(format!("cosmium profiles: {e}")))?;

        if !resp.status().is_success() {
            return Err(DomainError::Upstream(format!(
                "cosmium profiles returned {}",
                resp.status()
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DomainError::Upstream(format!("cosmium profiles decode: {e}")))?;

        if let Some(arr) = body.get("profiles").and_then(|v| v.as_array()) {
            Ok(arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        } else if let Ok(list) = serde_json::from_value::<Vec<String>>(body) {
            Ok(list)
        } else {
            Ok(vec![])
        }
    }
}
