use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::domain::error::DomainError;

pub struct ApiError(pub DomainError);

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            DomainError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            DomainError::AlreadyExists(msg) => (StatusCode::CONFLICT, msg.clone()),
            DomainError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            DomainError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate limit exceeded".to_string(),
            ),
            DomainError::ConcurrencyLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "concurrency limit exceeded".to_string(),
            ),
            DomainError::TenantInactive => (StatusCode::FORBIDDEN, "tenant inactive".to_string()),
            DomainError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            DomainError::ScrapeBlocked => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "scrape blocked by target site".to_string(),
            ),
            DomainError::Upstream(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            DomainError::Internal(err) => {
                tracing::error!("internal error: {err:#}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        let body = json!({ "error": message });
        (status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = std::result::Result<T, ApiError>;
