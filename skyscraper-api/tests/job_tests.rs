#[allow(dead_code)]
mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use uuid::Uuid;

use skyscraper::adapter::http::{router, AppState, ParallaxConfig};
use skyscraper::infrastructure::metrics::MetricsCollector;

use common::{
    body_json, test_app_with_tenant, test_tenant, MockCarrierClient, MockJobQueue,
    MockJobRepository, MockResponseCache, MockUsageRepository,
};

fn build_stateful_app(
    tenant: skyscraper::domain::entity::Tenant,
) -> (axum::Router, Arc<MockJobRepository>) {
    let tenant_repo = Arc::new(common::MockTenantRepository::with_tenant(tenant));
    let job_repo = Arc::new(MockJobRepository::new());
    let queue = Arc::new(MockJobQueue::new());
    let scrape_service = Arc::new(common::MockScrapeService {
        job_repo: job_repo.clone(),
        queue: queue.clone(),
    });
    let state = Arc::new(AppState {
        tenant_repo,
        job_repo: job_repo.clone(),
        usage_repo: Arc::new(MockUsageRepository),
        queue,
        carrier_client: Arc::new(MockCarrierClient),
        scrape_service,
        response_cache: Arc::new(MockResponseCache),
        parallax_config: ParallaxConfig {
            proxy_host: "localhost".into(),
            proxy_port: 1080,
            proxy_pass: "test_pass".into(),
        },
        admin_key: "admin_test_key".into(),
        metrics: Arc::new(MetricsCollector::new()),
    });
    (router(state), job_repo)
}

#[tokio::test]
async fn list_jobs_empty() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/jobs")
                .header("x-api-key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert!(body["data"].as_array().unwrap().is_empty());
    assert_eq!(body["pagination"]["page"], 1);
    assert_eq!(body["pagination"]["total"], 0);
}

#[tokio::test]
async fn get_job_not_found() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let app = test_app_with_tenant(tenant);

    let fake_id = Uuid::new_v4();
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/jobs/{fake_id}"))
                .header("x-api-key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_job_and_list() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let tenant_id = tenant.id;
    let (app, _job_repo) = build_stateful_app(tenant);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("x-api-key", &api_key)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"url": "https://example.com", "extract": [".title"]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp.into_body()).await;
    let job_id = body["id"].as_str().unwrap();
    assert_eq!(body["status"], "pending");
    assert_eq!(body["tenant_id"], tenant_id.to_string());

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/jobs")
                .header("x-api-key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    let arr = body["data"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_str().unwrap(), job_id);
    assert_eq!(body["pagination"]["total"], 1);
}

#[tokio::test]
async fn create_job_empty_url_rejected() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("x-api-key", &api_key)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"url": ""}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn cancel_pending_job() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let (app, _job_repo) = build_stateful_app(tenant);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("x-api-key", &api_key)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"url": "https://example.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp.into_body()).await;
    let job_id = body["id"].as_str().unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/jobs/{job_id}"))
                .header("x-api-key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["cancelled"], true);
}

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
