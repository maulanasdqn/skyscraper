#[allow(dead_code)]
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use common::{body_json, test_app, test_app_with_tenant, test_tenant};

#[tokio::test]
async fn admin_create_tenant() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/tenants")
                .header("x-api-key", "admin_test_key")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email": "new@test.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["email"], "new@test.com");
    assert!(body["api_key"].as_str().unwrap().starts_with("sk_"));
    assert_eq!(body["active"], true);
}

#[tokio::test]
async fn admin_list_tenants() {
    let tenant = test_tenant();
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/tenants")
                .header("x-api-key", "admin_test_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["email"], "user@test.com");
}

#[tokio::test]
async fn admin_update_tenant() {
    let tenant = test_tenant();
    let id = tenant.id;
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/admin/tenants/{id}"))
                .header("x-api-key", "admin_test_key")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"active": false}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["updated"], true);
}

#[tokio::test]
async fn admin_regenerate_key() {
    let tenant = test_tenant();
    let id = tenant.id;
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/admin/tenants/{id}/regenerate-key"))
                .header("x-api-key", "admin_test_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert!(body["api_key"].as_str().unwrap().starts_with("sk_"));
}

#[tokio::test]
async fn admin_endpoints_reject_non_admin_key() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/tenants")
                .header("x-api-key", "sk_not_admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_endpoints_reject_missing_key() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/tenants")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
