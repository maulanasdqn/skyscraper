use super::mocks::{MockJobQueue, MockJobRepository, MockUsageRepository};
use crate::application::scrape_service::ScrapeServiceImpl;
use crate::domain::entity::*;
use crate::domain::error::DomainError;
use crate::domain::port::ScrapeService;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

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

fn make_service() -> ScrapeServiceImpl {
    let queue = Arc::new(MockJobQueue::new(true));
    let repo = Arc::new(MockJobRepository::new(0));
    let usage = Arc::new(MockUsageRepository);
    ScrapeServiceImpl::new(repo, queue, usage)
}

#[tokio::test]
async fn submit_job_private_ip_rejected() {
    let svc = make_service();
    let tenant = test_tenant();
    let err = svc
        .submit_job(&tenant, &test_request("http://127.0.0.1/admin"), 5, 2, None)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::InvalidInput(_)));
    let err = svc
        .submit_job(&tenant, &test_request("http://10.0.0.1:8080"), 5, 2, None)
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::InvalidInput(_)));
}

#[tokio::test]
async fn submit_job_invalid_scheme_rejected() {
    let svc = make_service();
    let err = svc
        .submit_job(
            &test_tenant(),
            &test_request("ftp://example.com/file"),
            5,
            2,
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::InvalidInput(_)));
}

#[tokio::test]
async fn list_jobs_filters_by_tenant() {
    let svc = make_service();
    let (tenant_a, tenant_b) = (test_tenant(), test_tenant());
    let req = test_request("https://example.com");
    svc.submit_job(&tenant_a, &req, 5, 2, None).await.unwrap();
    svc.submit_job(&tenant_a, &req, 5, 2, None).await.unwrap();
    svc.submit_job(&tenant_b, &req, 5, 2, None).await.unwrap();
    assert_eq!(
        svc.list_jobs(tenant_a.id, None, 100, 0)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        svc.list_jobs(tenant_b.id, None, 100, 0)
            .await
            .unwrap()
            .len(),
        1
    );
}
