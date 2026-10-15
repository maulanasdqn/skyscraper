use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entity::{Job, ScrapeJobRequest, ScrapeJobResult};
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;
use crate::domain::port::JobRepository;

use super::job_repo_queries;

const JOB_COLS: &str = "id, tenant_id, status, priority, request, result, \
    carrier_id, profile_name, attempts, max_retries, \
    scheduled_at, started_at, completed_at, error, created_at";

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
        sqlx::query_as::<_, Job>(&format!(
            "INSERT INTO jobs (tenant_id, status, priority, request, max_retries, scheduled_at) \
             VALUES ($1, $2::job_status, $3, $4, $5, $6) RETURNING {JOB_COLS}",
        ))
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
        sqlx::query_as::<_, Job>(&format!("SELECT {JOB_COLS} FROM jobs WHERE id = $1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)
    }

    async fn list_by_tenant(
        &self,
        tid: Uuid,
        s: Option<JobStatus>,
        l: i64,
        o: i64,
    ) -> Result<Vec<Job>, DomainError> {
        job_repo_queries::list_by_tenant(&self.pool, tid, s, l, o).await
    }

    async fn set_running(
        &self,
        id: Uuid,
        cid: Option<Uuid>,
        profile: &str,
    ) -> Result<(), DomainError> {
        let r = sqlx::query(
            "UPDATE jobs SET status = 'running'::job_status, \
             carrier_id = $1, profile_name = $2, \
             started_at = NOW(), attempts = attempts + 1 WHERE id = $3",
        )
        .bind(cid)
        .bind(profile)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        if r.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("job {id}")));
        }
        Ok(())
    }

    async fn set_completed(&self, id: Uuid, result: &ScrapeJobResult) -> Result<(), DomainError> {
        let json = serde_json::to_value(result).map_err(|e| DomainError::Internal(e.into()))?;
        let r = sqlx::query("UPDATE jobs SET status = 'completed'::job_status, result = $1, completed_at = NOW() WHERE id = $2")
            .bind(&json).bind(id).execute(&self.pool).await.map_err(map_sqlx_error)?;
        if r.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("job {id}")));
        }
        Ok(())
    }

    async fn set_failed(&self, id: Uuid, error: &str, attempts: i32) -> Result<(), DomainError> {
        let r = sqlx::query("UPDATE jobs SET status = 'failed'::job_status, error = $1, attempts = $2 WHERE id = $3")
            .bind(error).bind(attempts).bind(id).execute(&self.pool).await.map_err(map_sqlx_error)?;
        if r.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("job {id}")));
        }
        Ok(())
    }

    async fn set_status(&self, id: Uuid, status: JobStatus) -> Result<(), DomainError> {
        let r = sqlx::query("UPDATE jobs SET status = $1::job_status WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        if r.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("job {id}")));
        }
        Ok(())
    }

    async fn count_running_for_tenant(&self, tid: Uuid) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM jobs WHERE tenant_id = $1 AND status = 'running'::job_status",
        )
        .bind(tid)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(row.0)
    }

    async fn count_by_tenant(
        &self,
        tid: Uuid,
        status: Option<JobStatus>,
    ) -> Result<i64, DomainError> {
        job_repo_queries::count_by_tenant(&self.pool, tid, status).await
    }

    async fn update_priority(&self, id: Uuid, priority: i16) -> Result<(), DomainError> {
        job_repo_queries::update_priority(&self.pool, id, priority).await
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let r = sqlx::query("DELETE FROM jobs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        if r.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("job {id}")));
        }
        Ok(())
    }

    async fn delete_completed_before(&self, before: DateTime<Utc>) -> Result<u64, DomainError> {
        job_repo_queries::delete_completed_before(&self.pool, before).await
    }

    async fn list_all(
        &self,
        s: Option<JobStatus>,
        tid: Option<Uuid>,
        l: i64,
        o: i64,
    ) -> Result<Vec<Job>, DomainError> {
        job_repo_queries::list_all(&self.pool, s, tid, l, o).await
    }

    async fn count_all(&self, s: Option<JobStatus>, tid: Option<Uuid>) -> Result<i64, DomainError> {
        job_repo_queries::count_all(&self.pool, s, tid).await
    }
}
