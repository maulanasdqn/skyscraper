use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
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
use crate::infrastructure::dedup::InflightDedup;
use crate::infrastructure::domain_throttle::DomainThrottle;
use crate::infrastructure::metrics::MetricsCollector;
use crate::infrastructure::webhook::WebhookDelivery;

const JOB_TIMEOUT: Duration = Duration::from_mins(2);

pub struct Worker {
    queue: Arc<dyn JobQueue>,
    job_repo: Arc<dyn JobRepository>,
    tenant_repo: Arc<dyn TenantRepository>,
    usage_repo: Arc<dyn UsageRepository>,
    proxy_selector: Arc<dyn ProxySelection>,
    profile_rotator: Arc<dyn ProfileSelection>,
    scrape_client: Arc<dyn ScrapeClient>,
    response_cache: Arc<dyn ResponseCache>,
    dedup: Arc<InflightDedup>,
    domain_throttle: Arc<DomainThrottle>,
    metrics: Arc<MetricsCollector>,
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
        dedup: Arc<InflightDedup>,
        domain_throttle: Arc<DomainThrottle>,
        metrics: Arc<MetricsCollector>,
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
            dedup,
            domain_throttle,
            metrics,
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
            self.metrics.jobs_failed.fetch_add(1, Ordering::Relaxed);
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
            let metrics = self.metrics.clone();
            tokio::spawn(async move {
                wh.deliver(&url, job_id, result_owned.as_ref(), error_owned.as_deref())
                    .await;
                metrics.webhook_deliveries.fetch_add(1, Ordering::Relaxed);
            });
        }
    }

    async fn complete_from_cache(
        &self,
        job_id: Uuid,
        job: &Job,
        cached: &crate::domain::entity::ScrapeJobResult,
    ) -> Result<(), DomainError> {
        let bytes = cached.html_length as i64;
        self.job_repo
            .set_running(job_id, None, &cached.profile_used)
            .await?;
        self.job_repo.set_completed(job_id, cached).await?;
        self.queue.ack(job_id).await?;
        self.usage_repo.record(job.tenant_id, job_id, bytes).await?;
        self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
        self.metrics.record_job_completed(0);
        self.deliver_webhook(job.tenant_id, job_id, Some(cached), None)
            .await;
        Ok(())
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

        // 1. Check response cache
        if let Ok(Some(cached)) = self.response_cache.get(&request.url).await {
            info!(
                "job {job_id} served from cache ({}ms saved)",
                cached.elapsed_ms
            );
            return self.complete_from_cache(job_id, &job, &cached).await;
        }

        // 2. Domain throttle — reschedule if domain is being hit too fast
        if let Ok(false) = self.domain_throttle.check_allowed(&request.url).await {
            self.queue.ack(job_id).await?;
            let delay = self.domain_throttle.delay_for_domain(&request.url);
            let run_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64()
                + delay;
            self.queue.schedule(job_id, run_at).await?;
            self.metrics
                .domain_throttled
                .fetch_add(1, Ordering::Relaxed);
            info!("job {job_id} domain-throttled, rescheduled in {delay}s");
            return Ok(());
        }

        // 3. In-flight dedup — another worker may be scraping this URL right now
        let acquired = self.dedup.try_acquire(&request.url).await.unwrap_or(true);
        if !acquired {
            info!(
                "job {job_id} waiting for in-flight scrape of {}",
                request.url
            );
            self.metrics.dedup_coalesced.fetch_add(1, Ordering::Relaxed);

            if let Ok(Some(result)) = self
                .dedup
                .wait_for_result(&request.url, self.response_cache.as_ref())
                .await
            {
                info!("job {job_id} coalesced from in-flight result");
                return self.complete_from_cache(job_id, &job, &result).await;
            }
            // Timed out waiting — fall through to scrape ourselves
        }

        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
        self.execute_scrape(job_id, &job, &request).await
    }

    async fn execute_scrape(
        &self,
        job_id: Uuid,
        job: &Job,
        request: &crate::domain::entity::ScrapeJobRequest,
    ) -> Result<(), DomainError> {
        let proxy_url = self.proxy_selector.select_proxy(request).await?;
        let profile = self.profile_rotator.select_profile(request).await?;

        self.job_repo.set_running(job_id, None, &profile).await?;

        let scrape_result = tokio::time::timeout(
            JOB_TIMEOUT,
            self.scrape_client
                .scrape(&request.url, &profile, request, proxy_url.as_deref()),
        )
        .await;

        let scrape_result = if let Ok(inner) = scrape_result {
            inner
        } else {
            self.metrics.jobs_timed_out.fetch_add(1, Ordering::Relaxed);
            warn!("job {job_id} timed out after {}s", JOB_TIMEOUT.as_secs());
            let _ = self.dedup.release(&request.url).await;
            Err(DomainError::Upstream(format!(
                "job timed out after {}s",
                JOB_TIMEOUT.as_secs()
            )))
        };

        match scrape_result {
            Ok(result) => {
                let bytes = result.html_length as i64;
                self.job_repo.set_completed(job_id, &result).await?;
                self.queue.ack(job_id).await?;
                self.usage_repo.record(job.tenant_id, job_id, bytes).await?;
                self.metrics.record_job_completed(result.elapsed_ms);
                info!("job {job_id} completed in {}ms", result.elapsed_ms);

                if let Err(e) = self.response_cache.set(&request.url, &result, None).await {
                    warn!("failed to cache result for {}: {e}", request.url);
                }
                let _ = self.dedup.release(&request.url).await;

                self.deliver_webhook(job.tenant_id, job_id, Some(&result), None)
                    .await;
            }
            Err(DomainError::ScrapeBlocked) => {
                let _ = self.dedup.release(&request.url).await;
                if let Some(ref proxy) = proxy_url {
                    self.proxy_selector.report_proxy_failure(proxy).await;
                }
                self.profile_rotator.report_profile_failure(&profile).await;
                self.handle_failure(job_id, job, "blocked by target site")
                    .await?;
            }
            Err(e) => {
                let _ = self.dedup.release(&request.url).await;
                self.handle_failure(job_id, job, &e.to_string()).await?;
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
