use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;

use crate::domain::entity::ScrapeJobResult;
use crate::domain::error::DomainError;
use crate::domain::port::ResponseCache;

pub struct RedisResponseCache {
    conn: ConnectionManager,
    default_ttl_secs: u64,
}

const CACHE_PREFIX: &str = "skyscraper:cache:";

impl RedisResponseCache {
    pub fn new(conn: ConnectionManager, default_ttl_secs: u64) -> Self {
        Self {
            conn,
            default_ttl_secs,
        }
    }

    fn cache_key(url: &str) -> String {
        format!("{CACHE_PREFIX}{url}")
    }
}

#[async_trait]
impl ResponseCache for RedisResponseCache {
    async fn get(&self, url: &str) -> Result<Option<ScrapeJobResult>, DomainError> {
        let mut conn = self.conn.clone();
        let key = Self::cache_key(url);

        let raw: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| DomainError::Upstream(format!("cache get: {e}")))?;

        let Some(json_str) = raw else {
            return Ok(None);
        };

        let result: ScrapeJobResult = serde_json::from_str(&json_str)
            .map_err(|e| DomainError::Internal(anyhow::anyhow!("cache deserialize: {e}")))?;

        Ok(Some(result))
    }

    async fn set(
        &self,
        url: &str,
        result: &ScrapeJobResult,
        ttl_secs: Option<u64>,
    ) -> Result<(), DomainError> {
        let mut conn = self.conn.clone();
        let key = Self::cache_key(url);
        let ttl = ttl_secs.unwrap_or(self.default_ttl_secs);

        let json_str = serde_json::to_string(result)
            .map_err(|e| DomainError::Internal(anyhow::anyhow!("cache serialize: {e}")))?;

        conn.set_ex::<_, _, ()>(&key, &json_str, ttl)
            .await
            .map_err(|e| DomainError::Upstream(format!("cache set: {e}")))?;

        Ok(())
    }
}
