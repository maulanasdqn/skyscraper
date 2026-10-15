pub mod mock_job_repo;
pub mod mock_services;
pub mod mocks;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::Body;
use axum::Router;
use chrono::Utc;
use http_body_util::BodyExt;
use uuid::Uuid;

use skyscraper::adapter::http::{router, AppState, ParallaxConfig};
use skyscraper::domain::entity::{QueueStats, RateLimitInfo, Tenant, TenantPlan};
use skyscraper::domain::error::DomainError;
use skyscraper::domain::port::JobQueue;
use skyscraper::infrastructure::metrics::MetricsCollector;

pub use mock_job_repo::MockJobRepository;
pub use mock_services::{MockCarrierClient, MockResponseCache, MockScrapeService};
pub use mocks::{MockTenantRepository, MockUsageRepository};

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
    async fn schedule(&self, _: Uuid, _: f64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn dequeue(&self, _: f64) -> Result<Option<Uuid>, DomainError> {
        Ok(None)
    }
    async fn ack(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn move_to_dead(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn promote_scheduled(&self) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn reclaim_stale(&self, _: u64) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn check_rate_limit(&self, _: &Uuid, _: i32) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn rate_limit_info(
        &self,
        _: &Uuid,
        limit_rpm: i32,
    ) -> Result<RateLimitInfo, DomainError> {
        Ok(RateLimitInfo {
            limit: limit_rpm,
            remaining: i64::from(limit_rpm),
            reset_epoch: 0,
        })
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
    async fn retry_dead(&self, _: usize) -> Result<u64, DomainError> {
        Ok(0)
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
    let svc = Arc::new(MockScrapeService {
        job_repo: job_repo.clone(),
        queue: queue.clone(),
    });
    let state = Arc::new(AppState {
        tenant_repo,
        job_repo,
        usage_repo: Arc::new(MockUsageRepository),
        queue,
        carrier_client: Arc::new(MockCarrierClient),
        scrape_service: svc,
        response_cache: Arc::new(MockResponseCache),
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
