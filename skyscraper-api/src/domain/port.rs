use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::entity::{
    CarrierInfo, Job, QueueStats, ScrapeJobRequest, ScrapeJobResult, Tenant, TenantPlan,
    UsageRecord,
};
use super::error::DomainError;
use super::job_status::JobStatus;

pub type Result<T> = std::result::Result<T, DomainError>;

#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn find_by_api_key(&self, key: &str) -> Result<Option<Tenant>>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Tenant>>;
    async fn create(&self, email: &str, plan: TenantPlan) -> Result<Tenant>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Tenant>>;
    async fn update_plan(&self, id: Uuid, plan: TenantPlan) -> Result<()>;
    async fn set_active(&self, id: Uuid, active: bool) -> Result<()>;
    async fn set_webhook_url(&self, id: Uuid, url: Option<&str>) -> Result<()>;
    async fn regenerate_api_key(&self, id: Uuid) -> Result<String>;
}

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn create(
        &self,
        tenant_id: Uuid,
        request: &ScrapeJobRequest,
        priority: i16,
        max_retries: i32,
        scheduled_at: Option<DateTime<Utc>>,
    ) -> Result<Job>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Job>>;
    async fn list_by_tenant(
        &self,
        tenant_id: Uuid,
        status: Option<JobStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Job>>;
    async fn set_running(
        &self,
        id: Uuid,
        carrier_id: Option<Uuid>,
        profile_name: &str,
    ) -> Result<()>;
    async fn set_completed(&self, id: Uuid, result: &ScrapeJobResult) -> Result<()>;
    async fn set_failed(&self, id: Uuid, error: &str, attempts: i32) -> Result<()>;
    async fn set_status(&self, id: Uuid, status: JobStatus) -> Result<()>;
    async fn count_running_for_tenant(&self, tenant_id: Uuid) -> Result<i64>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}

#[async_trait]
pub trait UsageRepository: Send + Sync {
    async fn record(&self, tenant_id: Uuid, job_id: Uuid, bytes: i64) -> Result<()>;
    async fn total_for_tenant(&self, tenant_id: Uuid, since: DateTime<Utc>) -> Result<i64>;
    async fn list_for_tenant(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UsageRecord>>;
}

#[async_trait]
pub trait CarrierClient: Send + Sync {
    async fn list_carriers(&self) -> Result<Vec<CarrierInfo>>;
}

#[async_trait]
pub trait ScrapeClient: Send + Sync {
    async fn scrape(
        &self,
        url: &str,
        profile: &str,
        request: &ScrapeJobRequest,
        proxy_url: Option<&str>,
    ) -> Result<ScrapeJobResult>;
    async fn list_profiles(&self) -> Result<Vec<String>>;
}

#[async_trait]
pub trait ProxySelection: Send + Sync {
    async fn select_proxy(&self, request: &ScrapeJobRequest) -> Result<Option<String>>;
    async fn report_proxy_failure(&self, proxy_url: &str);
}

#[async_trait]
pub trait ProfileSelection: Send + Sync {
    async fn select_profile(&self, request: &ScrapeJobRequest) -> Result<String>;
    async fn report_profile_failure(&self, profile: &str);
}

#[async_trait]
pub trait ScrapeService: Send + Sync {
    async fn submit_job(
        &self,
        tenant: &Tenant,
        request: &ScrapeJobRequest,
        priority: i16,
        max_retries: i32,
        scheduled_at: Option<DateTime<Utc>>,
    ) -> Result<Job>;

    async fn get_job(&self, tenant_id: Uuid, job_id: Uuid) -> Result<Job>;

    async fn list_jobs(
        &self,
        tenant_id: Uuid,
        status: Option<JobStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Job>>;

    async fn cancel_job(&self, tenant_id: Uuid, job_id: Uuid) -> Result<()>;
}

#[async_trait]
pub trait JobQueue: Send + Sync {
    async fn enqueue(&self, job_id: Uuid, priority: i16) -> Result<()>;
    async fn schedule(&self, job_id: Uuid, run_at_epoch: f64) -> Result<()>;
    async fn dequeue(&self, timeout_secs: f64) -> Result<Option<Uuid>>;
    async fn ack(&self, job_id: Uuid) -> Result<()>;
    async fn move_to_dead(&self, job_id: Uuid) -> Result<()>;
    async fn promote_scheduled(&self) -> Result<u64>;
    async fn reclaim_stale(&self, max_age_secs: u64) -> Result<u64>;
    async fn check_rate_limit(&self, tenant_id: &Uuid, limit_rpm: i32) -> Result<bool>;
    async fn queue_stats(&self) -> Result<QueueStats>;
}
