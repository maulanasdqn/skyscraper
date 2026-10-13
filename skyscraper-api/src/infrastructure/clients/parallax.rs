use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

use crate::domain::entity::CarrierInfo;
use crate::domain::error::DomainError;
use crate::domain::port::CarrierClient;

pub struct ParallaxClient {
    client: Client,
    base_url: String,
    cache: Arc<RwLock<Option<CachedCarriers>>>,
}

struct CachedCarriers {
    carriers: Vec<CarrierInfo>,
    fetched_at: std::time::Instant,
}

const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(30);

impl ParallaxClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            cache: Arc::new(RwLock::new(None)),
        }
    }

    async fn fetch_carriers(&self) -> Result<Vec<CarrierInfo>, DomainError> {
        let resp = self
            .client
            .get(format!("{}/api/carriers", self.base_url))
            .send()
            .await
            .map_err(|e| DomainError::Upstream(format!("parallax: {e}")))?;

        if !resp.status().is_success() {
            return Err(DomainError::Upstream(format!(
                "parallax returned {}",
                resp.status()
            )));
        }

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
