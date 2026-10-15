#[allow(dead_code)]
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use common::{body_json, test_app};

#[tokio::test]
async fn admin_queue_stats() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/queue/stats")
                .header("x-api-key", "admin_test_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["pending"], 0);
    assert_eq!(body["processing"], 0);
    assert_eq!(body["dead"], 0);
}

#[tokio::test]
async fn admin_list_carriers() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/carriers")
                .header("x-api-key", "admin_test_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    let carriers = body.as_array().unwrap();
    assert_eq!(carriers.len(), 1);
    assert_eq!(carriers[0]["name"], "test-carrier");
    assert_eq!(carriers[0]["online"], true);
}
