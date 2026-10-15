pub mod mocks;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::Body;
use axum::Router;
use chrono::{DateTime, Utc};
use http_body_util::BodyExt;
use uuid::Uuid;

use skyscraper::adapter::http::{router, AppState, ParallaxConfig};
use skyscraper::domain::entity::{
    CarrierInfo, Job, QueueStats, ScrapeJobRequest, Tenant, TenantPlan,
};
use skyscraper::domain::error::DomainError;
use skyscraper::domain::job_status::JobStatus;
use skyscraper::domain::port::{CarrierClient, JobQueue, JobRepository, ScrapeService};
use skyscraper::infrastructure::metrics::MetricsCollector;

pub use mocks::{MockJobRepository, MockTenantRepository, MockUsageRepository};

pub struct MockJobQueue {
    pub enqueued: Mutex<Vec<Uuid>>,
}

impl MockJobQueue {
    pub fn new() -> Self {
        Self {
            enqueued: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl JobQueue for MockJobQueue {
    async fn enqueue(&self, job_id: Uuid, _priority: i16) -> Result<(), DomainError> {
        self.enqueued.lock().unwrap().push(job_id);
        Ok(())
    }
    async fn schedule(&self, _job_id: Uuid, _run_at_epoch: f64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn dequeue(&self, _timeout_secs: f64) -> Result<Option<Uuid>, DomainError> {
        Ok(None)
    }
    async fn ack(&self, _job_id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn move_to_dead(&self, _job_id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn promote_scheduled(&self) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn reclaim_stale(&self, _max_age_secs: u64) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn check_rate_limit(
        &self,
        _tenant_id: &Uuid,
        _limit_rpm: i32,
    ) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn queue_stats(&self) -> Result<QueueStats, DomainError> {
        Ok(QueueStats {
            pending: 0,
            pending_high: 0,
            pending_default: 0,
            pending_low: 0,
            processing: 0,
            scheduled: 0,
            dead: 0,
        })
    }
    async fn retry_dead(&self, _limit: usize) -> Result<u64, DomainError> {
        Ok(0)
    }
}

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

pub fn test_tenant() -> Tenant {
    Tenant {
        id: Uuid::new_v4(),
        email: "user@test.com".into(),
        api_key: "sk_test_key_123".into(),
        plan: TenantPlan::Pro,
        active: true,
        rate_limit_rpm: 60,
        concurrent_limit: 5,
        webhook_url: None,
        created_at: Utc::now(),
    }
}

pub fn test_app() -> Router {
    test_app_with_tenant_repo(Arc::new(MockTenantRepository::new()))
}

pub fn test_app_with_tenant(tenant: Tenant) -> Router {
    test_app_with_tenant_repo(Arc::new(MockTenantRepository::with_tenant(tenant)))
}

pub fn test_app_with_tenant_repo(tenant_repo: Arc<MockTenantRepository>) -> Router {
    let job_repo = Arc::new(MockJobRepository::new());
    let queue = Arc::new(MockJobQueue::new());
    let scrape_service = Arc::new(MockScrapeService {
        job_repo: job_repo.clone(),
        queue: queue.clone(),
    });
    let state = Arc::new(AppState {
        tenant_repo,
        job_repo,
        usage_repo: Arc::new(MockUsageRepository),
        queue,
        carrier_client: Arc::new(MockCarrierClient),
        scrape_service,
        parallax_config: ParallaxConfig {
            proxy_host: "localhost".into(),
            proxy_port: 1080,
            proxy_pass: "test_pass".into(),
        },
        admin_key: "admin_test_key".into(),
        metrics: Arc::new(MetricsCollector::new()),
    });
    router(state)
}

pub async fn body_json(body: Body) -> serde_json::Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}
