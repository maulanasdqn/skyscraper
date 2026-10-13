use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::entity::{ProfileStrategy, ScrapeJobRequest};
use crate::domain::error::DomainError;
use crate::domain::port::ScrapeClient;

pub struct ProfileRotator {
    scrape_client: Arc<dyn ScrapeClient>,
    counter: AtomicUsize,
    blocked_profiles: Arc<RwLock<HashSet<String>>>,
}

impl ProfileRotator {
    pub fn new(scrape_client: Arc<dyn ScrapeClient>) -> Self {
        Self {
            scrape_client,
            counter: AtomicUsize::new(0),
            blocked_profiles: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn select(
        &self,
        request: &ScrapeJobRequest,
    ) -> Result<String, DomainError> {
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
            ProfileStrategy::Auto => {
                let profiles = self.scrape_client.list_profiles().await?;
                if profiles.is_empty() {
                    return Err(DomainError::Upstream("no profiles available".into()));
                }

                let blocked = self.blocked_profiles.read().await;
                let available: Vec<_> = profiles
                    .iter()
                    .filter(|p| !blocked.contains(*p))
                    .collect();

                let pool = if available.is_empty() {
                    &profiles
                } else {
                    // SAFETY: available is Vec<&String>, we need &[String]
                    // Use the index approach instead
                    let idx = self.counter.fetch_add(1, Ordering::Relaxed) % available.len();
                    return Ok(available[idx].clone());
                };

                let idx = self.counter.fetch_add(1, Ordering::Relaxed) % pool.len();
                Ok(pool[idx].clone())
            }
        }
    }

    pub fn report_failure(&self, profile: &str) {
        let blocked = self.blocked_profiles.clone();
        let profile = profile.to_string();
        tokio::spawn(async move {
            let mut set = blocked.write().await;
            set.insert(profile);
        });
    }
}
