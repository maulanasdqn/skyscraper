use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entity::{Job, ScrapeJobRequest, ScrapeJobResult};
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;
use crate::domain::port::JobRepository;

pub struct PgJobRepository {
    pool: PgPool,
}

impl PgJobRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    DomainError::Internal(e.into())
}

#[async_trait]
impl JobRepository for PgJobRepository {
    async fn create(
        &self,
        tenant_id: Uuid,
        request: &ScrapeJobRequest,
        priority: i16,
        max_retries: i32,
        scheduled_at: Option<DateTime<Utc>>,
    ) -> Result<Job, DomainError> {
        let request_json =
            serde_json::to_value(request).map_err(|e| DomainError::Internal(e.into()))?;

        let status = if scheduled_at.is_some() {
            JobStatus::Scheduled
        } else {
            JobStatus::Pending
        };

        sqlx::query_as::<_, Job>(
            "INSERT INTO jobs (tenant_id, status, priority, request, max_retries, scheduled_at) \
             VALUES ($1, $2::job_status, $3, $4, $5, $6) \
             RETURNING id, tenant_id, status, priority, request, result, \
             carrier_id, profile_name, attempts, max_retries, \
             scheduled_at, started_at, completed_at, error, created_at",
        )
        .bind(tenant_id)
        .bind(status)
        .bind(priority)
        .bind(&request_json)
        .bind(max_retries)
        .bind(scheduled_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Job>, DomainError> {
        sqlx::query_as::<_, Job>(
            "SELECT id, tenant_id, status, priority, request, result, \
             carrier_id, profile_name, attempts, max_retries, \
             scheduled_at, started_at, completed_at, error, created_at \
             FROM jobs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    async fn list_by_tenant(
        &self,
        tenant_id: Uuid,
        status: Option<JobStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Job>, DomainError> {
        match status {
            Some(s) => {
                sqlx::query_as::<_, Job>(
                    "SELECT id, tenant_id, status, priority, request, result, \
                     carrier_id, profile_name, attempts, max_retries, \
                     scheduled_at, started_at, completed_at, error, created_at \
                     FROM jobs WHERE tenant_id = $1 AND status = $2::job_status \
                     ORDER BY created_at DESC LIMIT $3 OFFSET $4",
                )
                .bind(tenant_id)
                .bind(s)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, Job>(
                    "SELECT id, tenant_id, status, priority, request, result, \
                     carrier_id, profile_name, attempts, max_retries, \
                     scheduled_at, started_at, completed_at, error, created_at \
                     FROM jobs WHERE tenant_id = $1 \
                     ORDER BY created_at DESC LIMIT $2 OFFSET $3",
                )
                .bind(tenant_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(map_sqlx_error)
    }

    async fn set_running(
        &self,
        id: Uuid,
        carrier_id: Option<Uuid>,
        profile_name: &str,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            "UPDATE jobs SET status = 'running'::job_status, \
             carrier_id = $1, profile_name = $2, \
             started_at = NOW(), attempts = attempts + 1 \
             WHERE id = $3",
        )
        .bind(carrier_id)
        .bind(profile_name)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("job {id}")));
        }
        Ok(())
    }

    async fn set_completed(&self, id: Uuid, result: &ScrapeJobResult) -> Result<(), DomainError> {
        let result_json =
            serde_json::to_value(result).map_err(|e| DomainError::Internal(e.into()))?;

        let affected = sqlx::query(
            "UPDATE jobs SET status = 'completed'::job_status, \
             result = $1, completed_at = NOW() \
             WHERE id = $2",
        )
        .bind(&result_json)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        if affected.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("job {id}")));
        }
        Ok(())
    }

    async fn set_failed(&self, id: Uuid, error: &str, attempts: i32) -> Result<(), DomainError> {
        let affected = sqlx::query(
            "UPDATE jobs SET status = 'failed'::job_status, \
             error = $1, attempts = $2 \
             WHERE id = $3",
        )
        .bind(error)
        .bind(attempts)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        if affected.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("job {id}")));
        }
        Ok(())
    }

    async fn set_status(&self, id: Uuid, status: JobStatus) -> Result<(), DomainError> {
        let affected = sqlx::query("UPDATE jobs SET status = $1::job_status WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        if affected.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("job {id}")));
        }
        Ok(())
    }

    async fn count_running_for_tenant(&self, tenant_id: Uuid) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM jobs \
             WHERE tenant_id = $1 AND status = 'running'::job_status",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.0)
    }

    async fn count_by_tenant(
        &self,
        tenant_id: Uuid,
        status: Option<JobStatus>,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = match status {
            Some(s) => {
                sqlx::query_as(
                    "SELECT COUNT(*) FROM jobs \
                     WHERE tenant_id = $1 AND status = $2::job_status",
                )
                .bind(tenant_id)
                .bind(s)
                .fetch_one(&self.pool)
                .await
            }
            None => {
                sqlx::query_as("SELECT COUNT(*) FROM jobs WHERE tenant_id = $1")
                    .bind(tenant_id)
                    .fetch_one(&self.pool)
                    .await
            }
        }
        .map_err(map_sqlx_error)?;

        Ok(row.0)
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let affected = sqlx::query("DELETE FROM jobs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        if affected.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("job {id}")));
        }
        Ok(())
    }

    async fn delete_completed_before(&self, before: DateTime<Utc>) -> Result<u64, DomainError> {
        let result = sqlx::query(
            "DELETE FROM jobs \
             WHERE status IN ('completed'::job_status, 'dead'::job_status) \
             AND completed_at < $1",
        )
        .bind(before)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(result.rows_affected())
    }
}
