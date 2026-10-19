use std::sync::Arc;

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::watch;
use tracing::info;
use tracing_subscriber::EnvFilter;

use skyscraper::adapter::postgres::{PgJobRepository, PgTenantRepository, PgUsageRepository};
use skyscraper::domain::port::{JobQueue, ProfileSelection, ProxySelection, ResponseCache};
use skyscraper::infrastructure::cache::RedisResponseCache;
use skyscraper::infrastructure::clients::cosmium::CosmiumClient;
use skyscraper::infrastructure::clients::parallax::ParallaxClient;
use skyscraper::infrastructure::dedup::InflightDedup;
use skyscraper::infrastructure::domain_throttle::DomainThrottle;
use skyscraper::infrastructure::intelligence::profile_rotator::ProfileRotator;
use skyscraper::infrastructure::intelligence::proxy_selector::ProxySelector;
use skyscraper::infrastructure::metrics::MetricsCollector;
use skyscraper::infrastructure::queue::redis_queue::RedisQueue;
use skyscraper::infrastructure::queue::worker::Worker;
use skyscraper::infrastructure::webhook::WebhookDelivery;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn required_env(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("{key} must be set"))
}

fn init_logging() -> Result<()> {
    let filter = EnvFilter::from_default_env().add_directive("skyscraper=info".parse()?);
    let json_logs = env_or("LOG_FORMAT", "text") == "json";

    if json_logs {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging()?;

    let database_url = required_env("DATABASE_URL")?;
    let redis_url = env_or("REDIS_URL", "redis://localhost:6379");
    let parallax_url = env_or("PARALLAX_URL", "http://localhost:3001");
    let parallax_proxy_host = env_or("PARALLAX_PROXY_HOST", "localhost");
    let parallax_proxy_port: u16 = env_or("PARALLAX_PROXY_PORT", "1080").parse()?;
    let proxy_pass = required_env("PROXY_PASS")?;
    let cosmium_url = env_or("COSMIUM_URL", "http://localhost:3000");
    let cosmium_api_key = std::env::var("COSMIUM_API_KEY").ok();
    let concurrency: usize = env_or("WORKER_CONCURRENCY", "4").parse()?;
    let cache_ttl: u64 = env_or("CACHE_TTL_SECS", "300").parse()?;
    let domain_max_rpm: u32 = env_or("DOMAIN_MAX_RPM", "30").parse()?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .context("failed to connect to database")?;

    let redis_queue = RedisQueue::new(&redis_url).await?;
    let queue: Arc<dyn JobQueue> = Arc::new(redis_queue);

    let redis_client =
        redis::Client::open(redis_url.as_str()).context("failed to open redis for cache")?;
    let cache_conn = redis::aio::ConnectionManager::new(redis_client)
        .await
        .context("failed to connect redis for cache")?;
    let response_cache: Arc<dyn ResponseCache> =
        Arc::new(RedisResponseCache::new(cache_conn.clone(), cache_ttl));

    let dedup = Arc::new(InflightDedup::new(cache_conn.clone()));
    let domain_throttle = Arc::new(DomainThrottle::new(cache_conn, domain_max_rpm));
    let metrics = Arc::new(MetricsCollector::new());

    let job_repo = Arc::new(PgJobRepository::new(pool.clone()));
    let tenant_repo = Arc::new(PgTenantRepository::new(pool.clone()));
    let usage_repo = Arc::new(PgUsageRepository::new(pool.clone()));
    let webhook = Arc::new(WebhookDelivery::new());

    let parallax_client: Arc<ParallaxClient> = Arc::new(ParallaxClient::new(&parallax_url));
    let cosmium_client: Arc<CosmiumClient> =
        Arc::new(CosmiumClient::new(&cosmium_url, cosmium_api_key));

    let proxy_selector: Arc<dyn ProxySelection> = Arc::new(ProxySelector::new(
        parallax_client,
        parallax_proxy_host,
        parallax_proxy_port,
        proxy_pass,
    ));
    let profile_rotator: Arc<dyn ProfileSelection> =
        Arc::new(ProfileRotator::new(cosmium_client.clone()));

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    info!("starting worker (concurrency={concurrency}, cache TTL={cache_ttl}s, domain limit={domain_max_rpm} rpm)");

    let worker = Arc::new(Worker::new(
        queue,
        job_repo,
        tenant_repo,
        usage_repo,
        proxy_selector,
        profile_rotator,
        cosmium_client,
        response_cache,
        dedup,
        domain_throttle,
        metrics.clone(),
        webhook,
        concurrency,
        shutdown_rx,
    ));

    let handle = tokio::spawn(async move { worker.run().await });

    tokio::signal::ctrl_c().await?;
    info!("shutting down...");
    let _ = shutdown_tx.send(true);
    let _ = handle.await;

    let snap = metrics.snapshot();
    info!(
        "final metrics — processed: {}, failed: {}, cache hits: {}, dedup coalesced: {}",
        snap.jobs_processed, snap.jobs_failed, snap.cache_hits, snap.dedup_coalesced
    );

    info!("worker stopped");
    Ok(())
}
