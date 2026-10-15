use crate::domain::entity::Job;
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    DomainError::Internal(e.into())
}

const COLS: &str = "id, tenant_id, status, priority, request, result, \
    carrier_id, profile_name, attempts, max_retries, \
    scheduled_at, started_at, completed_at, error, created_at";

pub async fn count_by_tenant(
    pool: &PgPool,
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
            .fetch_one(pool)
            .await
        }
        None => {
            sqlx::query_as("SELECT COUNT(*) FROM jobs WHERE tenant_id = $1")
                .bind(tenant_id)
                .fetch_one(pool)
                .await
        }
    }
    .map_err(map_sqlx_error)?;
    Ok(row.0)
}

pub async fn list_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    status: Option<JobStatus>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Job>, DomainError> {
    match status {
        Some(s) => {
            sqlx::query_as::<_, Job>(&format!(
                "SELECT {COLS} FROM jobs WHERE tenant_id = $1 AND status = $2::job_status \
                 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
            ))
            .bind(tenant_id)
            .bind(s)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, Job>(&format!(
                "SELECT {COLS} FROM jobs WHERE tenant_id = $1 \
                 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            ))
            .bind(tenant_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(map_sqlx_error)
}
pub async fn update_priority(pool: &PgPool, id: Uuid, priority: i16) -> Result<(), DomainError> {
    let affected = sqlx::query("UPDATE jobs SET priority = $1 WHERE id = $2")
        .bind(priority)
        .bind(id)
        .execute(pool)
        .await
        .map_err(map_sqlx_error)?;
    if affected.rows_affected() == 0 {
        return Err(DomainError::NotFound(format!("job {id}")));
    }
    Ok(())
}
pub async fn delete_completed_before(
    pool: &PgPool,
    before: DateTime<Utc>,
) -> Result<u64, DomainError> {
    let result = sqlx::query(
        "DELETE FROM jobs \
         WHERE status IN ('completed'::job_status, 'dead'::job_status) \
         AND completed_at < $1",
    )
    .bind(before)
    .execute(pool)
    .await
    .map_err(map_sqlx_error)?;
    Ok(result.rows_affected())
}

pub async fn list_all(
    pool: &PgPool,
    status: Option<JobStatus>,
    tenant_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Job>, DomainError> {
    match (status, tenant_id) {
        (Some(s), Some(tid)) => {
            sqlx::query_as::<_, Job>(&format!(
                "SELECT {COLS} FROM jobs WHERE status = $1::job_status AND tenant_id = $2 \
                 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
            ))
            .bind(s)
            .bind(tid)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
        }
        (Some(s), None) => {
            sqlx::query_as::<_, Job>(&format!(
                "SELECT {COLS} FROM jobs WHERE status = $1::job_status \
                 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            ))
            .bind(s)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
        }
        (None, Some(tid)) => {
            sqlx::query_as::<_, Job>(&format!(
                "SELECT {COLS} FROM jobs WHERE tenant_id = $1 \
                 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            ))
            .bind(tid)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
        }
        (None, None) => {
            sqlx::query_as::<_, Job>(&format!(
                "SELECT {COLS} FROM jobs ORDER BY created_at DESC LIMIT $1 OFFSET $2"
            ))
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(map_sqlx_error)
}
pub async fn count_all(
    pool: &PgPool,
    status: Option<JobStatus>,
    tenant_id: Option<Uuid>,
) -> Result<i64, DomainError> {
    let row: (i64,) = match (status, tenant_id) {
        (Some(s), Some(tid)) => {
            sqlx::query_as(
                "SELECT COUNT(*) FROM jobs \
                 WHERE status = $1::job_status AND tenant_id = $2",
            )
            .bind(s)
            .bind(tid)
            .fetch_one(pool)
            .await
        }
        (Some(s), None) => {
            sqlx::query_as("SELECT COUNT(*) FROM jobs WHERE status = $1::job_status")
                .bind(s)
                .fetch_one(pool)
                .await
        }
        (None, Some(tid)) => {
            sqlx::query_as("SELECT COUNT(*) FROM jobs WHERE tenant_id = $1")
                .bind(tid)
                .fetch_one(pool)
                .await
        }
        (None, None) => {
            sqlx::query_as("SELECT COUNT(*) FROM jobs")
                .fetch_one(pool)
                .await
        }
    }
    .map_err(map_sqlx_error)?;
    Ok(row.0)
}
