use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use async_trait::async_trait;

use crate::domain::entity::{ProxyStrategy, ScrapeJobRequest};
use crate::domain::error::DomainError;
use crate::domain::port::{CarrierClient, ProxySelection};

const FAILURE_WINDOW: Duration = Duration::from_mins(30);

pub struct ProxySelector {
    carrier_client: Arc<dyn CarrierClient>,
    proxy_host: String,
    proxy_port: u16,
    proxy_pass: String,
    failure_tracker: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
}

impl ProxySelector {
    pub fn new(
        carrier_client: Arc<dyn CarrierClient>,
        proxy_host: String,
        proxy_port: u16,
        proxy_pass: String,
    ) -> Self {
        Self {
            carrier_client,
            proxy_host,
            proxy_port,
            proxy_pass,
            failure_tracker: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn select(&self, request: &ScrapeJobRequest) -> Result<Option<String>, DomainError> {
        match &request.proxy_strategy {
            ProxyStrategy::None => Ok(None),
            ProxyStrategy::Carrier { name } => {
                let carriers = self.carrier_client.list_carriers().await?;
                let carrier = carriers
                    .iter()
                    .find(|c| c.name.eq_ignore_ascii_case(name))
                    .ok_or_else(|| DomainError::NotFound(format!("carrier '{name}' not found")))?;
                if !carrier.online {
                    return Err(DomainError::Upstream(format!(
                        "carrier '{name}' is offline"
                    )));
                }
                Ok(Some(self.build_proxy_url(&carrier.name)))
            }
            ProxyStrategy::Auto => self.select_auto(request).await,
        }
    }

    async fn select_auto(&self, request: &ScrapeJobRequest) -> Result<Option<String>, DomainError> {
        let carriers = self
            .carrier_client
            .list_carriers()
            .await
            .unwrap_or_default();
        let online: Vec<_> = carriers.iter().filter(|c| c.online).collect();
        if online.is_empty() {
            return Ok(None);
        }

        let geo_filtered: Vec<_> = if let Some(ref geo) = request.geo {
            let filtered: Vec<_> = online
                .iter()
                .filter(|c| c.country.eq_ignore_ascii_case(geo))
                .collect();
            if filtered.is_empty() {
                online.iter().collect()
            } else {
                filtered
            }
        } else {
            online.iter().collect()
        };

        let now = Instant::now();
        let mut failures = self.failure_tracker.write().await;
        failures.retain(|_, ts| {
            ts.retain(|t| now.duration_since(*t) < FAILURE_WINDOW);
            !ts.is_empty()
        });

        let weights: Vec<f64> = geo_filtered
            .iter()
            .map(|c| {
                let f = failures.get(&c.name).map_or(0, Vec::len);
                1.0 / (1.0 + f as f64)
            })
            .collect();

        let total: f64 = weights.iter().sum();
        let pick = rand::random_range(0.0..total);
        let mut cumulative = 0.0;
        for (i, w) in weights.iter().enumerate() {
            cumulative += w;
            if pick < cumulative {
                return Ok(Some(self.build_proxy_url(&geo_filtered[i].name)));
            }
        }

        Ok(Some(self.build_proxy_url(&geo_filtered[0].name)))
    }

    fn build_proxy_url(&self, carrier_name: &str) -> String {
        let session_id = Uuid::new_v4();
        format!(
            "socks5://{carrier_name}-{session_id}:{}@{}:{}",
            self.proxy_pass, self.proxy_host, self.proxy_port
        )
    }

    pub async fn report_failure(&self, proxy_url: &str) {
        if let Some(carrier) = proxy_url
            .strip_prefix("socks5://")
            .and_then(|s| s.split('-').next())
        {
            let mut map = self.failure_tracker.write().await;
            map.entry(carrier.to_string())
                .or_default()
                .push(Instant::now());
        }
    }
}

#[async_trait]
impl ProxySelection for ProxySelector {
    async fn select_proxy(
        &self,
        request: &ScrapeJobRequest,
    ) -> Result<Option<String>, DomainError> {
        self.select(request).await
    }

    async fn report_proxy_failure(&self, proxy_url: &str) {
        self.report_failure(proxy_url).await;
    }
}
