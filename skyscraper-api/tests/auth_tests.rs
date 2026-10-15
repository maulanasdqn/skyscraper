#[allow(dead_code)]
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use common::{test_app, test_app_with_tenant, test_tenant};

#[tokio::test]
async fn missing_api_key_returns_401() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_api_key_returns_401() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/jobs")
                .header("x-api-key", "sk_nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn inactive_tenant_returns_403() {
    let mut tenant = test_tenant();
    tenant.active = false;

    let app = test_app_with_tenant(tenant.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/jobs")
                .header("x-api-key", &tenant.api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
