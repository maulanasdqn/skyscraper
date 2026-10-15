use redis::aio::ConnectionManager;
use redis::AsyncCommands;

use crate::domain::entity::ScrapeJobResult;
use crate::domain::error::DomainError;
use crate::domain::port::ResponseCache;

pub struct InflightDedup {
    conn: ConnectionManager,
}

const INFLIGHT_PREFIX: &str = "skyscraper:inflight:";
const INFLIGHT_TTL_SECS: u64 = 180;
const POLL_INTERVAL_MS: u64 = 200;
const MAX_WAIT_SECS: u64 = 120;

impl InflightDedup {
    pub fn new(conn: ConnectionManager) -> Self {
        Self { conn }
    }

    fn lock_key(url: &str) -> String {
        format!("{INFLIGHT_PREFIX}{url}")
    }

    pub async fn try_acquire(&self, url: &str) -> Result<bool, DomainError> {
        let mut conn = self.conn.clone();
        let key = Self::lock_key(url);

        let set: bool = redis::cmd("SET")
            .arg(&key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(INFLIGHT_TTL_SECS)
            .query_async(&mut conn)
            .await
            .map_err(|e| DomainError::Upstream(format!("inflight acquire: {e}")))?;

        Ok(set)
    }

    pub async fn release(&self, url: &str) -> Result<(), DomainError> {
        let mut conn = self.conn.clone();
        let key = Self::lock_key(url);
        conn.del::<_, ()>(&key)
            .await
            .map_err(|e| DomainError::Upstream(format!("inflight release: {e}")))?;
        Ok(())
    }

    pub async fn wait_for_result(
        &self,
        url: &str,
        cache: &dyn ResponseCache,
    ) -> Result<Option<ScrapeJobResult>, DomainError> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(MAX_WAIT_SECS);
        let poll = std::time::Duration::from_millis(POLL_INTERVAL_MS);

        while tokio::time::Instant::now() < deadline {
            if let Some(result) = cache.get(url).await? {
                return Ok(Some(result));
            }

            let mut conn = self.conn.clone();
            let key = Self::lock_key(url);
            let exists: bool = conn.exists(&key).await.unwrap_or(false);

            if !exists {
                return cache.get(url).await;
            }

            tokio::time::sleep(poll).await;
        }

        Ok(None)
    }
}
