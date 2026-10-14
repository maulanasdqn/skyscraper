use std::sync::Arc;
use tokio::sync::watch;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::domain::entity::Job;
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;
use crate::domain::port::{
    JobQueue, JobRepository, ProfileSelection, ProxySelection, ResponseCache, ScrapeClient,
    TenantRepository, UsageRepository,
};
use crate::infrastructure::webhook::WebhookDelivery;

pub struct Worker {
    queue: Arc<dyn JobQueue>,
    job_repo: Arc<dyn JobRepository>,
    tenant_repo: Arc<dyn TenantRepository>,
    usage_repo: Arc<dyn UsageRepository>,
    proxy_selector: Arc<dyn ProxySelection>,
    profile_rotator: Arc<dyn ProfileSelection>,
    scrape_client: Arc<dyn ScrapeClient>,
    response_cache: Arc<dyn ResponseCache>,
    webhook: Arc<WebhookDelivery>,
    shutdown_rx: watch::Receiver<bool>,
}

impl Worker {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        queue: Arc<dyn JobQueue>,
        job_repo: Arc<dyn JobRepository>,
        tenant_repo: Arc<dyn TenantRepository>,
        usage_repo: Arc<dyn UsageRepository>,
        proxy_selector: Arc<dyn ProxySelection>,
        profile_rotator: Arc<dyn ProfileSelection>,
        scrape_client: Arc<dyn ScrapeClient>,
        response_cache: Arc<dyn ResponseCache>,
        webhook: Arc<WebhookDelivery>,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Self {
        Self {
            queue,
            job_repo,
            tenant_repo,
            usage_repo,
            proxy_selector,
            profile_rotator,
            scrape_client,
            response_cache,
            webhook,
            shutdown_rx,
        }
    }

