use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::warn;

use crate::domain::entity::CarrierInfo;
use crate::domain::error::DomainError;
use crate::domain::port::CarrierClient;
use crate::infrastructure::circuit_breaker::CircuitBreaker;

pub struct ParallaxClient {
    client: Client,
    base_url: String,
    cache: Arc<RwLock<Option<CachedCarriers>>>,
    circuit_breaker: CircuitBreaker,
}

struct CachedCarriers {
    carriers: Vec<CarrierInfo>,
    fetched_at: std::time::Instant,
}

const CACHE_TTL: Duration = Duration::from_secs(30);

impl ParallaxClient {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(5)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(15))
            .build()
            .expect("failed to build parallax http client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            cache: Arc::new(RwLock::new(None)),
            circuit_breaker: CircuitBreaker::new("parallax", 5, Duration::from_secs(30)),
        }
    }

    async fn fetch_carriers(&self) -> Result<Vec<CarrierInfo>, DomainError> {
        if !self.circuit_breaker.is_call_permitted() {
            return Err(DomainError::Upstream(format!(
                "circuit breaker '{}' is open — parallax appears down",
                self.circuit_breaker.name()
            )));
        }

        let resp = self
            .client
            .get(format!("{}/api/carriers", self.base_url))
            .send()
            .await
            .map_err(|e| {
                self.circuit_breaker.record_failure();
                DomainError::Upstream(format!("parallax: {e}"))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            if status.is_server_error() {
                self.circuit_breaker.record_failure();
            }
            return Err(DomainError::Upstream(format!("parallax returned {status}")));
        }

        self.circuit_breaker.record_success();

        resp.json()
            .await
            .map_err(|e| DomainError::Upstream(format!("parallax decode: {e}")))
    }
}

#[async_trait]
impl CarrierClient for ParallaxClient {
    async fn list_carriers(&self) -> Result<Vec<CarrierInfo>, DomainError> {
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.as_ref() {
                if cached.fetched_at.elapsed() < CACHE_TTL {
                    return Ok(cached.carriers.clone());
                }
            }
        }

        match self.fetch_carriers().await {
            Ok(carriers) => {
                let mut cache = self.cache.write().await;
                *cache = Some(CachedCarriers {
                    carriers: carriers.clone(),
                    fetched_at: std::time::Instant::now(),
                });
                Ok(carriers)
            }
            Err(e) => {
                warn!("failed to refresh carriers, using stale cache: {e}");
                let cache = self.cache.read().await;
                if let Some(cached) = cache.as_ref() {
                    return Ok(cached.carriers.clone());
                }
                Err(e)
            }
        }
    }
}
