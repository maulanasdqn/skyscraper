use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entity::*;
use crate::domain::error::DomainError;
use crate::domain::port::{CarrierClient, ScrapeClient};

use super::profile_rotator::ProfileRotator;
use super::proxy_selector::ProxySelector;

struct MockCarrierClient {
    carriers: Vec<CarrierInfo>,
}

#[async_trait]
impl CarrierClient for MockCarrierClient {
    async fn list_carriers(&self) -> Result<Vec<CarrierInfo>, DomainError> {
        Ok(self.carriers.clone())
    }
}

struct MockScrapeClient {
    profiles: Vec<String>,
}

#[async_trait]
impl ScrapeClient for MockScrapeClient {
    async fn scrape(
        &self,
        _url: &str,
        _profile: &str,
        _request: &ScrapeJobRequest,
        _proxy_url: Option<&str>,
    ) -> Result<ScrapeJobResult, DomainError> {
        unimplemented!()
    }

    async fn list_profiles(&self) -> Result<Vec<String>, DomainError> {
        Ok(self.profiles.clone())
    }
}

fn make_request(
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

fn make_carrier(name: &str, country: &str, online: bool) -> CarrierInfo {
    CarrierInfo {
        id: Uuid::new_v4(),
        name: name.into(),
        country: country.into(),
        upstream_addr: "10.0.0.1:1080".into(),
        online,
    }
}

fn make_selector(carriers: Vec<CarrierInfo>) -> ProxySelector {
    let client: Arc<dyn CarrierClient> = Arc::new(MockCarrierClient { carriers });
    ProxySelector::new(client, "proxy.local".into(), 1080, "secret".into())
}

fn make_rotator(profiles: Vec<&str>) -> ProfileRotator {
    let client: Arc<dyn ScrapeClient> = Arc::new(MockScrapeClient {
        profiles: profiles.into_iter().map(String::from).collect(),
    });
    ProfileRotator::new(client)
}

// ── ProxySelector ────────────────────────────────────────────────────

#[tokio::test]
async fn proxy_none_returns_none() {
    let selector = make_selector(vec![make_carrier("tmobile", "US", true)]);
    let req = make_request(ProxyStrategy::None, ProfileStrategy::Auto, None);
    let result = selector.select(&req).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn proxy_carrier_returns_socks5_url() {
    let selector = make_selector(vec![make_carrier("tmobile", "US", true)]);
    let req = make_request(
        ProxyStrategy::Carrier {
            name: "tmobile".into(),
        },
        ProfileStrategy::Auto,
        None,
    );
    let url = selector.select(&req).await.unwrap().unwrap();
    assert!(url.starts_with("socks5://tmobile-"));
    assert!(url.contains(":secret@proxy.local:1080"));
}

#[tokio::test]
async fn proxy_carrier_unknown_returns_not_found() {
    let selector = make_selector(vec![make_carrier("tmobile", "US", true)]);
    let req = make_request(
        ProxyStrategy::Carrier {
            name: "verizon".into(),
        },
        ProfileStrategy::Auto,
        None,
    );
    let err = selector.select(&req).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn proxy_carrier_offline_returns_upstream_error() {
    let selector = make_selector(vec![make_carrier("tmobile", "US", false)]);
    let req = make_request(
        ProxyStrategy::Carrier {
            name: "tmobile".into(),
        },
        ProfileStrategy::Auto,
        None,
    );
    let err = selector.select(&req).await.unwrap_err();
    assert!(matches!(err, DomainError::Upstream(_)));
}

#[tokio::test]
async fn proxy_auto_prefers_geo_match() {
    let selector = make_selector(vec![
        make_carrier("vodafone", "DE", true),
        make_carrier("tmobile", "US", true),
    ]);
    let req = make_request(ProxyStrategy::Auto, ProfileStrategy::Auto, Some("US"));
    let url = selector.select(&req).await.unwrap().unwrap();
    assert!(url.starts_with("socks5://tmobile-"));
}

#[tokio::test]
async fn proxy_auto_no_online_carriers_returns_error() {
    let selector = make_selector(vec![
        make_carrier("tmobile", "US", false),
        make_carrier("vodafone", "DE", false),
    ]);
    let req = make_request(ProxyStrategy::Auto, ProfileStrategy::Auto, None);
    let err = selector.select(&req).await.unwrap_err();
    assert!(matches!(err, DomainError::Upstream(_)));
}

#[tokio::test]
async fn proxy_auto_avoids_failed_carrier() {
    let selector = make_selector(vec![
        make_carrier("tmobile", "US", true),
        make_carrier("att", "US", true),
    ]);

    selector
        .report_failure("socks5://tmobile-some-uuid:secret@proxy.local:1080")
        .await;

    let req = make_request(ProxyStrategy::Auto, ProfileStrategy::Auto, None);
    let url = selector.select(&req).await.unwrap().unwrap();
    assert!(
        url.starts_with("socks5://att-"),
        "should prefer att over failed tmobile, got: {url}"
    );
}

// ── ProfileRotator ───────────────────────────────────────────────────

#[tokio::test]
async fn profile_named_returns_exact() {
    let rotator = make_rotator(vec!["win11", "macos14"]);
    let req = make_request(
        ProxyStrategy::None,
        ProfileStrategy::Named {
            name: "win11".into(),
        },
        None,
    );
    let result = rotator.select(&req).await.unwrap();
    assert_eq!(result, "win11");
}

#[tokio::test]
async fn profile_auto_round_robins() {
    let rotator = make_rotator(vec!["win11", "macos14", "linux"]);
    let req = make_request(ProxyStrategy::None, ProfileStrategy::Auto, None);

    let first = rotator.select(&req).await.unwrap();
    let second = rotator.select(&req).await.unwrap();
    let third = rotator.select(&req).await.unwrap();
    let fourth = rotator.select(&req).await.unwrap();

    assert_ne!(first, second);
    assert_ne!(second, third);
    assert_eq!(first, fourth); // wraps around
}

#[tokio::test]
async fn profile_random_returns_valid() {
    let profiles = vec!["win11", "macos14", "linux"];
    let rotator = make_rotator(profiles.clone());
    let req = make_request(ProxyStrategy::None, ProfileStrategy::Random, None);
    let result = rotator.select(&req).await.unwrap();
    assert!(profiles.contains(&result.as_str()));
}

#[tokio::test]
async fn profile_auto_skips_blocked() {
    let rotator = make_rotator(vec!["win11", "macos14"]);

    rotator.report_failure("win11").await;

    let req = make_request(ProxyStrategy::None, ProfileStrategy::Auto, None);

    // All selections should be macos14 since win11 is blocked
    for _ in 0..5 {
        let result = rotator.select(&req).await.unwrap();
        assert_eq!(result, "macos14", "should skip blocked win11");
    }
}

#[tokio::test]
async fn profile_empty_returns_error() {
    let rotator = make_rotator(vec![]);
    let req = make_request(ProxyStrategy::None, ProfileStrategy::Auto, None);
    let err = rotator.select(&req).await.unwrap_err();
    assert!(matches!(err, DomainError::Upstream(_)));
}