    pub async fn run(&self) {
        info!("worker started");

        let queue = self.queue.clone();
        let mut maintenance_interval = tokio::time::interval(std::time::Duration::from_secs(5));

        loop {
            if *self.shutdown_rx.borrow() {
                info!("worker shutting down");
                break;
            }

            tokio::select! {
                _ = maintenance_interval.tick() => {
                    self.run_maintenance().await;
                }
                result = queue.dequeue(5.0) => {
                    match result {
                        Ok(Some(job_id)) => {
                            if let Err(e) = self.process_job(job_id).await {
                                error!("job {job_id} processing error: {e}");
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            error!("dequeue error: {e}");
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                    }
                }
            }
        }
    }

    async fn run_maintenance(&self) {
        if let Ok(promoted) = self.queue.promote_scheduled().await {
            if promoted > 0 {
                info!("promoted {promoted} scheduled jobs");
            }
        }
        if let Ok(reclaimed) = self.queue.reclaim_stale(300).await {
            if reclaimed > 0 {
                warn!("reclaimed {reclaimed} stale jobs");
            }
        }
    }

    fn backoff_delay_secs(attempts: i32) -> f64 {
        match attempts {
            0 => 5.0,
            1 => 30.0,
            2 => 120.0,
            _ => 300.0,
        }
    }

    async fn handle_failure(
        &self,
        job_id: Uuid,
        job: &Job,
        error_msg: &str,
    ) -> Result<(), DomainError> {
        let attempts = job.attempts + 1;
        if attempts < job.max_retries {
            self.job_repo
                .set_failed(job_id, error_msg, attempts)
                .await?;
            self.job_repo.set_status(job_id, JobStatus::Pending).await?;
            self.queue.ack(job_id).await?;

            let delay = Self::backoff_delay_secs(attempts);
            let run_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64()
                + delay;
            self.queue.schedule(job_id, run_at).await?;

            warn!(
                "job {job_id} failed ({error_msg}), retry {attempts}/{} in {delay}s",
                job.max_retries
            );
        } else {
            self.job_repo
                .set_failed(job_id, error_msg, attempts)
                .await?;
            self.job_repo.set_status(job_id, JobStatus::Failed).await?;
            self.queue.move_to_dead(job_id).await?;
            error!("job {job_id} failed permanently: {error_msg}");

            self.deliver_webhook(job.tenant_id, job_id, None, Some(error_msg))
                .await;
        }
        Ok(())
    }

    async fn deliver_webhook(
        &self,
        tenant_id: Uuid,
        job_id: Uuid,
        result: Option<&crate::domain::entity::ScrapeJobResult>,
        error: Option<&str>,
    ) {
        let Ok(Some(tenant)) = self.tenant_repo.find_by_id(tenant_id).await else {
            return;
        };

        if let Some(ref webhook_url) = tenant.webhook_url {
            let wh = self.webhook.clone();
            let url = webhook_url.clone();
            let result_owned = result.cloned();
            let error_owned = error.map(String::from);
            tokio::spawn(async move {
                wh.deliver(&url, job_id, result_owned.as_ref(), error_owned.as_deref())
                    .await;
            });
        }
    }

    async fn process_job(&self, job_id: Uuid) -> Result<(), DomainError> {
        let job = self
            .job_repo
            .find_by_id(job_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("job {job_id}")))?;

        if job.status != JobStatus::Pending {
            self.queue.ack(job_id).await?;
            return Ok(());
        }

        let request: crate::domain::entity::ScrapeJobRequest =
            serde_json::from_value(job.request.clone())
                .map_err(|e| DomainError::InvalidInput(format!("malformed job request: {e}")))?;

        if let Ok(Some(cached)) = self.response_cache.get(&request.url).await {
            info!(
                "job {job_id} served from cache ({}ms saved)",
                cached.elapsed_ms
            );
            let bytes = cached.html_length as i64;
            self.job_repo
                .set_running(job_id, None, &cached.profile_used)
                .await?;
            self.job_repo.set_completed(job_id, &cached).await?;
            self.queue.ack(job_id).await?;
            self.usage_repo.record(job.tenant_id, job_id, bytes).await?;
            self.deliver_webhook(job.tenant_id, job_id, Some(&cached), None)
                .await;
            return Ok(());
        }

        let proxy_url = self.proxy_selector.select_proxy(&request).await?;
        let profile = self.profile_rotator.select_profile(&request).await?;

        self.job_repo.set_running(job_id, None, &profile).await?;

        match self
            .scrape_client
            .scrape(&request.url, &profile, &request, proxy_url.as_deref())
            .await
        {
            Ok(result) => {
                let bytes = result.html_length as i64;
                self.job_repo.set_completed(job_id, &result).await?;
                self.queue.ack(job_id).await?;
                self.usage_repo.record(job.tenant_id, job_id, bytes).await?;
                info!("job {job_id} completed in {}ms", result.elapsed_ms);

                if let Err(e) = self.response_cache.set(&request.url, &result, None).await {
                    warn!("failed to cache result for {}: {e}", request.url);
                }

                self.deliver_webhook(job.tenant_id, job_id, Some(&result), None)
                    .await;
            }
            Err(DomainError::ScrapeBlocked) => {
                if let Some(ref proxy) = proxy_url {
                    self.proxy_selector.report_proxy_failure(proxy).await;
                }
                self.profile_rotator.report_profile_failure(&profile).await;
                self.handle_failure(job_id, &job, "blocked by target site")
                    .await?;
            }
            Err(e) => {
                self.handle_failure(job_id, &job, &e.to_string()).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_delay_increases() {
        assert!((Worker::backoff_delay_secs(0) - 5.0).abs() < f64::EPSILON);
        assert!((Worker::backoff_delay_secs(1) - 30.0).abs() < f64::EPSILON);
        assert!((Worker::backoff_delay_secs(2) - 120.0).abs() < f64::EPSILON);
        assert!((Worker::backoff_delay_secs(3) - 300.0).abs() < f64::EPSILON);
        assert!((Worker::backoff_delay_secs(10) - 300.0).abs() < f64::EPSILON);
    }
}
