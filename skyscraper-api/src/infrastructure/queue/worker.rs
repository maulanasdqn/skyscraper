use chrono::Utc;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{error, info, warn};
use uuid::Uuid;

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

const JOB_TTL_HOURS: i64 = 72;

pub struct Worker {
    pub(super) queue: Arc<dyn JobQueue>,
    pub(super) job_repo: Arc<dyn JobRepository>,
    pub(super) tenant_repo: Arc<dyn TenantRepository>,
    pub(super) usage_repo: Arc<dyn UsageRepository>,
    pub(super) proxy_selector: Arc<dyn ProxySelection>,
    pub(super) profile_rotator: Arc<dyn ProfileSelection>,
    pub(super) scrape_client: Arc<dyn ScrapeClient>,
    pub(super) response_cache: Arc<dyn ResponseCache>,
    pub(super) dedup: Arc<InflightDedup>,
    pub(super) domain_throttle: Arc<DomainThrottle>,
    pub(super) metrics: Arc<MetricsCollector>,
    pub(super) webhook: Arc<WebhookDelivery>,
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
        let mut last_maintenance = tokio::time::Instant::now();
        let maintenance_interval = std::time::Duration::from_secs(5);
        loop {
            if *self.shutdown_rx.borrow() {
                info!("worker shutting down");
                break;
            }
            if last_maintenance.elapsed() >= maintenance_interval {
                self.run_maintenance().await;
                last_maintenance = tokio::time::Instant::now();
            }
            match self.queue.dequeue(5.0).await {
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
        let cutoff = Utc::now() - chrono::Duration::hours(JOB_TTL_HOURS);
        if let Ok(purged) = self.job_repo.delete_completed_before(cutoff).await {
            if purged > 0 {
                info!("purged {purged} old completed/dead jobs");
            }
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
            info!("job {job_id} served from cache");
            return self.complete_from_cache(job_id, &job, &cached).await;
        }

        if let Ok(false) = self.domain_throttle.check_allowed(&request.url).await {
            self.queue.ack(job_id).await?;
            let delay = self.domain_throttle.delay_for_domain(&request.url);
            let run_at = epoch_secs() + delay;
            self.queue.schedule(job_id, run_at).await?;
            self.metrics
                .domain_throttled
                .fetch_add(1, Ordering::Relaxed);
            info!("job {job_id} domain-throttled, rescheduled in {delay}s");
            return Ok(());
        }

        let acquired = self.dedup.try_acquire(&request.url).await.unwrap_or(true);
        if !acquired {
            self.metrics.dedup_coalesced.fetch_add(1, Ordering::Relaxed);
            if let Ok(Some(result)) = self
                .dedup
                .wait_for_result(&request.url, self.response_cache.as_ref())
                .await
            {
                info!("job {job_id} coalesced from in-flight result");
                return self.complete_from_cache(job_id, &job, &result).await;
            }
        }

        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
        self.execute_scrape(job_id, &job, &request).await
    }
}

fn epoch_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
