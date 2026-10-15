use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entity::*;
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;
use crate::domain::port::JobRepository;

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

    async fn set_running(&self, _: Uuid, _: Option<Uuid>, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_completed(&self, _: Uuid, _: &ScrapeJobResult) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_failed(&self, _: Uuid, _: &str, _: i32) -> Result<(), DomainError> {
        Ok(())
    }

    async fn set_status(&self, id: Uuid, status: JobStatus) -> Result<(), DomainError> {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.iter_mut().find(|j| j.id == id) {
            job.status = status;
        }
        Ok(())
    }

    async fn count_running_for_tenant(&self, _: Uuid) -> Result<i64, DomainError> {
        Ok(self.running_count)
    }

    async fn count_by_tenant(
        &self,
        tenant_id: Uuid,
        status: Option<JobStatus>,
    ) -> Result<i64, DomainError> {
        let jobs = self.jobs.lock().unwrap();
        Ok(jobs
            .iter()
            .filter(|j| j.tenant_id == tenant_id)
            .filter(|j| status.is_none_or(|s| j.status == s))
            .count() as i64)
    }

    async fn update_priority(&self, id: Uuid, priority: i16) -> Result<(), DomainError> {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.iter_mut().find(|j| j.id == id) {
            job.priority = priority;
            Ok(())
        } else {
            Err(DomainError::NotFound(format!("job {id}")))
        }
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.retain(|j| j.id != id);
        Ok(())
    }

    async fn delete_completed_before(&self, _: DateTime<Utc>) -> Result<u64, DomainError> {
        Ok(0)
    }

    async fn list_all(
        &self,
        status: Option<JobStatus>,
        tenant_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Job>, DomainError> {
        let jobs = self.jobs.lock().unwrap();
        Ok(jobs
            .iter()
            .filter(|j| status.is_none_or(|s| j.status == s))
            .filter(|j| tenant_id.is_none_or(|tid| j.tenant_id == tid))
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn count_all(
        &self,
        status: Option<JobStatus>,
        tenant_id: Option<Uuid>,
    ) -> Result<i64, DomainError> {
        let jobs = self.jobs.lock().unwrap();
        Ok(jobs
            .iter()
            .filter(|j| status.is_none_or(|s| j.status == s))
            .filter(|j| tenant_id.is_none_or(|tid| j.tenant_id == tid))
            .count() as i64)
    }
}
