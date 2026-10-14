use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use base64::Engine;
use uuid::Uuid;

use super::auth::AuthenticatedTenant;
use super::error::{ApiError, ApiResult};
use super::AppState;
use crate::domain::entity::ScrapeJobResult;
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;

pub async fn get_result(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> ApiResult<Json<ScrapeJobResult>> {
    let job = state.scrape_service.get_job(tenant.0.id, job_id).await?;

    if job.status != JobStatus::Completed {
        return Err(DomainError::InvalidInput("job not completed yet".into()).into());
    }

    let result: ScrapeJobResult = job
        .result
        .ok_or_else(|| DomainError::NotFound("result not available".into()))
        .and_then(|v| {
            serde_json::from_value(v)
                .map_err(|e| DomainError::Internal(anyhow::anyhow!("malformed result: {e}")))
        })?;

    Ok(Json(result))
}

pub async fn get_screenshot(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let job = state.scrape_service.get_job(tenant.0.id, job_id).await?;

    if job.status != JobStatus::Completed {
        return Err(DomainError::InvalidInput("job not completed yet".into()).into());
    }

    let result: ScrapeJobResult = job
        .result
        .ok_or_else(|| DomainError::NotFound("result not available".into()))
        .and_then(|v| {
            serde_json::from_value(v)
                .map_err(|e| DomainError::Internal(anyhow::anyhow!("malformed result: {e}")))
        })?;

    let base64_data = result
        .screenshot_base64
        .ok_or_else(|| DomainError::NotFound("no screenshot available".into()))?;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&base64_data)
        .map_err(|e| DomainError::Internal(anyhow::anyhow!("invalid base64: {e}")))?;

    Ok((StatusCode::OK, [(header::CONTENT_TYPE, "image/png")], bytes))
}
