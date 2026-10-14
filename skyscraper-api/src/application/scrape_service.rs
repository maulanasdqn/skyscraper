use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entity::{Job, ScrapeJobRequest, Tenant};
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;
use crate::domain::port::{JobQueue, JobRepository, ScrapeService};

pub struct ScrapeServiceImpl {
    job_repo: Arc<dyn JobRepository>,
    queue: Arc<dyn JobQueue>,
}

impl ScrapeServiceImpl {
    pub fn new(job_repo: Arc<dyn JobRepository>, queue: Arc<dyn JobQueue>) -> Self {
        Self { job_repo, queue }
    }

    async fn enqueue(&self, job: &Job, priority: i16) -> Result<(), DomainError> {
        if let Some(scheduled_at) = job.scheduled_at {
            self.queue
                .schedule(job.id, scheduled_at.timestamp() as f64)
                .await?;
        } else {
            self.queue.enqueue(job.id, priority).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl ScrapeService for ScrapeServiceImpl {
    async fn submit_job(
        &self,
        tenant: &Tenant,
        request: &ScrapeJobRequest,
        priority: i16,
        max_retries: i32,
        scheduled_at: Option<DateTime<Utc>>,
    ) -> Result<Job, DomainError> {
        if request.url.is_empty() {
            return Err(DomainError::InvalidInput("url is required".into()));
        }

        let allowed = self
            .queue
            .check_rate_limit(&tenant.id, tenant.rate_limit_rpm)
            .await?;
        if !allowed {
            return Err(DomainError::RateLimitExceeded);
        }

        let running = self.job_repo.count_running_for_tenant(tenant.id).await?;
        if running >= i64::from(tenant.concurrent_limit) {
            return Err(DomainError::ConcurrencyLimitExceeded);
        }

        let job = self
            .job_repo
            .create(tenant.id, request, priority, max_retries, scheduled_at)
            .await?;

        self.enqueue(&job, priority).await?;
        Ok(job)
    }

    async fn get_job(&self, tenant_id: Uuid, job_id: Uuid) -> Result<Job, DomainError> {
        let job = self
            .job_repo
            .find_by_id(job_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("job not found".into()))?;

        if job.tenant_id != tenant_id {
            return Err(DomainError::NotFound("job not found".into()));
        }

        Ok(job)
    }

    async fn list_jobs(
        &self,
        tenant_id: Uuid,
        status: Option<JobStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Job>, DomainError> {
        self.job_repo
            .list_by_tenant(tenant_id, status, limit, offset)
            .await
    }

    async fn cancel_job(&self, tenant_id: Uuid, job_id: Uuid) -> Result<(), DomainError> {
        let job = self.get_job(tenant_id, job_id).await?;
        if job.status != JobStatus::Pending && job.status != JobStatus::Scheduled {
            return Err(DomainError::InvalidInput(
                "can only cancel pending or scheduled jobs".into(),
            ));
        }
        self.job_repo.delete(job_id).await
    }
}
