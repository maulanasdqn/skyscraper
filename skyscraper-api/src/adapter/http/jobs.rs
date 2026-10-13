use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::auth::AuthenticatedTenant;
use super::error::{ApiError, ApiResult};
use super::AppState;
use crate::domain::entity::{Job, ScrapeJobRequest};
use crate::domain::error::DomainError;
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

#[derive(Debug, Deserialize)]
pub struct ScrapeQuery {
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    60_000
}

#[derive(Debug, Deserialize)]
pub struct BatchRequest {
    pub jobs: Vec<CreateJobRequest>,
}

#[derive(Debug, Serialize)]
pub struct BatchResponse {
    pub job_ids: Vec<Uuid>,
}

async fn enforce_rate_limit(
    state: &AppState,
    tenant: &AuthenticatedTenant,
) -> Result<(), ApiError> {
    let allowed = state
        .queue
        .check_rate_limit(&tenant.id, tenant.rate_limit_rpm)
        .await?;
    if !allowed {
        return Err(DomainError::RateLimitExceeded.into());
    }
    Ok(())
}

async fn enqueue_job(state: &AppState, job: &Job, priority: i16) -> Result<(), ApiError> {
    if let Some(scheduled_at) = job.scheduled_at {
        state
            .queue
            .schedule(job.id, scheduled_at.timestamp() as f64)
            .await?;
    } else {
        state.queue.enqueue(job.id, priority).await?;
    }
    Ok(())
}

pub async fn create_job(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateJobRequest>,
) -> ApiResult<(StatusCode, Json<Job>)> {
    if body.scrape.url.is_empty() {
        return Err(DomainError::InvalidInput("url is required".into()).into());
    }

    enforce_rate_limit(&state, &tenant).await?;

    let job = state
        .job_repo
        .create(
            tenant.id,
            &body.scrape,
            body.priority,
            body.max_retries,
            body.scheduled_at,
        )
        .await?;

    enqueue_job(&state, &job, body.priority).await?;

    Ok((StatusCode::CREATED, Json(job)))
}

pub async fn scrape_sync(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ScrapeQuery>,
    Json(body): Json<CreateJobRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.scrape.url.is_empty() {
        return Err(DomainError::InvalidInput("url is required".into()).into());
    }

    enforce_rate_limit(&state, &tenant).await?;

    let job = state
        .job_repo
        .create(
            tenant.id,
            &body.scrape,
            body.priority,
            body.max_retries,
            body.scheduled_at,
        )
        .await?;

    enqueue_job(&state, &job, body.priority).await?;

    let deadline =
        tokio::time::Instant::now() + std::time::Duration::from_millis(query.timeout_ms);
    let poll_interval = std::time::Duration::from_millis(100);

    loop {
        if tokio::time::Instant::now() >= deadline {
            return Ok((
                StatusCode::REQUEST_TIMEOUT,
                Json(serde_json::json!({
                    "error": "scrape timed out",
                    "job_id": job.id,
                })),
            ));
        }

        tokio::time::sleep(poll_interval).await;

        let current = state.job_repo.find_by_id(job.id).await?;
        let Some(current) = current else {
            return Err(DomainError::NotFound("job disappeared".into()).into());
        };

        match current.status {
            JobStatus::Completed => {
                return Ok((
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "job_id": current.id,
                        "status": "completed",
                        "result": current.result,
                    })),
                ));
            }
            JobStatus::Failed | JobStatus::Dead => {
                return Ok((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({
                        "job_id": current.id,
                        "status": current.status,
                        "error": current.error,
                    })),
                ));
            }
            _ => {}
        }
    }
}

pub async fn create_batch(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Json(body): Json<BatchRequest>,
) -> ApiResult<(StatusCode, Json<BatchResponse>)> {
    if body.jobs.is_empty() {
        return Err(DomainError::InvalidInput("jobs array is empty".into()).into());
    }
    if body.jobs.len() > 100 {
        return Err(DomainError::InvalidInput("max 100 jobs per batch".into()).into());
    }

    enforce_rate_limit(&state, &tenant).await?;

    let mut job_ids = Vec::with_capacity(body.jobs.len());
    for item in &body.jobs {
        if item.scrape.url.is_empty() {
            return Err(DomainError::InvalidInput("all jobs must have a url".into()).into());
        }

        let job = state
            .job_repo
            .create(
                tenant.id,
                &item.scrape,
                item.priority,
                item.max_retries,
                item.scheduled_at,
            )
            .await?;

        enqueue_job(&state, &job, item.priority).await?;
        job_ids.push(job.id);
    }

    Ok((StatusCode::CREATED, Json(BatchResponse { job_ids })))
}

pub async fn list_jobs(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListJobsQuery>,
) -> ApiResult<Json<Vec<Job>>> {
    let offset = (q.page - 1) * q.per_page;
    let jobs = state
        .job_repo
        .list_by_tenant(tenant.id, q.status, q.per_page, offset)
        .await?;
    Ok(Json(jobs))
}

pub async fn get_job(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Job>> {
    let job = state
        .job_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| DomainError::NotFound("job not found".into()))?;

    if job.tenant_id != tenant.id {
        return Err(DomainError::NotFound("job not found".into()).into());
    }

    Ok(Json(job))
}

pub async fn cancel_job(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let job = state
        .job_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| DomainError::NotFound("job not found".into()))?;

    if job.tenant_id != tenant.id {
        return Err(DomainError::NotFound("job not found".into()).into());
    }

    if job.status != JobStatus::Pending && job.status != JobStatus::Scheduled {
        return Err(
            DomainError::InvalidInput("can only cancel pending or scheduled jobs".into()).into(),
        );
    }

    state.job_repo.delete(id).await?;
    Ok(Json(serde_json::json!({ "cancelled": true })))
}
