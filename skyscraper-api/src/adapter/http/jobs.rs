use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use super::auth::AuthenticatedTenant;
use super::error::{ApiError, ApiResult};
use super::pagination::PaginatedResponse;
use super::AppState;
use crate::domain::entity::{Job, ScrapeJobRequest};
use crate::domain::job_status::JobStatus;

#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    #[serde(flatten)]
    pub scrape: ScrapeJobRequest,
    #[serde(default = "default_priority")]
    pub priority: i16,
    #[serde(default = "default_max_retries")]
    pub max_retries: i32,
    pub scheduled_at: Option<DateTime<Utc>>,
}

fn default_priority() -> i16 {
    5
}
fn default_max_retries() -> i32 {
    2
}

#[derive(Debug, Deserialize)]
pub struct ListJobsQuery {
    pub status: Option<JobStatus>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}
fn default_per_page() -> i64 {
    20
}

fn rate_limit_headers(limit: i32, remaining: i64, reset_epoch: i64) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(&limit.to_string()) {
        headers.insert("x-ratelimit-limit", v);
    }
    if let Ok(v) = HeaderValue::from_str(&remaining.max(0).to_string()) {
        headers.insert("x-ratelimit-remaining", v);
    }
    if let Ok(v) = HeaderValue::from_str(&reset_epoch.to_string()) {
        headers.insert("x-ratelimit-reset", v);
    }
    headers
}

pub async fn create_job(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateJobRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let job = state
        .scrape_service
        .submit_job(
            &tenant.0,
            &body.scrape,
            body.priority,
            body.max_retries,
            body.scheduled_at,
        )
        .await?;

    let rl = state
        .queue
        .rate_limit_info(&tenant.0.id, tenant.0.rate_limit_rpm)
        .await
        .unwrap_or(crate::domain::entity::RateLimitInfo {
            limit: tenant.0.rate_limit_rpm,
            remaining: 0,
            reset_epoch: 0,
        });

    let headers = rate_limit_headers(rl.limit, rl.remaining, rl.reset_epoch);
    Ok((StatusCode::CREATED, headers, Json(job)))
}

pub async fn list_jobs(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListJobsQuery>,
) -> ApiResult<Json<PaginatedResponse<Job>>> {
    let offset = (q.page - 1) * q.per_page;
    let jobs = state
        .scrape_service
        .list_jobs(tenant.0.id, q.status, q.per_page, offset)
        .await?;
    let total = state
        .job_repo
        .count_by_tenant(tenant.0.id, q.status)
        .await?;
    Ok(Json(PaginatedResponse::new(
        jobs, q.page, q.per_page, total,
    )))
}

pub async fn get_job(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Job>> {
    let job = state.scrape_service.get_job(tenant.0.id, id).await?;
    Ok(Json(job))
}

pub async fn cancel_job(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    state.scrape_service.cancel_job(tenant.0.id, id).await?;
    Ok(Json(serde_json::json!({ "cancelled": true })))
}
