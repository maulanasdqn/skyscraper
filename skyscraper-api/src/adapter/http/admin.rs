use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use super::auth::AdminAuth;
use super::error::ApiResult;
use super::AppState;
use crate::domain::entity::{CarrierInfo, QueueStats};

pub async fn queue_stats(
    _admin: AdminAuth,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<QueueStats>> {
    let result = state.queue.queue_stats().await?;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct RetryDeadQuery {
    #[serde(default = "default_retry_limit")]
    pub limit: usize,
}

fn default_retry_limit() -> usize {
    100
}

pub async fn retry_dead(
    _admin: AdminAuth,
    State(state): State<Arc<AppState>>,
    Query(q): Query<RetryDeadQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let retried = state.queue.retry_dead(q.limit).await?;
    Ok(Json(serde_json::json!({ "retried": retried })))
}

pub async fn list_carriers(
    _admin: AdminAuth,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<CarrierInfo>>> {
    let carriers = state.carrier_client.list_carriers().await?;
    Ok(Json(carriers))
}

pub async fn metrics(
    _admin: AdminAuth,
    State(state): State<Arc<AppState>>,
) -> Json<crate::infrastructure::metrics::Metrics> {
    Json(state.metrics.snapshot())
}
