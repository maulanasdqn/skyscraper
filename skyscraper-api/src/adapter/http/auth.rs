use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use super::AppState;
use crate::domain::entity::Tenant;

pub struct AuthenticatedTenant(pub Tenant);

enum AuthError {
    MissingKey,
    InvalidKey,
    Inactive,
    Internal,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            Self::MissingKey => (StatusCode::UNAUTHORIZED, "missing X-Api-Key header"),
            Self::InvalidKey => (StatusCode::UNAUTHORIZED, "invalid api key"),
            Self::Inactive => (StatusCode::FORBIDDEN, "tenant inactive"),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal error"),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

impl FromRequestParts<Arc<AppState>> for AuthenticatedTenant {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let api_key = parts
            .headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AuthError::MissingKey.into_response())?;

        let tenant = state
            .tenant_repo
            .find_by_api_key(api_key)
            .await
            .map_err(|_| AuthError::Internal.into_response())?
            .ok_or_else(|| AuthError::InvalidKey.into_response())?;

        if !tenant.active {
            return Err(AuthError::Inactive.into_response());
        }

        Ok(Self(tenant))
    }
}

pub struct AdminAuth;

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

impl FromRequestParts<Arc<AppState>> for AdminAuth {
    type Rejection = Response;

    #[allow(unknown_lints, clippy::unused_async, clippy::unused_async_trait_impl)]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let api_key = parts
            .headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AuthError::MissingKey.into_response())?;

        if !constant_time_eq(api_key.as_bytes(), state.admin_key.as_bytes()) {
            return Err(AuthError::InvalidKey.into_response());
        }

        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn constant_time_eq_same_strings() {
        assert!(constant_time_eq(b"hello", b"hello"));
    }

    #[test]
    fn constant_time_eq_different_strings() {
        assert!(!constant_time_eq(b"hello", b"world"));
    }

    #[test]
    fn constant_time_eq_different_lengths() {
        assert!(!constant_time_eq(b"hi", b"hello"));
    }

    #[test]
    fn constant_time_eq_empty_strings() {
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn constant_time_eq_one_byte_diff() {
        assert!(!constant_time_eq(b"abc", b"abd"));
    }
}
