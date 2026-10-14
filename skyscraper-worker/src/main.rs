use std::sync::Arc;

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::watch;
use tracing::info;
use tracing_subscriber::EnvFilter;

use skyscraper::adapter::postgres::{PgJobRepository, PgUsageRepository};
use skyscraper::infrastructure::clients::cosmium::CosmiumClient;
use skyscraper::infrastructure::clients::parallax::ParallaxClient;
use skyscraper::infrastructure::intelligence::profile_rotator::ProfileRotator;
use skyscraper::infrastructure::intelligence::proxy_selector::ProxySelector;
use skyscraper::domain::port::{JobQueue, ProfileSelection, ProxySelection};
use skyscraper::infrastructure::queue::redis_queue::RedisQueue;
use skyscraper::infrastructure::queue::worker::Worker;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn required_env(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("{key} must be set"))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("skyscraper=info".parse()?))
        .init();

    let database_url = required_env("DATABASE_URL")?;
    let redis_url = env_or("REDIS_URL", "redis://localhost:6379");
    let parallax_url = env_or("PARALLAX_URL", "http://localhost:3001");
    let parallax_proxy_host = env_or("PARALLAX_PROXY_HOST", "localhost");
    let parallax_proxy_port: u16 = env_or("PARALLAX_PROXY_PORT", "1080").parse()?;
    let proxy_pass = required_env("PROXY_PASS")?;
    let cosmium_url = env_or("COSMIUM_URL", "http://localhost:3000");
    let concurrency: usize = env_or("WORKER_CONCURRENCY", "4").parse()?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .context("failed to connect to database")?;

    let queue: Arc<dyn JobQueue> = Arc::new(RedisQueue::new(&redis_url).await?);
    let job_repo = Arc::new(PgJobRepository::new(pool.clone()));
    let usage_repo = Arc::new(PgUsageRepository::new(pool.clone()));

    let parallax_client: Arc<ParallaxClient> = Arc::new(ParallaxClient::new(&parallax_url));
    let cosmium_client: Arc<CosmiumClient> = Arc::new(CosmiumClient::new(&cosmium_url));

    let proxy_selector: Arc<dyn ProxySelection> = Arc::new(ProxySelector::new(
        parallax_client,
        parallax_proxy_host,
        parallax_proxy_port,
        proxy_pass,
    ));
    let profile_rotator: Arc<dyn ProfileSelection> =
        Arc::new(ProfileRotator::new(cosmium_client.clone()));

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    info!("starting {concurrency} workers");

    let mut handles = Vec::new();
    for i in 0..concurrency {
        let worker = Worker::new(
            queue.clone(),
            job_repo.clone(),
            usage_repo.clone(),
            proxy_selector.clone(),
            profile_rotator.clone(),
            cosmium_client.clone(),
            shutdown_rx.clone(),
        );
        handles.push(tokio::spawn(async move {
            info!("worker {i} started");
            worker.run().await;
        }));
    }

    tokio::signal::ctrl_c().await?;
    info!("shutting down workers...");
    let _ = shutdown_tx.send(true);

    for handle in handles {
        let _ = handle.await;
    }

    info!("all workers stopped");
    Ok(())
}
