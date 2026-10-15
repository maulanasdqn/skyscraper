use redis::aio::ConnectionManager;

use crate::domain::error::DomainError;

pub struct DomainThrottle {
    conn: ConnectionManager,
    max_rpm: u32,
}

const DOMAIN_RATE_PREFIX: &str = "skyscraper:domain_rate:";

impl DomainThrottle {
    pub fn new(conn: ConnectionManager, max_rpm: u32) -> Self {
        Self { conn, max_rpm }
    }

    pub fn extract_domain(url: &str) -> Option<String> {
        let after_scheme = url.split("://").nth(1)?;
        let host_port = after_scheme.split('/').next()?;
        let host = host_port.split(':').next()?;
        if host.is_empty() {
            return None;
        }
        Some(host.to_lowercase())
    }

    pub async fn check_allowed(&self, url: &str) -> Result<bool, DomainError> {
        let Some(domain) = Self::extract_domain(url) else {
            return Ok(true);
        };

        let key = format!("{DOMAIN_RATE_PREFIX}{domain}");
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
            .map_err(|e| DomainError::Upstream(format!("domain throttle: {e}")))?;

        Ok(count <= i64::from(self.max_rpm))
    }

    pub fn delay_for_domain(&self, url: &str) -> f64 {
        let _ = url;
        5.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_domain_https() {
        assert_eq!(
            DomainThrottle::extract_domain("https://example.com/page"),
            Some("example.com".into())
        );
    }

    #[test]
    fn extract_domain_with_port() {
        assert_eq!(
            DomainThrottle::extract_domain("http://api.example.com:8080/v1"),
            Some("api.example.com".into())
        );
    }

    #[test]
    fn extract_domain_subdomain() {
        assert_eq!(
            DomainThrottle::extract_domain("https://www.sub.example.co.uk/path"),
            Some("www.sub.example.co.uk".into())
        );
    }

    #[test]
    fn extract_domain_none_for_invalid() {
        assert_eq!(DomainThrottle::extract_domain("not-a-url"), None);
    }

    #[test]
    fn extract_domain_lowercase() {
        assert_eq!(
            DomainThrottle::extract_domain("https://EXAMPLE.COM"),
            Some("example.com".into())
        );
    }
}
