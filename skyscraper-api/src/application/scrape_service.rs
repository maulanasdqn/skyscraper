use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Utc};
use uuid::Uuid;

use crate::domain::entity::{Job, ScrapeJobRequest, Tenant};
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;
use crate::domain::port::{JobQueue, JobRepository, ScrapeService, UsageRepository};
use crate::domain::validation::validate_scrape_url;

pub struct ScrapeServiceImpl {
    job_repo: Arc<dyn JobRepository>,
    queue: Arc<dyn JobQueue>,
    usage_repo: Arc<dyn UsageRepository>,
}

impl ScrapeServiceImpl {
    pub fn new(
        job_repo: Arc<dyn JobRepository>,
        queue: Arc<dyn JobQueue>,
        usage_repo: Arc<dyn UsageRepository>,
    ) -> Self {
        Self {
            job_repo,
            queue,
            usage_repo,
        }
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

    async fn check_quota(&self, tenant: &Tenant) -> Result<(), DomainError> {
        let Some(limit) = tenant.plan.monthly_request_limit() else {
            return Ok(());
        };

        let now = Utc::now();
        let naive = now.date_naive();
        let first_of_month =
            chrono::NaiveDate::from_ymd_opt(naive.year(), naive.month(), 1).unwrap_or(naive);
        let period_start = first_of_month.and_hms_opt(0, 0, 0).unwrap().and_utc();

        let used = self
            .usage_repo
            .total_for_tenant(tenant.id, period_start)
            .await?;

        if used >= limit {
            return Err(DomainError::QuotaExceeded(format!(
                "{:?} plan allows {limit} requests/month",
                tenant.plan
            )));
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
        validate_scrape_url(&request.url)?;
        self.check_quota(tenant).await?;

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

    async fn retry_job(&self, tenant_id: Uuid, job_id: Uuid) -> Result<Job, DomainError> {
        let job = self.get_job(tenant_id, job_id).await?;
        if job.status != JobStatus::Failed && job.status != JobStatus::Dead {
            return Err(DomainError::InvalidInput(
                "can only retry failed or dead jobs".into(),
            ));
        }

        self.job_repo.set_failed(job_id, "", 0).await?;
        self.job_repo.set_status(job_id, JobStatus::Pending).await?;
        self.queue.enqueue(job_id, job.priority).await?;

        self.job_repo
            .find_by_id(job_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("job not found".into()))
    }

    async fn update_priority(
        &self,
        tenant_id: Uuid,
        job_id: Uuid,
        priority: i16,
    ) -> Result<(), DomainError> {
        let job = self.get_job(tenant_id, job_id).await?;
        if job.status != JobStatus::Pending && job.status != JobStatus::Scheduled {
            return Err(DomainError::InvalidInput(
                "can only reprioritize pending or scheduled jobs".into(),
            ));
        }
        self.job_repo.update_priority(job_id, priority).await
    }
}
