use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entity::{Job, ScrapeJobRequest, Tenant, TenantPlan, UsageRecord};
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;
use crate::domain::port::{JobRepository, TenantRepository, UsageRepository};

type Result<T> = std::result::Result<T, DomainError>;

pub struct AppService {
    pub tenant_repo: Arc<dyn TenantRepository>,
    pub job_repo: Arc<dyn JobRepository>,
    pub usage_repo: Arc<dyn UsageRepository>,
}

impl AppService {
    pub fn new(
        tenant_repo: Arc<dyn TenantRepository>,
        job_repo: Arc<dyn JobRepository>,
        usage_repo: Arc<dyn UsageRepository>,
    ) -> Self {
        Self {
            tenant_repo,
            job_repo,
            usage_repo,
        }
    }

    pub async fn create_tenant(&self, email: &str, plan: TenantPlan) -> Result<Tenant> {
        self.tenant_repo.create(email, plan).await
    }

    pub async fn create_job(
        &self,
        tenant: &Tenant,
        request: &ScrapeJobRequest,
        priority: i16,
        max_retries: i32,
        scheduled_at: Option<DateTime<Utc>>,
    ) -> Result<Job> {
        let running = self
            .job_repo
            .count_running_for_tenant(tenant.id)
            .await?;

        if running >= i64::from(tenant.concurrent_limit) {
            return Err(DomainError::ConcurrencyLimitExceeded);
        }

        self.job_repo
            .create(tenant.id, request, priority, max_retries, scheduled_at)
            .await
    }

    pub async fn get_job(&self, tenant_id: Uuid, job_id: Uuid) -> Result<Job> {
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

    pub async fn list_jobs(
        &self,
        tenant_id: Uuid,
        status: Option<JobStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Job>> {
        self.job_repo
            .list_by_tenant(tenant_id, status, limit, offset)
            .await
    }

    pub async fn cancel_job(&self, tenant_id: Uuid, job_id: Uuid) -> Result<()> {
        let job = self.get_job(tenant_id, job_id).await?;

        if job.status != JobStatus::Pending && job.status != JobStatus::Scheduled {
            return Err(DomainError::InvalidInput(
                "can only cancel pending or scheduled jobs".into(),
            ));
        }

        self.job_repo.delete(job_id).await
    }

    pub async fn get_usage(&self, tenant_id: Uuid, since: DateTime<Utc>) -> Result<i64> {
        self.usage_repo.total_for_tenant(tenant_id, since).await
    }

    pub async fn list_usage(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UsageRecord>> {
        self.usage_repo
            .list_for_tenant(tenant_id, limit, offset)
            .await
    }
}
