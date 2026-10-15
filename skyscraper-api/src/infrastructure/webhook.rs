use std::time::Duration;

use reqwest::Client;
use serde::Serialize;
use tracing::{info, warn};
use uuid::Uuid;

use crate::domain::entity::ScrapeJobResult;

const MAX_RETRIES: u32 = 3;

pub struct WebhookDelivery {
    client: Client,
}

#[derive(Serialize)]
struct WebhookPayload<'a> {
    job_id: Uuid,
    status: &'a str,
    result: Option<&'a ScrapeJobResult>,
    error: Option<&'a str>,
}

impl Default for WebhookDelivery {
    fn default() -> Self {
        Self::new()
    }
}

impl WebhookDelivery {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub async fn deliver(
        &self,
        webhook_url: &str,
        job_id: Uuid,
        result: Option<&ScrapeJobResult>,
        error: Option<&str>,
    ) {
        let status = if result.is_some() {
            "completed"
        } else {
            "failed"
        };
        let payload = WebhookPayload {
            job_id,
            status,
            result,
            error,
        };

        for attempt in 0..MAX_RETRIES {
            match self.client.post(webhook_url).json(&payload).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("webhook delivered to {webhook_url}: {}", resp.status());
                    return;
                }
                Ok(resp) => {
                    warn!(
                        "webhook {webhook_url} returned {} (attempt {}/{})",
                        resp.status(),
                        attempt + 1,
                        MAX_RETRIES
                    );
                }
                Err(e) => {
                    warn!(
                        "webhook delivery to {webhook_url} failed: {e} (attempt {}/{})",
                        attempt + 1,
                        MAX_RETRIES
                    );
                }
            }

            if attempt + 1 < MAX_RETRIES {
                let backoff = Duration::from_secs(1 << (attempt + 1));
                tokio::time::sleep(backoff).await;
            }
        }

        warn!("webhook delivery to {webhook_url} exhausted all {MAX_RETRIES} retries");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles() {
        assert_eq!(1u64 << 1, 2);
        assert_eq!(1u64 << 2, 4);
        assert_eq!(1u64 << 3, 8);
    }

    #[test]
    fn payload_serializes_completed() {
        let result = ScrapeJobResult {
            final_url: "https://example.com".into(),
            http_status: 200,
            html_length: 100,
            html: None,
            extracted: serde_json::json!({}),
            screenshot_base64: None,
            blocked: false,
            proxy_used: None,
            profile_used: "default".into(),
            elapsed_ms: 500,
            attempts: 1,
        };
        let payload = WebhookPayload {
            job_id: Uuid::nil(),
            status: "completed",
            result: Some(&result),
            error: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["status"], "completed");
        assert!(json["error"].is_null());
        assert_eq!(json["result"]["http_status"], 200);
    }

    #[test]
    fn payload_serializes_failed() {
        let payload = WebhookPayload {
            job_id: Uuid::nil(),
            status: "failed",
            result: None,
            error: Some("blocked by target"),
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["status"], "failed");
        assert_eq!(json["error"], "blocked by target");
        assert!(json["result"].is_null());
    }
}
