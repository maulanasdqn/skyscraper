use redis::AsyncCommands;
use uuid::Uuid;

use crate::domain::entity::{QueueStats, RateLimitInfo};
use crate::domain::error::DomainError;

use super::redis_queue::{
    RedisQueue, DEAD_QUEUE, DEFAULT_QUEUE, HIGH_QUEUE, LOW_QUEUE, PROCESSING_SET, SCHEDULED_SET,
};

impl RedisQueue {
    pub(super) async fn do_promote_scheduled(&self) -> Result<u64, DomainError> {
        let mut conn = self.conn.clone();
        let now = epoch_secs();

        let due: Vec<String> = conn
            .zrangebyscore(SCHEDULED_SET, 0.0_f64, now)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis scheduled range: {e}")))?;

        let mut promoted = 0u64;
        for id_str in &due {
            let removed: i64 = conn
                .zrem(SCHEDULED_SET, id_str)
                .await
                .map_err(|e| DomainError::Upstream(format!("redis scheduled rem: {e}")))?;
            if removed > 0 {
                conn.rpush::<_, _, ()>(DEFAULT_QUEUE, id_str)
                    .await
                    .map_err(|e| DomainError::Upstream(format!("redis promote push: {e}")))?;
                promoted += 1;
            }
        }
        Ok(promoted)
    }

    pub(super) async fn do_reclaim_stale(&self, max_age_secs: u64) -> Result<u64, DomainError> {
        let mut conn = self.conn.clone();
        let cutoff = epoch_secs() - max_age_secs as f64;

        let stale: Vec<String> = conn
            .zrangebyscore(PROCESSING_SET, 0.0_f64, cutoff)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis stale range: {e}")))?;

        let mut reclaimed = 0u64;
        for id_str in &stale {
            let removed: i64 = conn
                .zrem(PROCESSING_SET, id_str)
                .await
                .map_err(|e| DomainError::Upstream(format!("redis stale rem: {e}")))?;
            if removed > 0 {
                conn.rpush::<_, _, ()>(DEFAULT_QUEUE, id_str)
                    .await
                    .map_err(|e| DomainError::Upstream(format!("redis reclaim push: {e}")))?;
                reclaimed += 1;
            }
        }
        Ok(reclaimed)
    }

    pub(super) async fn do_rate_limit_info(
        &self,
        tenant_id: &Uuid,
        limit_rpm: i32,
    ) -> Result<RateLimitInfo, DomainError> {
        let key = format!("skyscraper:rate:{tenant_id}");
        let mut conn = self.conn.clone();

        let script = redis::Script::new(
            r"
            local count = tonumber(redis.call('GET', KEYS[1]) or '0')
            local ttl = redis.call('TTL', KEYS[1])
            if ttl < 0 then ttl = 60 end
            return {count, ttl}
            ",
        );

        let (count, ttl): (i64, i64) = script
            .key(&key)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| DomainError::Upstream(format!("redis rate limit info: {e}")))?;

        let limit = i64::from(limit_rpm);
        let remaining = (limit - count).max(0);
        let now = epoch_secs() as i64;

        Ok(RateLimitInfo {
            limit: limit_rpm,
            remaining,
            reset_epoch: now + ttl,
        })
    }

    pub(super) async fn do_retry_dead(&self, limit: usize) -> Result<u64, DomainError> {
        let mut conn = self.conn.clone();
        let mut retried = 0u64;

        for _ in 0..limit {
            let id_str: Option<String> = conn
                .rpop(DEAD_QUEUE, None)
                .await
                .map_err(|e| DomainError::Upstream(format!("redis dead pop: {e}")))?;

            let Some(id_str) = id_str else {
                break;
            };

            conn.rpush::<_, _, ()>(DEFAULT_QUEUE, &id_str)
                .await
                .map_err(|e| DomainError::Upstream(format!("redis retry push: {e}")))?;
            retried += 1;
        }
        Ok(retried)
    }

    pub(super) async fn do_queue_stats(&self) -> Result<QueueStats, DomainError> {
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
}

fn epoch_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
