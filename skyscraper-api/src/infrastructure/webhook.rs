use reqwest::Client;
use serde::Serialize;
use tracing::{info, warn};
use uuid::Uuid;

use crate::domain::entity::ScrapeJobResult;

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
            .timeout(std::time::Duration::from_secs(5))
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

        match self.client.post(webhook_url).json(&payload).send().await {
            Ok(resp) => {
                info!("webhook delivered to {webhook_url}: {}", resp.status());
            }
            Err(e) => {
                warn!("webhook delivery failed to {webhook_url}: {e}");
            }
        }
    }
}
