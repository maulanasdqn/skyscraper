#[allow(dead_code)]
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use common::{body_json, test_app_with_tenant, test_tenant};

#[tokio::test]
async fn batch_create_jobs() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs/batch")
                .header("x-api-key", &api_key)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"jobs": [
                        {"url": "https://a.com"},
                        {"url": "https://b.com"},
                        {"url": "https://c.com"}
                    ]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["job_ids"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn batch_rejects_empty() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs/batch")
                .header("x-api-key", &api_key)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"jobs": []}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn usage_returns_zero() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/usage")
                .header("x-api-key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["total_bytes"], 0);
}
