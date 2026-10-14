use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use chrono::{DateTime, Utc};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use skyscraper::adapter::http::{router, AppState, ParallaxConfig};
use skyscraper::domain::entity::{
    CarrierInfo, Job, QueueStats, ScrapeJobRequest, ScrapeJobResult, Tenant, TenantPlan,
    UsageRecord,
};
use skyscraper::domain::error::DomainError;
use skyscraper::domain::job_status::JobStatus;
use skyscraper::domain::port::{
    CarrierClient, JobQueue, JobRepository, ScrapeService, TenantRepository, UsageRepository,
};

// --- Mock implementations ---

struct MockTenantRepository {
    tenants: Mutex<Vec<Tenant>>,
}

impl MockTenantRepository {
    fn new() -> Self {
        Self {
            tenants: Mutex::new(Vec::new()),
        }
    }

    fn with_tenant(tenant: Tenant) -> Self {
        Self {
            tenants: Mutex::new(vec![tenant]),
        }
    }
}

#[async_trait]
impl TenantRepository for MockTenantRepository {
    async fn find_by_api_key(&self, key: &str) -> Result<Option<Tenant>, DomainError> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants.iter().find(|t| t.api_key == key).cloned())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Tenant>, DomainError> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants.iter().find(|t| t.id == id).cloned())
    }

    async fn create(&self, email: &str, plan: TenantPlan) -> Result<Tenant, DomainError> {
        let tenant = Tenant {
            id: Uuid::new_v4(),
            email: email.to_string(),
            api_key: format!("sk_{}", Uuid::new_v4()),
            plan,
            active: true,
            rate_limit_rpm: 60,
            concurrent_limit: 5,
            webhook_url: None,
            created_at: Utc::now(),
        };
        self.tenants.lock().unwrap().push(tenant.clone());
        Ok(tenant)
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Tenant>, DomainError> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn update_plan(&self, id: Uuid, plan: TenantPlan) -> Result<(), DomainError> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(t) = tenants.iter_mut().find(|t| t.id == id) {
            t.plan = plan;
            Ok(())
        } else {
            Err(DomainError::NotFound("tenant".into()))
        }
    }

    async fn set_active(&self, id: Uuid, active: bool) -> Result<(), DomainError> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(t) = tenants.iter_mut().find(|t| t.id == id) {
            t.active = active;
            Ok(())
        } else {
            Err(DomainError::NotFound("tenant".into()))
        }
    }

    async fn set_webhook_url(&self, id: Uuid, url: Option<&str>) -> Result<(), DomainError> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(t) = tenants.iter_mut().find(|t| t.id == id) {
            t.webhook_url = url.map(String::from);
            Ok(())
        } else {
            Err(DomainError::NotFound("tenant".into()))
        }
    }

    async fn regenerate_api_key(&self, id: Uuid) -> Result<String, DomainError> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(t) = tenants.iter_mut().find(|t| t.id == id) {
            t.api_key = format!("sk_{}", Uuid::new_v4());
            Ok(t.api_key.clone())
        } else {
            Err(DomainError::NotFound("tenant".into()))
        }
    }
}

struct MockJobRepository {
    jobs: Mutex<Vec<Job>>,
}

