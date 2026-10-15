#[allow(dead_code)]
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use common::test_app;

#[tokio::test]
async fn response_includes_request_id() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let request_id = resp.headers().get("x-request-id");
    assert!(request_id.is_some(), "response must include x-request-id");
    let id_str = request_id.unwrap().to_str().unwrap();
    assert!(
        uuid::Uuid::parse_str(id_str).is_ok(),
        "x-request-id should be a valid UUID, got: {id_str}"
    );
}

#[tokio::test]
async fn request_id_echoed_if_provided() {
    let app = test_app();
    let custom_id = "my-custom-request-id-12345";
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .header("x-request-id", custom_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let returned_id = resp
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(returned_id, custom_id);
}
