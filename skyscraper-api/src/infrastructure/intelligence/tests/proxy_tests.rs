use crate::domain::entity::*;
use crate::domain::error::DomainError;

use super::test_helpers::*;

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
async fn proxy_auto_no_online_carriers_returns_none() {
    let selector = make_selector(vec![
        make_carrier("tmobile", "US", false),
        make_carrier("vodafone", "DE", false),
    ]);
    let req = make_request(ProxyStrategy::Auto, ProfileStrategy::Auto, None);
    let result = selector.select(&req).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn proxy_auto_avoids_failed_carrier() {
    let selector = make_selector(vec![
        make_carrier("tmobile", "US", true),
        make_carrier("att", "US", true),
    ]);
    for _ in 0..20 {
        selector
            .report_failure("socks5://tmobile-some-uuid:secret@proxy.local:1080")
            .await;
    }
    let req = make_request(ProxyStrategy::Auto, ProfileStrategy::Auto, None);
    let mut att_count = 0;
    for _ in 0..10 {
        let url = selector.select(&req).await.unwrap().unwrap();
        if url.starts_with("socks5://att-") {
            att_count += 1;
        }
    }
    assert!(
        att_count >= 8,
        "should strongly prefer att over failed tmobile, att picked {att_count}/10"
    );
}
