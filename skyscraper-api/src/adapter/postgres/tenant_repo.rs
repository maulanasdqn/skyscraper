use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entity::{Tenant, TenantPlan};
use crate::domain::error::DomainError;
use crate::domain::port::TenantRepository;

pub struct PgTenantRepository {
    pool: PgPool,
}

impl PgTenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    if let sqlx::Error::Database(ref db_err) = e {
        if db_err.code().as_deref() == Some("23505") {
            return DomainError::AlreadyExists(db_err.message().to_string());
        }
    }
    DomainError::Internal(e.into())
}

#[async_trait]
impl TenantRepository for PgTenantRepository {
    async fn find_by_api_key(&self, key: &str) -> Result<Option<Tenant>, DomainError> {
        sqlx::query_as::<_, Tenant>(
            "SELECT id, email, api_key, plan, active, rate_limit_rpm, \
             concurrent_limit, webhook_url, created_at \
             FROM tenants WHERE api_key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Tenant>, DomainError> {
        sqlx::query_as::<_, Tenant>(
            "SELECT id, email, api_key, plan, active, rate_limit_rpm, \
             concurrent_limit, webhook_url, created_at \
             FROM tenants WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    async fn create(&self, email: &str, plan: TenantPlan) -> Result<Tenant, DomainError> {
        let api_key = format!("sk_{}", Uuid::new_v4());
        sqlx::query_as::<_, Tenant>(
            "INSERT INTO tenants (email, api_key, plan) \
             VALUES ($1, $2, $3::tenant_plan) \
             RETURNING id, email, api_key, plan, active, rate_limit_rpm, \
             concurrent_limit, webhook_url, created_at",
        )
        .bind(email)
        .bind(&api_key)
        .bind(&plan)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Tenant>, DomainError> {
        sqlx::query_as::<_, Tenant>(
            "SELECT id, email, api_key, plan, active, rate_limit_rpm, \
             concurrent_limit, webhook_url, created_at \
             FROM tenants ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    async fn update_plan(&self, id: Uuid, plan: TenantPlan) -> Result<(), DomainError> {
        let result = sqlx::query("UPDATE tenants SET plan = $1::tenant_plan WHERE id = $2")
            .bind(&plan)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("tenant {id}")));
        }
        Ok(())
    }

    async fn set_active(&self, id: Uuid, active: bool) -> Result<(), DomainError> {
        let result = sqlx::query("UPDATE tenants SET active = $1 WHERE id = $2")
            .bind(active)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("tenant {id}")));
        }
        Ok(())
    }

    async fn set_webhook_url(&self, id: Uuid, url: Option<&str>) -> Result<(), DomainError> {
        let result = sqlx::query("UPDATE tenants SET webhook_url = $1 WHERE id = $2")
            .bind(url)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("tenant {id}")));
        }
        Ok(())
    }

    async fn regenerate_api_key(&self, id: Uuid) -> Result<String, DomainError> {
        let new_key = format!("sk_{}", Uuid::new_v4());
        let result = sqlx::query("UPDATE tenants SET api_key = $1 WHERE id = $2")
            .bind(&new_key)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("tenant {id}")));
        }
        Ok(new_key)
    }
}
