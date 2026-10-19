use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::domain::entity::{Job, ScrapeJobRequest, ScrapeJobResult};
use crate::domain::error::DomainError;
use crate::domain::job_status::JobStatus;
use crate::infrastructure::content_validator;

use super::worker::Worker;

const JOB_TIMEOUT: Duration = Duration::from_secs(30);

impl Worker {
    pub(super) fn backoff_delay_secs(attempts: i32) -> f64 {
        match attempts {
            0 => 5.0,
            1 => 30.0,
            2 => 120.0,
            _ => 300.0,
        }
    }

    pub(super) async fn handle_failure(
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

    pub(super) async fn deliver_webhook(
        &self,
        tenant_id: Uuid,
        job_id: Uuid,
        result: Option<&ScrapeJobResult>,
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

    pub(super) async fn complete_from_cache(
        &self,
        job_id: Uuid,
        job: &Job,
        cached: &ScrapeJobResult,
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

    pub(super) async fn execute_scrape(
        &self,
        job_id: Uuid,
        job: &Job,
        request: &ScrapeJobRequest,
    ) -> Result<(), DomainError> {
        let cache_key = compute_cache_key(request);
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
            let _ = self.dedup.release(&cache_key).await;
            Err(DomainError::Upstream(format!(
                "job timed out after {}s",
                JOB_TIMEOUT.as_secs()
            )))
        };

        match scrape_result {
            Ok(result) => {
                let validation = content_validator::validate(&result);
                if validation.is_valid {
                    self.on_scrape_success(job_id, job, &cache_key, &result)
                        .await?;
                } else {
                    let reason = validation.reason.unwrap_or_default();
                    warn!("job {job_id} soft-blocked: {reason}");
                    let _ = self.dedup.release(&cache_key).await;
                    if let Some(ref proxy) = proxy_url {
                        self.proxy_selector.report_proxy_failure(proxy).await;
                    }
                    self.profile_rotator.report_profile_failure(&profile).await;
                    self.handle_failure(job_id, job, &format!("soft block: {reason}"))
                        .await?;
                }
            }
            Err(DomainError::ScrapeBlocked) => {
                let _ = self.dedup.release(&cache_key).await;
                if let Some(ref proxy) = proxy_url {
                    self.proxy_selector.report_proxy_failure(proxy).await;
                }
                self.profile_rotator.report_profile_failure(&profile).await;
                self.handle_failure(job_id, job, "blocked by target site")
                    .await?;
            }
            Err(e) => {
                let _ = self.dedup.release(&cache_key).await;
                self.handle_failure(job_id, job, &e.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn on_scrape_success(
        &self,
        job_id: Uuid,
        job: &Job,
        cache_key: &str,
        result: &ScrapeJobResult,
    ) -> Result<(), DomainError> {
        let bytes = result.html_length as i64;
        self.job_repo.set_completed(job_id, result).await?;
        self.queue.ack(job_id).await?;
        self.usage_repo.record(job.tenant_id, job_id, bytes).await?;
        self.metrics.record_job_completed(result.elapsed_ms);
        info!("job {job_id} completed in {}ms", result.elapsed_ms);

        if let Err(e) = self.response_cache.set(cache_key, result, None).await {
            warn!("failed to cache result for {cache_key}: {e}");
        }
        let _ = self.dedup.release(cache_key).await;

        self.deliver_webhook(job.tenant_id, job_id, Some(result), None)
            .await;
        Ok(())
    }
}

pub(super) fn compute_cache_key(request: &ScrapeJobRequest) -> String {
    let mut hasher = DefaultHasher::new();
    request.extract.hash(&mut hasher);
    request.script.hash(&mut hasher);
    request.screenshot.hash(&mut hasher);
    let hash = hasher.finish();
    if hash == DefaultHasher::new().finish() {
        request.url.clone()
    } else {
        format!("{}#{:x}", request.url, hash)
    }
}

#[cfg(test)]
mod tests {
    use super::Worker;

    #[test]
    fn backoff_delay_increases() {
        assert!((Worker::backoff_delay_secs(0) - 5.0).abs() < f64::EPSILON);
        assert!((Worker::backoff_delay_secs(1) - 30.0).abs() < f64::EPSILON);
        assert!((Worker::backoff_delay_secs(2) - 120.0).abs() < f64::EPSILON);
        assert!((Worker::backoff_delay_secs(3) - 300.0).abs() < f64::EPSILON);
        assert!((Worker::backoff_delay_secs(10) - 300.0).abs() < f64::EPSILON);
    }
}
