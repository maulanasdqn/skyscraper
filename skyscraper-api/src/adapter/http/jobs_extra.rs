use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::auth::AuthenticatedTenant;
use super::error::{ApiError, ApiResult};
use super::jobs::CreateJobRequest;
use super::AppState;
use crate::domain::entity::Job;
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;

#[derive(Debug, Deserialize)]
pub struct BatchRequest {
    pub jobs: Vec<CreateJobRequest>,
}

#[derive(Debug, Serialize)]
pub struct BatchResponse {
    pub job_ids: Vec<Uuid>,
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

    let mut handles = Vec::with_capacity(body.jobs.len());
    for item in body.jobs {
        let svc = state.scrape_service.clone();
        let t = tenant.0.clone();
        handles.push(tokio::spawn(async move {
            svc.submit_job(
                &t,
                &item.scrape,
                item.priority,
                item.max_retries,
                item.scheduled_at,
            )
            .await
        }));
    }

    let mut job_ids = Vec::with_capacity(handles.len());
    for handle in handles {
        let job = handle
            .await
            .map_err(|e| DomainError::Internal(anyhow::anyhow!("batch task panicked: {e}")))?
            .map_err(ApiError::from)?;
        job_ids.push(job.id);
    }

    Ok((StatusCode::CREATED, Json(BatchResponse { job_ids })))
}

pub async fn retry_job(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Job>> {
    let job = state.scrape_service.retry_job(tenant.0.id, id).await?;
    Ok(Json(job))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePriorityRequest {
    pub priority: i16,
}

pub async fn update_priority(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdatePriorityRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    state
        .scrape_service
        .update_priority(tenant.0.id, id, body.priority)
        .await?;
    Ok(Json(
        serde_json::json!({ "updated": true, "priority": body.priority }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct ScrapeQuery {
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    60_000
}

pub async fn scrape_sync(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ScrapeQuery>,
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

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(query.timeout_ms);
    let mut interval = std::time::Duration::from_millis(100);

    loop {
        if tokio::time::Instant::now() >= deadline {
            return Ok((
                StatusCode::REQUEST_TIMEOUT,
                Json(serde_json::json!({"error": "scrape timed out", "job_id": job.id})),
            ));
        }

        tokio::time::sleep(interval).await;
        interval = (interval * 2).min(std::time::Duration::from_secs(2));

        let current = state.job_repo.find_by_id(job.id).await?;
        let Some(current) = current else {
            return Err(DomainError::NotFound("job disappeared".into()).into());
        };

        match current.status {
            JobStatus::Completed => {
                return Ok((
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "job_id": current.id, "status": "completed", "result": current.result,
                    })),
                ));
            }
            JobStatus::Failed | JobStatus::Dead => {
                return Ok((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({
                        "job_id": current.id, "status": current.status, "error": current.error,
                    })),
                ));
            }
            _ => {}
        }
    }
}
