use std::sync::Arc;

use axum::extract::State;
use axum::Json;

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

pub async fn retry_dead(
    _admin: AdminAuth,
    State(_state): State<Arc<AppState>>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "message": "not yet implemented" })))
}

pub async fn list_carriers(
    _admin: AdminAuth,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<CarrierInfo>>> {
    let carriers = state.carrier_client.list_carriers().await?;
    Ok(Json(carriers))
}
