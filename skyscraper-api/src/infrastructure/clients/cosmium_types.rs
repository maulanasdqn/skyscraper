use serde::{Deserialize, Serialize};

use crate::domain::entity::WorkflowStep;

#[derive(Serialize)]
pub struct CosmiumScrapeRequest<'a> {
    pub url: &'a str,
    pub profile: &'a str,
    pub screenshot: bool,
    pub headful: bool,
    pub wait_ms: u32,
    pub extract: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: &'a Option<String>,
    #[serde(skip_serializing_if = "<[WorkflowStep]>::is_empty")]
    pub workflow: &'a [WorkflowStep],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<&'a str>,
    pub include_html: bool,
    pub retries: u32,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub geo_sync: bool,
}

#[derive(Deserialize)]
pub struct CosmiumScrapeResponse {
    pub final_url: String,
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub html_length: usize,
    pub html: Option<String>,
    #[serde(default)]
    pub extracted: serde_json::Value,
    pub screenshot_base64: Option<String>,
    #[serde(default)]
    pub blocked: bool,
    pub proxy_used: Option<String>,
    #[serde(default)]
    pub profile_used: String,
    #[serde(default)]
    pub elapsed_ms: u64,
    #[serde(default)]
    pub attempts: i32,
}
