use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};

use super::AppState;

pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

#[derive(Serialize)]
struct ReadinessResponse {
    status: &'static str,
    version: &'static str,
    checks: ReadinessChecks,
}

#[derive(Serialize)]
struct ReadinessChecks {
    postgres: CheckResult,
    redis: CheckResult,
}

#[derive(Serialize)]
struct CheckResult {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub async fn readiness(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let (pg_result, redis_result) = tokio::join!(check_postgres(&state), check_redis(&state));

    let all_healthy = pg_result.status == "ok" && redis_result.status == "ok";

    let resp = ReadinessResponse {
        status: if all_healthy { "ready" } else { "degraded" },
        version: env!("CARGO_PKG_VERSION"),
        checks: ReadinessChecks {
            postgres: pg_result,
            redis: redis_result,
        },
    };

    let status = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(resp))
}

async fn check_postgres(state: &AppState) -> CheckResult {
    let start = std::time::Instant::now();
    match state.tenant_repo.find_by_id(uuid::Uuid::nil()).await {
        Ok(_) | Err(crate::domain::error::DomainError::NotFound(_)) => CheckResult {
            status: "ok",
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Err(e) => CheckResult {
            status: "error",
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: Some(e.to_string()),
        },
    }
}

async fn check_redis(state: &AppState) -> CheckResult {
    let start = std::time::Instant::now();
    match state.queue.queue_stats().await {
        Ok(_) => CheckResult {
            status: "ok",
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Err(e) => CheckResult {
            status: "error",
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: Some(e.to_string()),
        },
    }
}
