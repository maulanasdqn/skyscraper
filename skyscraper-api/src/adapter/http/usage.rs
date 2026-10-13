use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use chrono::{Datelike, Utc};
use serde::Serialize;

use super::auth::AuthenticatedTenant;
use super::error::ApiResult;
use super::AppState;

#[derive(Debug, Serialize)]
pub struct UsageSummary {
    pub total_bytes: i64,
    pub period_start: String,
    pub period_end: String,
}

pub async fn get_usage(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<UsageSummary>> {
    let now = Utc::now();
    let naive = now.date_naive();
    let first_of_month = chrono::NaiveDate::from_ymd_opt(naive.year(), naive.month(), 1)
        .unwrap_or(naive);
    let period_start = first_of_month
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    let total_bytes = state
        .usage_repo
        .total_for_tenant(tenant.id, period_start)
        .await?;

    Ok(Json(UsageSummary {
        total_bytes,
        period_start: period_start.to_rfc3339(),
        period_end: now.to_rfc3339(),
    }))
}
