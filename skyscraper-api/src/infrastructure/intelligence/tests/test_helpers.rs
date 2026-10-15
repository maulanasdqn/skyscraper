use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entity::*;
use crate::domain::error::DomainError;
use crate::domain::port::{CarrierClient, ScrapeClient};

use crate::infrastructure::intelligence::profile_rotator::ProfileRotator;
use crate::infrastructure::intelligence::proxy_selector::ProxySelector;

pub struct MockCarrierClient {
    pub carriers: Vec<CarrierInfo>,
}

#[async_trait]
impl CarrierClient for MockCarrierClient {
    async fn list_carriers(&self) -> Result<Vec<CarrierInfo>, DomainError> {
        Ok(self.carriers.clone())
    }
}

pub struct MockScrapeClient {
    pub profiles: Vec<String>,
}

#[async_trait]
impl ScrapeClient for MockScrapeClient {
    async fn scrape(
        &self,
        _: &str,
        _: &str,
        _: &ScrapeJobRequest,
        _: Option<&str>,
    ) -> Result<ScrapeJobResult, DomainError> {
        unimplemented!()
    }
    async fn list_profiles(&self) -> Result<Vec<String>, DomainError> {
        Ok(self.profiles.clone())
    }
}

pub fn make_request(
    proxy: ProxyStrategy,
    profile: ProfileStrategy,
    geo: Option<&str>,
) -> ScrapeJobRequest {
    ScrapeJobRequest {
        url: "https://example.com".into(),
        extract: vec![],
        script: None,
        workflow: vec![],
        screenshot: false,
        include_html: false,
        wait_ms: 3000,
        headful: false,
        geo: geo.map(String::from),
        proxy_strategy: proxy,
        profile_strategy: profile,
    }
}

pub fn make_carrier(name: &str, country: &str, online: bool) -> CarrierInfo {
    CarrierInfo {
        id: Uuid::new_v4(),
        name: name.into(),
        country: country.into(),
        upstream_addr: "10.0.0.1:1080".into(),
        online,
    }
}

pub fn make_selector(carriers: Vec<CarrierInfo>) -> ProxySelector {
    let client: Arc<dyn CarrierClient> = Arc::new(MockCarrierClient { carriers });
    ProxySelector::new(client, "proxy.local".into(), 1080, "secret".into())
}

pub fn make_rotator(profiles: Vec<&str>) -> ProfileRotator {
    let client: Arc<dyn ScrapeClient> = Arc::new(MockScrapeClient {
        profiles: profiles.into_iter().map(String::from).collect(),
    });
    ProfileRotator::new(client)
}