impl MockJobRepository {
    fn new() -> Self {
        Self {
            jobs: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl JobRepository for MockJobRepository {
    async fn create(
        &self,
        tenant_id: Uuid,
        request: &ScrapeJobRequest,
        priority: i16,
        max_retries: i32,
        scheduled_at: Option<DateTime<Utc>>,
    ) -> Result<Job, DomainError> {
        let job = Job {
            id: Uuid::new_v4(),
            tenant_id,
            status: if scheduled_at.is_some() {
                JobStatus::Scheduled
            } else {
                JobStatus::Pending
            },
            priority,
            request: serde_json::to_value(request).unwrap(),
            result: None,
            carrier_id: None,
            profile_name: None,
            attempts: 0,
            max_retries,
            scheduled_at,
            started_at: None,
            completed_at: None,
            error: None,
            created_at: Utc::now(),
        };
        self.jobs.lock().unwrap().push(job.clone());
        Ok(job)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Job>, DomainError> {
        let jobs = self.jobs.lock().unwrap();
        Ok(jobs.iter().find(|j| j.id == id).cloned())
    }

    async fn list_by_tenant(
        &self,
        tenant_id: Uuid,
        status: Option<JobStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Job>, DomainError> {
        let jobs = self.jobs.lock().unwrap();
        Ok(jobs
            .iter()
            .filter(|j| j.tenant_id == tenant_id)
            .filter(|j| status.is_none_or(|s| j.status == s))
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn set_running(
        &self,
        id: Uuid,
        carrier_id: Option<Uuid>,
        profile_name: &str,
    ) -> Result<(), DomainError> {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(j) = jobs.iter_mut().find(|j| j.id == id) {
            j.status = JobStatus::Running;
            j.carrier_id = carrier_id;
            j.profile_name = Some(profile_name.to_string());
            j.started_at = Some(Utc::now());
            Ok(())
        } else {
            Err(DomainError::NotFound("job".into()))
        }
    }

    async fn set_completed(&self, id: Uuid, result: &ScrapeJobResult) -> Result<(), DomainError> {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(j) = jobs.iter_mut().find(|j| j.id == id) {
            j.status = JobStatus::Completed;
            j.result = Some(serde_json::to_value(result).unwrap());
            j.completed_at = Some(Utc::now());
            Ok(())
        } else {
            Err(DomainError::NotFound("job".into()))
        }
    }

    async fn set_failed(&self, id: Uuid, error: &str, attempts: i32) -> Result<(), DomainError> {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(j) = jobs.iter_mut().find(|j| j.id == id) {
            j.error = Some(error.to_string());
            j.attempts = attempts;
            Ok(())
        } else {
            Err(DomainError::NotFound("job".into()))
        }
    }

    async fn set_status(&self, id: Uuid, status: JobStatus) -> Result<(), DomainError> {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(j) = jobs.iter_mut().find(|j| j.id == id) {
            j.status = status;
            Ok(())
        } else {
            Err(DomainError::NotFound("job".into()))
        }
    }

    async fn count_running_for_tenant(&self, tenant_id: Uuid) -> Result<i64, DomainError> {
        let jobs = self.jobs.lock().unwrap();
        Ok(jobs
            .iter()
            .filter(|j| j.tenant_id == tenant_id && j.status == JobStatus::Running)
            .count() as i64)
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut jobs = self.jobs.lock().unwrap();
        let len_before = jobs.len();
        jobs.retain(|j| j.id != id);
        if jobs.len() == len_before {
            Err(DomainError::NotFound("job".into()))
        } else {
            Ok(())
        }
    }
}

struct MockUsageRepository;

#[async_trait]
impl UsageRepository for MockUsageRepository {
    async fn record(
        &self,
        _tenant_id: Uuid,
        _job_id: Uuid,
        _bytes: i64,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn total_for_tenant(
        &self,
        _tenant_id: Uuid,
        _since: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        Ok(0)
    }

    async fn list_for_tenant(
        &self,
        _tenant_id: Uuid,
        _limit: i64,
        _offset: i64,
    ) -> Result<Vec<UsageRecord>, DomainError> {
        Ok(vec![])
    }
}

struct MockJobQueue {
    enqueued: Mutex<Vec<Uuid>>,
}

impl MockJobQueue {
    fn new() -> Self {
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

struct MockScrapeService {
    job_repo: Arc<MockJobRepository>,
    queue: Arc<MockJobQueue>,
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

struct MockCarrierClient;

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

fn test_tenant() -> Tenant {
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

fn test_app() -> Router {
    test_app_with_tenant_repo(Arc::new(MockTenantRepository::new()))
}

fn test_app_with_tenant(tenant: Tenant) -> Router {
    test_app_with_tenant_repo(Arc::new(MockTenantRepository::with_tenant(tenant)))
}

fn test_app_with_tenant_repo(tenant_repo: Arc<MockTenantRepository>) -> Router {
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
    });
    router(state)
}

async fn body_json(body: Body) -> serde_json::Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

// --- Health ---

#[tokio::test]
async fn health_returns_ok() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["status"], "ok");
}

// --- Auth ---

#[tokio::test]
async fn missing_api_key_returns_401() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_api_key_returns_401() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/jobs")
                .header("x-api-key", "sk_nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn inactive_tenant_returns_403() {
    let mut tenant = test_tenant();
    tenant.active = false;

    let app = test_app_with_tenant(tenant.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/jobs")
                .header("x-api-key", &tenant.api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// --- Tenant admin ---

#[tokio::test]
async fn admin_create_tenant() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/tenants")
                .header("x-api-key", "admin_test_key")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email": "new@test.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["email"], "new@test.com");
    assert!(body["api_key"].as_str().unwrap().starts_with("sk_"));
    assert_eq!(body["active"], true);
}

#[tokio::test]
async fn admin_list_tenants() {
    let tenant = test_tenant();
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/tenants")
                .header("x-api-key", "admin_test_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["email"], "user@test.com");
}

#[tokio::test]
async fn admin_update_tenant() {
    let tenant = test_tenant();
    let id = tenant.id;
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/admin/tenants/{id}"))
                .header("x-api-key", "admin_test_key")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"active": false}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["updated"], true);
}

#[tokio::test]
async fn admin_regenerate_key() {
    let tenant = test_tenant();
    let id = tenant.id;
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/admin/tenants/{id}/regenerate-key"))
                .header("x-api-key", "admin_test_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert!(body["api_key"].as_str().unwrap().starts_with("sk_"));
}

#[tokio::test]
async fn admin_endpoints_reject_non_admin_key() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/tenants")
                .header("x-api-key", "sk_not_admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_endpoints_reject_missing_key() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/tenants")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// --- Jobs ---

#[tokio::test]
async fn list_jobs_empty() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/jobs")
                .header("x-api-key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn get_job_not_found() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let app = test_app_with_tenant(tenant);

    let fake_id = Uuid::new_v4();
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/jobs/{fake_id}"))
                .header("x-api-key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_job_and_list() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let tenant_id = tenant.id;
    let tenant_repo = Arc::new(MockTenantRepository::with_tenant(tenant));
    let job_repo = Arc::new(MockJobRepository::new());

    let queue = Arc::new(MockJobQueue::new());
    let scrape_service = Arc::new(MockScrapeService {
        job_repo: job_repo.clone(),
        queue: queue.clone(),
    });
    let state = Arc::new(AppState {
        tenant_repo,
        job_repo: job_repo.clone(),
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
    });
    let app = router(state);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("x-api-key", &api_key)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"url": "https://example.com", "extract": [".title"]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp.into_body()).await;
    let job_id = body["id"].as_str().unwrap();
    assert_eq!(body["status"], "pending");
    assert_eq!(body["tenant_id"], tenant_id.to_string());

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/jobs")
                .header("x-api-key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_str().unwrap(), job_id);
}

#[tokio::test]
async fn create_job_empty_url_rejected() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("x-api-key", &api_key)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"url": ""}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn cancel_pending_job() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let tenant_repo = Arc::new(MockTenantRepository::with_tenant(tenant));
    let job_repo = Arc::new(MockJobRepository::new());

    let queue = Arc::new(MockJobQueue::new());
    let scrape_service = Arc::new(MockScrapeService {
        job_repo: job_repo.clone(),
        queue: queue.clone(),
    });
    let state = Arc::new(AppState {
        tenant_repo,
        job_repo: job_repo.clone(),
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
    });
    let app = router(state);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("x-api-key", &api_key)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"url": "https://example.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp.into_body()).await;
    let job_id = body["id"].as_str().unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/jobs/{job_id}"))
                .header("x-api-key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["cancelled"], true);
}

// --- Batch ---

#[tokio::test]
async fn batch_create_jobs() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs/batch")
                .header("x-api-key", &api_key)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"jobs": [
                        {"url": "https://a.com"},
                        {"url": "https://b.com"},
                        {"url": "https://c.com"}
                    ]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["job_ids"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn batch_rejects_empty() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs/batch")
                .header("x-api-key", &api_key)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"jobs": []}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// --- Usage ---

#[tokio::test]
async fn usage_returns_zero() {
    let tenant = test_tenant();
    let api_key = tenant.api_key.clone();
    let app = test_app_with_tenant(tenant);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/usage")
                .header("x-api-key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["total_bytes"], 0);
}

// --- Admin queue/carriers ---

#[tokio::test]
async fn admin_queue_stats() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/queue/stats")
                .header("x-api-key", "admin_test_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["pending"], 0);
    assert_eq!(body["processing"], 0);
    assert_eq!(body["dead"], 0);
}

#[tokio::test]
async fn admin_list_carriers() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/carriers")
                .header("x-api-key", "admin_test_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    let carriers = body.as_array().unwrap();
    assert_eq!(carriers.len(), 1);
    assert_eq!(carriers[0]["name"], "test-carrier");
    assert_eq!(carriers[0]["online"], true);
}

// --- Readiness ---

#[tokio::test]
async fn readiness_returns_degraded_without_deps() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Mock repos succeed, mock queue succeeds → should be ready
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["status"], "ready");
    assert!(body["checks"]["postgres"]["latency_ms"].is_number());
    assert!(body["checks"]["redis"]["latency_ms"].is_number());
}

// --- Request ID middleware ---

#[tokio::test]
async fn response_includes_request_id() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let request_id = resp.headers().get("x-request-id");
    assert!(request_id.is_some(), "response must include x-request-id");
    let id_str = request_id.unwrap().to_str().unwrap();
    assert!(
        uuid::Uuid::parse_str(id_str).is_ok(),
        "x-request-id should be a valid UUID, got: {id_str}"
    );
}

#[tokio::test]
async fn request_id_echoed_if_provided() {
    let app = test_app();
    let custom_id = "my-custom-request-id-12345";
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .header("x-request-id", custom_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let returned_id = resp
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(returned_id, custom_id);
}
