use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entity::UsageRecord;
use crate::domain::error::DomainError;
use crate::domain::port::UsageRepository;

pub struct PgUsageRepository {
    pool: PgPool,
}

impl PgUsageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    DomainError::Internal(e.into())
}

#[async_trait]
impl UsageRepository for PgUsageRepository {
    async fn record(
        &self,
        tenant_id: Uuid,
        job_id: Uuid,
        bytes: i64,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO usage_records (tenant_id, job_id, bytes_consumed) \
             VALUES ($1, $2, $3)",
        )
        .bind(tenant_id)
        .bind(job_id)
        .bind(bytes)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn total_for_tenant(
        &self,
        tenant_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        let row: (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(bytes_consumed) FROM usage_records \
             WHERE tenant_id = $1 AND recorded_at >= $2",
        )
        .bind(tenant_id)
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.0.unwrap_or(0))
    }

    async fn list_for_tenant(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UsageRecord>, DomainError> {
        sqlx::query_as::<_, UsageRecord>(
            "SELECT id, tenant_id, job_id, bytes_consumed, billed, recorded_at \
             FROM usage_records WHERE tenant_id = $1 \
             ORDER BY recorded_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }
}
