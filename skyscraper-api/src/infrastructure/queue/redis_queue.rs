use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use uuid::Uuid;

use crate::domain::entity::{QueueStats, RateLimitInfo};
use crate::domain::error::DomainError;
use crate::domain::port::JobQueue;

pub struct RedisQueue {
    pub(super) conn: ConnectionManager,
}

pub(super) const HIGH_QUEUE: &str = "skyscraper:queue:high";
pub(super) const DEFAULT_QUEUE: &str = "skyscraper:queue:default";
pub(super) const LOW_QUEUE: &str = "skyscraper:queue:low";
pub(super) const PROCESSING_SET: &str = "skyscraper:queue:processing";
pub(super) const SCHEDULED_SET: &str = "skyscraper:queue:scheduled";
pub(super) const DEAD_QUEUE: &str = "skyscraper:queue:dead";
const DEAD_QUEUE_MAX: i64 = 10_000;

impl RedisQueue {
    pub async fn new(redis_url: &str) -> Result<Self, DomainError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| DomainError::Upstream(format!("redis: {e}")))?;
        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis conn: {e}")))?;
        Ok(Self { conn })
    }

    fn queue_key(priority: i16) -> &'static str {
        match priority {
            8..=10 => HIGH_QUEUE,
            0..=2 => LOW_QUEUE,
            _ => DEFAULT_QUEUE,
        }
    }
}

#[async_trait]
impl JobQueue for RedisQueue {
    async fn enqueue(&self, job_id: Uuid, priority: i16) -> Result<(), DomainError> {
        let mut conn = self.conn.clone();
        let key = Self::queue_key(priority);
        conn.rpush::<_, _, ()>(key, job_id.to_string())
            .await
            .map_err(|e| DomainError::Upstream(format!("redis enqueue: {e}")))?;
        Ok(())
    }

    async fn schedule(&self, job_id: Uuid, run_at_epoch: f64) -> Result<(), DomainError> {
        let mut conn = self.conn.clone();
        conn.zadd::<_, _, _, ()>(SCHEDULED_SET, job_id.to_string(), run_at_epoch)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis schedule: {e}")))?;
        Ok(())
    }

    async fn dequeue(&self, timeout_secs: f64) -> Result<Option<Uuid>, DomainError> {
        let mut conn = self.conn.clone();
        let result: Option<(String, String)> = conn
            .blpop(&[HIGH_QUEUE, DEFAULT_QUEUE, LOW_QUEUE], timeout_secs)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis dequeue: {e}")))?;

        let Some((_, id_str)) = result else {
            return Ok(None);
        };

        let job_id: Uuid = id_str
            .parse()
            .map_err(|e| DomainError::Upstream(format!("invalid job id in queue: {e}")))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        conn.zadd::<_, _, _, ()>(PROCESSING_SET, job_id.to_string(), now)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis processing add: {e}")))?;

        Ok(Some(job_id))
    }

    async fn ack(&self, job_id: Uuid) -> Result<(), DomainError> {
        let mut conn = self.conn.clone();
        conn.zrem::<_, _, ()>(PROCESSING_SET, job_id.to_string())
            .await
            .map_err(|e| DomainError::Upstream(format!("redis ack: {e}")))?;
        Ok(())
    }

    async fn move_to_dead(&self, job_id: Uuid) -> Result<(), DomainError> {
        let mut conn = self.conn.clone();
        conn.zrem::<_, _, ()>(PROCESSING_SET, job_id.to_string())
            .await
            .map_err(|e| DomainError::Upstream(format!("redis dead zrem: {e}")))?;
        conn.lpush::<_, _, ()>(DEAD_QUEUE, job_id.to_string())
            .await
            .map_err(|e| DomainError::Upstream(format!("redis dead push: {e}")))?;
        conn.ltrim::<_, ()>(DEAD_QUEUE, 0, (DEAD_QUEUE_MAX - 1) as isize)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis dead trim: {e}")))?;
        Ok(())
    }

    async fn check_rate_limit(
        &self,
        tenant_id: &Uuid,
        limit_rpm: i32,
    ) -> Result<bool, DomainError> {
        let key = format!("skyscraper:rate:{tenant_id}");
        let mut conn = self.conn.clone();
        let script = redis::Script::new(
            r"
            local count = redis.call('INCR', KEYS[1])
            if count == 1 then
                redis.call('EXPIRE', KEYS[1], 60)
            end
            return count
            ",
        );
        let count: i64 = script
            .key(&key)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis rate limit: {e}")))?;
        Ok(count <= i64::from(limit_rpm))
    }

    async fn promote_scheduled(&self) -> Result<u64, DomainError> {
        self.do_promote_scheduled().await
    }

    async fn reclaim_stale(&self, max_age_secs: u64) -> Result<u64, DomainError> {
        self.do_reclaim_stale(max_age_secs).await
    }

    async fn rate_limit_info(
        &self,
        tenant_id: &Uuid,
        limit_rpm: i32,
    ) -> Result<RateLimitInfo, DomainError> {
        self.do_rate_limit_info(tenant_id, limit_rpm).await
    }

    async fn retry_dead(&self, limit: usize) -> Result<u64, DomainError> {
        self.do_retry_dead(limit).await
    }

    async fn queue_stats(&self) -> Result<QueueStats, DomainError> {
        self.do_queue_stats().await
    }
}
