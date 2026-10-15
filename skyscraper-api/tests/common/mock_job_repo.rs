use async_trait::async_trait;
use chrono::{DateTime, Utc};
use skyscraper::domain::entity::{Job, ScrapeJobRequest, ScrapeJobResult};
use skyscraper::domain::error::DomainError;
use skyscraper::domain::job_status::JobStatus;
use skyscraper::domain::port::JobRepository;
use std::sync::Mutex;
use uuid::Uuid;

pub struct MockJobRepository {
    pub jobs: Mutex<Vec<Job>>,
}

impl MockJobRepository {
    pub fn new() -> Self {
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

    async fn delete_completed_before(&self, _before: DateTime<Utc>) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn update_priority(&self, id: Uuid, priority: i16) -> Result<(), DomainError> {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(j) = jobs.iter_mut().find(|j| j.id == id) {
            j.priority = priority;
            Ok(())
        } else {
            Err(DomainError::NotFound("job".into()))
        }
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
            .filter(|j| tenant_id.is_none_or(|t| j.tenant_id == t))
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
            .filter(|j| tenant_id.is_none_or(|t| j.tenant_id == t))
            .count() as i64)
    }
}
