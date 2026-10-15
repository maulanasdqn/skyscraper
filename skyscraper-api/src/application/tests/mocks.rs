use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entity::*;
use crate::domain::error::DomainError;
use crate::domain::port::{JobQueue, UsageRepository};

pub use super::mock_job_repo::MockJobRepository;

pub struct MockJobQueue {
    pub enqueued: Arc<Mutex<Vec<Uuid>>>,
    pub scheduled: Arc<Mutex<Vec<(Uuid, f64)>>>,
    rate_limit_result: bool,
}

impl MockJobQueue {
    pub fn new(rate_limit_result: bool) -> Self {
        Self {
            enqueued: Arc::new(Mutex::new(vec![])),
            scheduled: Arc::new(Mutex::new(vec![])),
            rate_limit_result,
        }
    }
}

#[async_trait]
impl JobQueue for MockJobQueue {
    async fn enqueue(&self, job_id: Uuid, _priority: i16) -> Result<(), DomainError> {
        self.enqueued.lock().unwrap().push(job_id);
        Ok(())
    }
    async fn schedule(&self, job_id: Uuid, run_at_epoch: f64) -> Result<(), DomainError> {
        self.scheduled.lock().unwrap().push((job_id, run_at_epoch));
        Ok(())
    }
    async fn dequeue(&self, _: f64) -> Result<Option<Uuid>, DomainError> {
        Ok(None)
    }
    async fn ack(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn move_to_dead(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn promote_scheduled(&self) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn reclaim_stale(&self, _: u64) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn check_rate_limit(&self, _: &Uuid, _: i32) -> Result<bool, DomainError> {
        Ok(self.rate_limit_result)
    }
    async fn rate_limit_info(
        &self,
        _: &Uuid,
        limit_rpm: i32,
    ) -> Result<RateLimitInfo, DomainError> {
        Ok(RateLimitInfo {
            limit: limit_rpm,
            remaining: i64::from(limit_rpm),
            reset_epoch: 0,
        })
    }
    async fn queue_stats(&self) -> Result<QueueStats, DomainError> {
        Ok(QueueStats {
            pending: 0,
            pending_high: 0,
            pending_default: 0,
            pending_low: 0,
            processing: 0,
            scheduled: 0,
            dead: 0,
        })
    }
    async fn retry_dead(&self, _: usize) -> Result<u64, DomainError> {
        Ok(0)
    }
}

pub struct MockUsageRepository;

#[async_trait]
impl UsageRepository for MockUsageRepository {
    async fn record(&self, _: Uuid, _: Uuid, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn total_for_tenant(&self, _: Uuid, _: DateTime<Utc>) -> Result<i64, DomainError> {
        Ok(0)
    }
    async fn list_for_tenant(
        &self,
        _: Uuid,
        _: i64,
        _: i64,
    ) -> Result<Vec<UsageRecord>, DomainError> {
        Ok(vec![])
    }
}
