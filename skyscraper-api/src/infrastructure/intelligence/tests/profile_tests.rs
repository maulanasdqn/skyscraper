use crate::domain::entity::*;
use crate::domain::error::DomainError;

use super::test_helpers::*;

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
    assert_eq!(first, fourth);
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
