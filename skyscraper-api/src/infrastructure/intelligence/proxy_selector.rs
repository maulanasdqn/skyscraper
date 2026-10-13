use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::entity::{ProxyStrategy, ScrapeJobRequest};
use crate::domain::error::DomainError;
use crate::domain::port::CarrierClient;

pub struct ProxySelector {
    carrier_client: Arc<dyn CarrierClient>,
    proxy_host: String,
    proxy_port: u16,
    proxy_pass: String,
    failure_tracker: Arc<RwLock<HashMap<String, u32>>>,
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

    pub async fn select(
        &self,
        request: &ScrapeJobRequest,
    ) -> Result<Option<String>, DomainError> {
        match &request.proxy_strategy {
            ProxyStrategy::None => Ok(None),
            ProxyStrategy::Carrier { name } => {
                let carriers = self.carrier_client.list_carriers().await?;
                let carrier = carriers
                    .iter()
                    .find(|c| c.name.eq_ignore_ascii_case(name))
                    .ok_or_else(|| {
                        DomainError::NotFound(format!("carrier '{name}' not found"))
                    })?;
                if !carrier.online {
                    return Err(DomainError::Upstream(format!(
                        "carrier '{name}' is offline"
                    )));
                }
                Ok(Some(self.build_proxy_url(&carrier.name)))
            }
            ProxyStrategy::Auto => {
                let carriers = self.carrier_client.list_carriers().await?;
                let online: Vec<_> = carriers.iter().filter(|c| c.online).collect();
                if online.is_empty() {
                    return Err(DomainError::Upstream(
                        "no online carriers available".into(),
                    ));
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

                let failures = self.failure_tracker.read().await;
                let mut best = geo_filtered[0];
                let mut best_failures = failures.get(&best.name).copied().unwrap_or(0);

                for carrier in &geo_filtered[1..] {
                    let f = failures.get(&carrier.name).copied().unwrap_or(0);
                    if f < best_failures {
                        best = carrier;
                        best_failures = f;
                    }
                }

                Ok(Some(self.build_proxy_url(&best.name)))
            }
        }
    }

    fn build_proxy_url(&self, carrier_name: &str) -> String {
        let session_id = Uuid::new_v4();
        format!(
            "socks5://{carrier_name}-{session_id}:{}@{}:{}",
            self.proxy_pass, self.proxy_host, self.proxy_port
        )
    }

    pub fn report_failure(&self, proxy_url: &str) {
        if let Some(carrier) = proxy_url
            .strip_prefix("socks5://")
            .and_then(|s| s.split('-').next())
        {
            let tracker = self.failure_tracker.clone();
            let carrier = carrier.to_string();
            tokio::spawn(async move {
                let mut map = tracker.write().await;
                *map.entry(carrier).or_insert(0) += 1;
            });
        }
    }
}
