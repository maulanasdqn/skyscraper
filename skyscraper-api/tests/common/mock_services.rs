use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use skyscraper::domain::entity::{
    CacheStats, CarrierInfo, Job, ScrapeJobRequest, ScrapeJobResult, Tenant,
};
use skyscraper::domain::error::DomainError;
use skyscraper::domain::job_status::JobStatus;
use skyscraper::domain::port::{
    CarrierClient, JobQueue, JobRepository, ResponseCache, ScrapeService,
};

use super::mock_job_repo::MockJobRepository;
use super::MockJobQueue;

pub struct MockScrapeService {
    pub job_repo: Arc<MockJobRepository>,
    pub queue: Arc<MockJobQueue>,
}

#[async_trait]
impl ScrapeService for MockScrapeService {
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
        let job = self
            .job_repo
            .create(tenant.id, request, priority, max_retries, scheduled_at)
            .await?;
        if let Some(sa) = job.scheduled_at {
            self.queue.schedule(job.id, sa.timestamp() as f64).await?;
        } else {
            self.queue.enqueue(job.id, priority).await?;
        }
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
        self.job_repo.set_status(job_id, JobStatus::Pending).await?;
        self.queue.enqueue(job_id, job.priority).await?;
        self.job_repo
            .find_by_id(job_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("job".into()))
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

pub struct MockCarrierClient;

#[async_trait]
impl CarrierClient for MockCarrierClient {
    async fn list_carriers(&self) -> Result<Vec<CarrierInfo>, DomainError> {
        Ok(vec![CarrierInfo {
            id: Uuid::new_v4(),
            name: "test-carrier".into(),
            country: "US".into(),
            upstream_addr: "127.0.0.1:1080".into(),
            online: true,
        }])
    }
}

pub struct MockResponseCache;

#[async_trait]
impl ResponseCache for MockResponseCache {
    async fn get(&self, _url: &str) -> Result<Option<ScrapeJobResult>, DomainError> {
        Ok(None)
    }
    async fn set(&self, _: &str, _: &ScrapeJobResult, _: Option<u64>) -> Result<(), DomainError> {
        Ok(())
    }
    async fn flush(&self) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn stats(&self) -> Result<CacheStats, DomainError> {
        Ok(CacheStats {
            keys: 0,
            memory_bytes: 0,
        })
    }
}
