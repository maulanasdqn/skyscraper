use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::job_status::JobStatus;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "tenant_plan", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TenantPlan {
    Free,
    Starter,
    Pro,
    Business,
}

impl TenantPlan {
    pub fn monthly_request_limit(&self) -> Option<i64> {
        match self {
            Self::Free => Some(1_000),
            Self::Starter => Some(10_000),
            Self::Pro => Some(100_000),
            Self::Business => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tenant {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub api_key: String,
    pub plan: TenantPlan,
    pub active: bool,
    pub rate_limit_rpm: i32,
    pub concurrent_limit: i32,
    pub webhook_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Job {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub status: JobStatus,
    pub priority: i16,
    pub request: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub carrier_id: Option<Uuid>,
    pub profile_name: Option<String>,
    pub attempts: i32,
    pub max_retries: i32,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeJobRequest {
    pub url: String,
    #[serde(default)]
    pub extract: Vec<String>,
    pub script: Option<String>,
    #[serde(default)]
    pub workflow: Vec<WorkflowStep>,
    #[serde(default)]
    pub screenshot: bool,
    #[serde(default)]
    pub include_html: bool,
    #[serde(default = "default_wait_ms")]
    pub wait_ms: u32,
    #[serde(default)]
    pub headful: bool,
    pub geo: Option<String>,
    #[serde(default)]
    pub proxy_strategy: ProxyStrategy,
    #[serde(default)]
    pub profile_strategy: ProfileStrategy,
}

fn default_wait_ms() -> u32 {
    3000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowStep {
    Click {
        selector: String,
    },
    Delay {
        duration_ms: u32,
    },
    Extract {
        name: String,
        selector: String,
        limit: Option<u32>,
    },
    Script {
        code: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProxyStrategy {
    #[default]
    Auto,
    Carrier {
        name: String,
    },
    None,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProfileStrategy {
    #[default]
    Auto,
    Named {
        name: String,
    },
    Random,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeJobResult {
    pub final_url: String,
    pub http_status: i32,
    pub html_length: usize,
    pub html: Option<String>,
    pub extracted: serde_json::Value,
    pub screenshot_base64: Option<String>,
    pub blocked: bool,
    pub proxy_used: Option<String>,
    pub profile_used: String,
    pub elapsed_ms: u64,
    pub attempts: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UsageRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub job_id: Uuid,
    pub bytes_consumed: i64,
    pub billed: bool,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarrierInfo {
    pub id: Uuid,
    pub name: String,
    pub country: String,
    pub upstream_addr: String,
    pub online: bool,
}

pub use super::entity_ext::{CacheStats, CrawlRequest, CrawlResponse, QueueStats, RateLimitInfo};
