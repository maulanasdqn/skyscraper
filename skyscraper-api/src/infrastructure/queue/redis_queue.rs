use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use uuid::Uuid;

use crate::domain::error::DomainError;

pub struct RedisQueue {
    conn: ConnectionManager,
}

const HIGH_QUEUE: &str = "skyscraper:queue:high";
const DEFAULT_QUEUE: &str = "skyscraper:queue:default";
const LOW_QUEUE: &str = "skyscraper:queue:low";
const PROCESSING_SET: &str = "skyscraper:queue:processing";
const SCHEDULED_SET: &str = "skyscraper:queue:scheduled";
const DEAD_QUEUE: &str = "skyscraper:queue:dead";
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

    pub async fn enqueue(&self, job_id: Uuid, priority: i16) -> Result<(), DomainError> {
        let mut conn = self.conn.clone();
        let key = Self::queue_key(priority);
        conn.rpush::<_, _, ()>(key, job_id.to_string())
            .await
            .map_err(|e| DomainError::Upstream(format!("redis enqueue: {e}")))?;
        Ok(())
    }

    pub async fn schedule(
        &self,
        job_id: Uuid,
        run_at_epoch: f64,
    ) -> Result<(), DomainError> {
        let mut conn = self.conn.clone();
        conn.zadd::<_, _, _, ()>(SCHEDULED_SET, job_id.to_string(), run_at_epoch)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis schedule: {e}")))?;
        Ok(())
    }

    pub async fn dequeue(&self, timeout_secs: f64) -> Result<Option<Uuid>, DomainError> {
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

    pub async fn ack(&self, job_id: Uuid) -> Result<(), DomainError> {
        let mut conn = self.conn.clone();
        conn.zrem::<_, _, ()>(PROCESSING_SET, job_id.to_string())
            .await
            .map_err(|e| DomainError::Upstream(format!("redis ack: {e}")))?;
        Ok(())
    }

    pub async fn move_to_dead(&self, job_id: Uuid) -> Result<(), DomainError> {
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

    pub async fn promote_scheduled(&self) -> Result<u64, DomainError> {
        let mut conn = self.conn.clone();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        let due: Vec<String> = conn
            .zrangebyscore(SCHEDULED_SET, 0.0_f64, now)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis scheduled range: {e}")))?;

        let mut promoted = 0u64;
        for id_str in &due {
            conn.zrem::<_, _, ()>(SCHEDULED_SET, id_str)
                .await
                .map_err(|e| DomainError::Upstream(format!("redis scheduled rem: {e}")))?;
            conn.rpush::<_, _, ()>(DEFAULT_QUEUE, id_str)
                .await
                .map_err(|e| DomainError::Upstream(format!("redis promote push: {e}")))?;
            promoted += 1;
        }

        Ok(promoted)
    }

    pub async fn reclaim_stale(&self, max_age_secs: u64) -> Result<u64, DomainError> {
        let mut conn = self.conn.clone();
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
            - max_age_secs as f64;

        let stale: Vec<String> = conn
            .zrangebyscore(PROCESSING_SET, 0.0_f64, cutoff)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis stale range: {e}")))?;

        let mut reclaimed = 0u64;
        for id_str in &stale {
            conn.zrem::<_, _, ()>(PROCESSING_SET, id_str)
                .await
                .map_err(|e| DomainError::Upstream(format!("redis stale rem: {e}")))?;
            conn.rpush::<_, _, ()>(DEFAULT_QUEUE, id_str)
                .await
                .map_err(|e| DomainError::Upstream(format!("redis reclaim push: {e}")))?;
            reclaimed += 1;
        }

        Ok(reclaimed)
    }

    pub async fn queue_stats(&self) -> Result<QueueStats, DomainError> {
        let mut conn = self.conn.clone();
        let high: u64 = conn.llen(HIGH_QUEUE).await.unwrap_or(0);
        let default: u64 = conn.llen(DEFAULT_QUEUE).await.unwrap_or(0);
        let low: u64 = conn.llen(LOW_QUEUE).await.unwrap_or(0);
        let processing: u64 = conn.zcard(PROCESSING_SET).await.unwrap_or(0);
        let scheduled: u64 = conn.zcard(SCHEDULED_SET).await.unwrap_or(0);
        let dead: u64 = conn.llen(DEAD_QUEUE).await.unwrap_or(0);

        Ok(QueueStats {
            pending: high + default + low,
            pending_high: high,
            pending_default: default,
            pending_low: low,
            processing,
            scheduled,
            dead,
        })
    }

    pub async fn check_rate_limit(
        &self,
        tenant_id: &Uuid,
        limit_rpm: i32,
    ) -> Result<bool, DomainError> {
        let key = format!("skyscraper:rate:{tenant_id}");
        let mut conn = self.conn.clone();

        let count: i64 = conn
            .incr(&key, 1)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis rate incr: {e}")))?;

        if count == 1 {
            conn.expire::<_, ()>(&key, 60)
                .await
                .map_err(|e| DomainError::Upstream(format!("redis rate expire: {e}")))?;
        }

        Ok(count <= i64::from(limit_rpm))
    }
}

#[derive(Debug, serde::Serialize)]
pub struct QueueStats {
    pub pending: u64,
    pub pending_high: u64,
    pub pending_default: u64,
    pub pending_low: u64,
    pub processing: u64,
    pub scheduled: u64,
    pub dead: u64,
}
