use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entity::*;
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;
use crate::domain::port::{JobQueue, JobRepository};

pub struct MockJobQueue {
    pub enqueued: Arc<Mutex<Vec<Uuid>>>,
    pub scheduled: Arc<Mutex<Vec<(Uuid, f64)>>>,
    rate_limit_result: bool,
}

impl MockJobQueue {
    pub fn new(rate_limit_result: bool) -> Self {
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
    async fn retry_dead(&self, _limit: usize) -> Result<u64, DomainError> {
        Ok(0)
    }
}

pub struct MockJobRepository {
    pub jobs: Arc<Mutex<Vec<Job>>>,
    running_count: i64,
}

impl MockJobRepository {
    pub fn new(running_count: i64) -> Self {
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

    async fn set_completed(&self, _id: Uuid, _result: &ScrapeJobResult) -> Result<(), DomainError> {
        Ok(())
    }

    async fn set_failed(&self, _id: Uuid, _error: &str, _attempts: i32) -> Result<(), DomainError> {
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
