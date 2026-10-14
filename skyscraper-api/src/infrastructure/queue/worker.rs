use std::sync::Arc;
use tokio::sync::watch;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::domain::entity::Job;
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;
use crate::domain::port::{
    JobQueue, JobRepository, ProfileSelection, ProxySelection, ScrapeClient, UsageRepository,
};

pub struct Worker {
    queue: Arc<dyn JobQueue>,
    job_repo: Arc<dyn JobRepository>,
    usage_repo: Arc<dyn UsageRepository>,
    proxy_selector: Arc<dyn ProxySelection>,
    profile_rotator: Arc<dyn ProfileSelection>,
    scrape_client: Arc<dyn ScrapeClient>,
    shutdown_rx: watch::Receiver<bool>,
}

impl Worker {
    pub fn new(
        queue: Arc<dyn JobQueue>,
        job_repo: Arc<dyn JobRepository>,
        usage_repo: Arc<dyn UsageRepository>,
        proxy_selector: Arc<dyn ProxySelection>,
        profile_rotator: Arc<dyn ProfileSelection>,
        scrape_client: Arc<dyn ScrapeClient>,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Self {
        Self {
            queue,
            job_repo,
            usage_repo,
            proxy_selector,
            profile_rotator,
            scrape_client,
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

    async fn handle_failure(
        &self,
        job_id: Uuid,
        job: &Job,
        error_msg: &str,
    ) -> Result<(), DomainError> {
        let attempts = job.attempts + 1;
        if attempts < job.max_retries {
            self.job_repo.set_failed(job_id, error_msg, attempts).await?;
            self.job_repo.set_status(job_id, JobStatus::Pending).await?;
            self.queue.ack(job_id).await?;
            self.queue.enqueue(job_id, job.priority).await?;
            warn!("job {job_id} failed ({error_msg}), retry {attempts}/{}", job.max_retries);
        } else {
            self.job_repo.set_failed(job_id, error_msg, attempts).await?;
            self.job_repo.set_status(job_id, JobStatus::Failed).await?;
            self.queue.move_to_dead(job_id).await?;
            error!("job {job_id} failed permanently: {error_msg}");
        }
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

        let proxy_url = self.proxy_selector.select_proxy(&request).await?;
        let profile = self.profile_rotator.select_profile(&request).await?;

        self.job_repo
            .set_running(job_id, None, &profile)
            .await?;

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
            }
            Err(DomainError::ScrapeBlocked) => {
                if let Some(ref proxy) = proxy_url {
                    self.proxy_selector.report_proxy_failure(proxy).await;
                }
                self.profile_rotator.report_profile_failure(&profile).await;
                self.handle_failure(job_id, &job, "blocked by target site").await?;
            }
            Err(e) => {
                self.handle_failure(job_id, &job, &e.to_string()).await?;
            }
        }

        Ok(())
    }
}
