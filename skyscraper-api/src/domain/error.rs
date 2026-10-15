use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("rate limit exceeded")]
    RateLimitExceeded,
    #[error("concurrency limit exceeded")]
    ConcurrencyLimitExceeded,
    #[error("tenant inactive")]
    TenantInactive,
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("quota exceeded: {0}")]
    QuotaExceeded(String),
    #[error("scrape blocked by target site")]
    ScrapeBlocked,
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}
