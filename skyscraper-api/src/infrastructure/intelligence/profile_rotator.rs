use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entity::{ProfileStrategy, ScrapeJobRequest};
use crate::domain::error::DomainError;
use crate::domain::port::{ProfileSelection, ScrapeClient};

const BLOCK_TTL: Duration = Duration::from_secs(30 * 60);

pub struct ProfileRotator {
    scrape_client: Arc<dyn ScrapeClient>,
    counter: AtomicUsize,
    blocked_profiles: Arc<RwLock<HashMap<String, Instant>>>,
}

impl ProfileRotator {
    pub fn new(scrape_client: Arc<dyn ScrapeClient>) -> Self {
        Self {
            scrape_client,
            counter: AtomicUsize::new(0),
            blocked_profiles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn select(&self, request: &ScrapeJobRequest) -> Result<String, DomainError> {
        match &request.profile_strategy {
            ProfileStrategy::Named { name } => Ok(name.clone()),
            ProfileStrategy::Random => {
                let profiles = self.scrape_client.list_profiles().await?;
                if profiles.is_empty() {
                    return Err(DomainError::Upstream("no profiles available".into()));
                }
                let idx = rand::random_range(0..profiles.len());
                Ok(profiles[idx].clone())
            }
            ProfileStrategy::Auto => self.select_auto().await,
        }
    }

    async fn select_auto(&self) -> Result<String, DomainError> {
        let profiles = self.scrape_client.list_profiles().await?;
        if profiles.is_empty() {
            return Err(DomainError::Upstream("no profiles available".into()));
        }

        let now = Instant::now();
        let mut blocked = self.blocked_profiles.write().await;
        blocked.retain(|_, ts| now.duration_since(*ts) < BLOCK_TTL);

        let available: Vec<_> = profiles.iter().filter(|p| !blocked.contains_key(*p)).collect();

        let pool = if available.is_empty() { &profiles } else {
            let idx = self.counter.fetch_add(1, Ordering::Relaxed) % available.len();
            return Ok(available[idx].clone());
        };

        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % pool.len();
        Ok(pool[idx].clone())
    }

    pub async fn report_failure(&self, profile: &str) {
        let mut map = self.blocked_profiles.write().await;
        map.insert(profile.to_string(), Instant::now());
    }
}

#[async_trait]
impl ProfileSelection for ProfileRotator {
    async fn select_profile(&self, request: &ScrapeJobRequest) -> Result<String, DomainError> {
        self.select(request).await
    }

    async fn report_profile_failure(&self, profile: &str) {
        self.report_failure(profile).await;
    }
}
