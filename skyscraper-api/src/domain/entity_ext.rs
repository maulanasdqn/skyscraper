use serde::{Deserialize, Serialize};

use super::entity::{ProfileStrategy, ProxyStrategy};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    pub limit: i32,
    pub remaining: i64,
    pub reset_epoch: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlRequest {
    pub start_url: String,
    #[serde(default = "default_max_pages")]
    pub max_pages: u32,
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    #[serde(default)]
    pub extract: Vec<String>,
    #[serde(default)]
    pub screenshot: bool,
    #[serde(default)]
    pub include_html: bool,
    pub geo: Option<String>,
    #[serde(default)]
    pub proxy_strategy: ProxyStrategy,
    #[serde(default)]
    pub profile_strategy: ProfileStrategy,
}

fn default_max_pages() -> u32 {
    10
}

fn default_max_depth() -> u32 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResponse {
    pub crawl_id: uuid::Uuid,
    pub job_ids: Vec<uuid::Uuid>,
    pub pages_discovered: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub keys: u64,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub pending: u64,
    pub pending_high: u64,
    pub pending_default: u64,
    pub pending_low: u64,
    pub processing: u64,
    pub scheduled: u64,
    pub dead: u64,
}
