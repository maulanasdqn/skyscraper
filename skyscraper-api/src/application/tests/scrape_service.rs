use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use super::mocks::{MockJobQueue, MockJobRepository};
use crate::application::scrape_service::ScrapeServiceImpl;
use crate::domain::entity::*;
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;
use crate::domain::port::ScrapeService;

fn test_tenant() -> Tenant {
    Tenant {
        id: Uuid::new_v4(),
        email: "test@test.com".into(),
        api_key: "sk_test".into(),
        plan: TenantPlan::Pro,
        active: true,
        rate_limit_rpm: 60,
        concurrent_limit: 5,
        webhook_url: None,
        created_at: Utc::now(),
    }
}

fn test_request(url: &str) -> ScrapeJobRequest {
    ScrapeJobRequest {
        url: url.to_string(),
        extract: vec![],
        script: None,
        workflow: vec![],
        screenshot: false,
        include_html: false,
        wait_ms: 3000,
        headful: false,
        geo: None,
        proxy_strategy: ProxyStrategy::Auto,
        profile_strategy: ProfileStrategy::Auto,
    }
}

fn make_service(
    rate_limit_result: bool,
    running_count: i64,
) -> (ScrapeServiceImpl, Arc<MockJobQueue>, Arc<MockJobRepository>) {
    let queue = Arc::new(MockJobQueue::new(rate_limit_result));
    let repo = Arc::new(MockJobRepository::new(running_count));
    let svc = ScrapeServiceImpl::new(repo.clone(), queue.clone());
    (svc, queue, repo)
}

#[tokio::test]
async fn submit_job_success() {
    let (svc, queue, _repo) = make_service(true, 0);
    let tenant = test_tenant();
    let req = test_request("https://example.com");

    let job = svc.submit_job(&tenant, &req, 5, 2, None).await.unwrap();

    assert_eq!(job.tenant_id, tenant.id);
    assert_eq!(job.status, JobStatus::Pending);
    assert_eq!(queue.enqueued.lock().unwrap().len(), 1);
    assert_eq!(queue.enqueued.lock().unwrap()[0], job.id);
}

#[tokio::test]
async fn submit_job_empty_url_rejected() {
    let (svc, _queue, _repo) = make_service(true, 0);
    let tenant = test_tenant();
    let req = test_request("");

    let err = svc.submit_job(&tenant, &req, 5, 2, None).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidInput(_)));
}

#[tokio::test]
async fn submit_job_rate_limited() {
    let (svc, _queue, _repo) = make_service(false, 0);
    let tenant = test_tenant();
    let req = test_request("https://example.com");

    let err = svc.submit_job(&tenant, &req, 5, 2, None).await.unwrap_err();
    assert!(matches!(err, DomainError::RateLimitExceeded));
}

#[tokio::test]
async fn submit_job_concurrency_exceeded() {
    let (svc, _queue, _repo) = make_service(true, 5);
    let tenant = test_tenant();
    let req = test_request("https://example.com");

    let err = svc.submit_job(&tenant, &req, 5, 2, None).await.unwrap_err();
    assert!(matches!(err, DomainError::ConcurrencyLimitExceeded));
}

#[tokio::test]
async fn submit_job_scheduled() {
    let (svc, queue, _repo) = make_service(true, 0);
    let tenant = test_tenant();
    let req = test_request("https://example.com");
    let future = Utc::now() + chrono::Duration::hours(1);

    let job = svc
        .submit_job(&tenant, &req, 5, 2, Some(future))
        .await
        .unwrap();

    assert!(queue.enqueued.lock().unwrap().is_empty());
    let scheduled = queue.scheduled.lock().unwrap();
    assert_eq!(scheduled.len(), 1);
    assert_eq!(scheduled[0].0, job.id);
}

#[tokio::test]
async fn get_job_success() {
    let (svc, _queue, _repo) = make_service(true, 0);
    let tenant = test_tenant();
    let req = test_request("https://example.com");

    let job = svc.submit_job(&tenant, &req, 5, 2, None).await.unwrap();
    let fetched = svc.get_job(tenant.id, job.id).await.unwrap();

    assert_eq!(fetched.id, job.id);
    assert_eq!(fetched.tenant_id, tenant.id);
}

#[tokio::test]
async fn get_job_wrong_tenant() {
    let (svc, _queue, _repo) = make_service(true, 0);
    let tenant = test_tenant();
    let req = test_request("https://example.com");

    let job = svc.submit_job(&tenant, &req, 5, 2, None).await.unwrap();
    let other_tenant_id = Uuid::new_v4();

    let err = svc.get_job(other_tenant_id, job.id).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn get_job_not_found() {
    let (svc, _queue, _repo) = make_service(true, 0);
    let err = svc
        .get_job(Uuid::new_v4(), Uuid::new_v4())
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn cancel_job_pending() {
    let (svc, _queue, repo) = make_service(true, 0);
    let tenant = test_tenant();
    let req = test_request("https://example.com");

    let job = svc.submit_job(&tenant, &req, 5, 2, None).await.unwrap();
    svc.cancel_job(tenant.id, job.id).await.unwrap();

    assert!(repo.jobs.lock().unwrap().is_empty());
}

#[tokio::test]
async fn cancel_job_running_rejected() {
    let (svc, _queue, repo) = make_service(true, 0);
    let tenant = test_tenant();
    let req = test_request("https://example.com");

    let job = svc.submit_job(&tenant, &req, 5, 2, None).await.unwrap();
    repo.jobs.lock().unwrap().iter_mut().for_each(|j| {
        if j.id == job.id {
            j.status = JobStatus::Running;
        }
    });

    let err = svc.cancel_job(tenant.id, job.id).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidInput(_)));
}

#[tokio::test]
async fn submit_job_private_ip_rejected() {
    let (svc, _queue, _repo) = make_service(true, 0);
    let tenant = test_tenant();

    let req = test_request("http://127.0.0.1/admin");
    let err = svc.submit_job(&tenant, &req, 5, 2, None).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidInput(_)));

    let req = test_request("http://10.0.0.1:8080");
    let err = svc.submit_job(&tenant, &req, 5, 2, None).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidInput(_)));
}

#[tokio::test]
async fn submit_job_invalid_scheme_rejected() {
    let (svc, _queue, _repo) = make_service(true, 0);
    let tenant = test_tenant();
    let req = test_request("ftp://example.com/file");

    let err = svc.submit_job(&tenant, &req, 5, 2, None).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidInput(_)));
}

#[tokio::test]
async fn list_jobs_filters_by_tenant() {
    let (svc, _queue, _repo) = make_service(true, 0);
    let tenant_a = test_tenant();
    let tenant_b = test_tenant();
    let req = test_request("https://example.com");

    svc.submit_job(&tenant_a, &req, 5, 2, None).await.unwrap();
    svc.submit_job(&tenant_a, &req, 5, 2, None).await.unwrap();
    svc.submit_job(&tenant_b, &req, 5, 2, None).await.unwrap();

    let a_jobs = svc.list_jobs(tenant_a.id, None, 100, 0).await.unwrap();
    let b_jobs = svc.list_jobs(tenant_b.id, None, 100, 0).await.unwrap();

    assert_eq!(a_jobs.len(), 2);
    assert_eq!(b_jobs.len(), 1);
}
