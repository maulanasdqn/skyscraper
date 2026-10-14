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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entity::*;
    use crate::domain::job_status::JobStatus;
    use std::sync::Mutex;

    struct MockJobQueue {
        enqueued: Arc<Mutex<Vec<Uuid>>>,
        scheduled: Arc<Mutex<Vec<(Uuid, f64)>>>,
        rate_limit_result: bool,
    }

    impl MockJobQueue {
        fn new(rate_limit_result: bool) -> Self {
            Self {
                enqueued: Arc::new(Mutex::new(vec![])),
                scheduled: Arc::new(Mutex::new(vec![])),
                rate_limit_result,
            }
        }
    }

    #[async_trait]
    impl JobQueue for MockJobQueue {
        async fn enqueue(&self, job_id: Uuid, _priority: i16) -> Result<(), DomainError> {
            self.enqueued.lock().unwrap().push(job_id);
            Ok(())
        }
        async fn schedule(&self, job_id: Uuid, run_at_epoch: f64) -> Result<(), DomainError> {
            self.scheduled.lock().unwrap().push((job_id, run_at_epoch));
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
            Ok(self.rate_limit_result)
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
    }

    struct MockJobRepository {
        jobs: Arc<Mutex<Vec<Job>>>,
        running_count: i64,
    }

    impl MockJobRepository {
        fn new(running_count: i64) -> Self {
            Self {
                jobs: Arc::new(Mutex::new(vec![])),
                running_count,
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
                status: JobStatus::Pending,
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
            Ok(self
                .jobs
                .lock()
                .unwrap()
                .iter()
                .find(|j| j.id == id)
                .cloned())
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
            _id: Uuid,
            _carrier_id: Option<Uuid>,
            _profile_name: &str,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn set_completed(
            &self,
            _id: Uuid,
            _result: &ScrapeJobResult,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn set_failed(
            &self,
            _id: Uuid,
            _error: &str,
            _attempts: i32,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn set_status(&self, id: Uuid, status: JobStatus) -> Result<(), DomainError> {
            let mut jobs = self.jobs.lock().unwrap();
            if let Some(job) = jobs.iter_mut().find(|j| j.id == id) {
                job.status = status;
            }
            Ok(())
        }

        async fn count_running_for_tenant(&self, _tenant_id: Uuid) -> Result<i64, DomainError> {
            Ok(self.running_count)
        }

        async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
            let mut jobs = self.jobs.lock().unwrap();
            jobs.retain(|j| j.id != id);
            Ok(())
        }
    }

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
}
