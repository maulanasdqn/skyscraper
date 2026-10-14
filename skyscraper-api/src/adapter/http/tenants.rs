use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::auth::AdminAuth;
use super::error::ApiResult;
use super::AppState;
use crate::domain::entity::{Tenant, TenantPlan};
use crate::domain::error::DomainError;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
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
pub struct CreateTenantRequest {
    pub email: String,
    #[serde(default)]
    pub plan: Option<TenantPlan>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTenantRequest {
    pub plan: Option<TenantPlan>,
    pub active: Option<bool>,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub api_key: String,
}

#[derive(Debug, Serialize)]
pub struct TenantCreatedResponse {
    pub id: Uuid,
    pub email: String,
    pub api_key: String,
    pub plan: TenantPlan,
    pub active: bool,
    pub rate_limit_rpm: i32,
    pub concurrent_limit: i32,
    pub created_at: DateTime<Utc>,
}

impl From<Tenant> for TenantCreatedResponse {
    fn from(t: Tenant) -> Self {
        Self {
            id: t.id,
            email: t.email,
            api_key: t.api_key,
            plan: t.plan,
            active: t.active,
            rate_limit_rpm: t.rate_limit_rpm,
            concurrent_limit: t.concurrent_limit,
            created_at: t.created_at,
        }
    }
}

pub async fn list_tenants(
    _admin: AdminAuth,
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<Tenant>>> {
    let offset = (q.page - 1) * q.per_page;
    let tenants = state.tenant_repo.list(q.per_page, offset).await?;
    Ok(Json(tenants))
}

pub async fn create_tenant(
    _admin: AdminAuth,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateTenantRequest>,
) -> ApiResult<Json<TenantCreatedResponse>> {
    if body.email.is_empty() {
        return Err(DomainError::InvalidInput("email is required".into()).into());
    }
    let plan = body.plan.unwrap_or(TenantPlan::Free);
    let tenant = state.tenant_repo.create(&body.email, plan).await?;
    Ok(Json(tenant.into()))
}

pub async fn update_tenant(
    _admin: AdminAuth,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTenantRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if let Some(plan) = body.plan {
        state.tenant_repo.update_plan(id, plan).await?;
    }
    if let Some(active) = body.active {
        state.tenant_repo.set_active(id, active).await?;
    }
    if let Some(ref url) = body.webhook_url {
        let url_opt = if url.is_empty() {
            None
        } else {
            Some(url.as_str())
        };
        state.tenant_repo.set_webhook_url(id, url_opt).await?;
    }
    Ok(Json(serde_json::json!({ "updated": true })))
}

pub async fn regenerate_key(
    _admin: AdminAuth,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiKeyResponse>> {
    let api_key = state.tenant_repo.regenerate_api_key(id).await?;
    Ok(Json(ApiKeyResponse { api_key }))
}
