use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use skyscraper::domain::entity::{
    Job, ScrapeJobRequest, ScrapeJobResult, Tenant, TenantPlan, UsageRecord,
};
use skyscraper::domain::error::DomainError;
use skyscraper::domain::job_status::JobStatus;
use skyscraper::domain::port::{JobRepository, TenantRepository, UsageRepository};

pub struct MockTenantRepository {
    tenants: Mutex<Vec<Tenant>>,
}

impl MockTenantRepository {
    pub fn new() -> Self {
        Self {
            tenants: Mutex::new(Vec::new()),
        }
    }

    pub fn with_tenant(tenant: Tenant) -> Self {
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
}

pub struct MockUsageRepository;

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
