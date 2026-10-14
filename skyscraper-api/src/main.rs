use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use tracing::info;
use tracing_subscriber::EnvFilter;

use skyscraper::adapter::http::{self, AppState, ParallaxConfig};
use skyscraper::adapter::postgres::{PgJobRepository, PgTenantRepository, PgUsageRepository};
use skyscraper::application::scrape_service::ScrapeServiceImpl;
use skyscraper::domain::port::JobQueue;
use skyscraper::infrastructure::clients::parallax::ParallaxClient;
use skyscraper::infrastructure::queue::redis_queue::RedisQueue;

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
    let admin_key = required_env("API_ADMIN_KEY")?;
    let redis_url = env_or("REDIS_URL", "redis://localhost:6379");
    let parallax_url = env_or("PARALLAX_URL", "http://localhost:3001");
    let parallax_proxy_host = env_or("PARALLAX_PROXY_HOST", "localhost");
    let parallax_proxy_port: u16 = env_or("PARALLAX_PROXY_PORT", "1080").parse()?;
    let proxy_pass = required_env("PROXY_PASS")?;
    let listen: SocketAddr = env_or("API_LISTEN", "0.0.0.0:3002").parse()?;

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .context("failed to connect to database")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run migrations")?;

    let queue: Arc<dyn JobQueue> = Arc::new(RedisQueue::new(&redis_url).await?);

    let tenant_repo = Arc::new(PgTenantRepository::new(pool.clone()));
    let job_repo = Arc::new(PgJobRepository::new(pool.clone()));
    let usage_repo = Arc::new(PgUsageRepository::new(pool.clone()));
    let carrier_client = Arc::new(ParallaxClient::new(&parallax_url));
    let scrape_service = Arc::new(ScrapeServiceImpl::new(job_repo.clone(), queue.clone()));

    let state = Arc::new(AppState {
        tenant_repo,
        job_repo,
        usage_repo,
        queue,
        carrier_client,
        scrape_service,
        parallax_config: ParallaxConfig {
            proxy_host: parallax_proxy_host,
            proxy_port: parallax_proxy_port,
            proxy_pass,
        },
        admin_key,
    });

    let app = http::router(state);
    let listener = tokio::net::TcpListener::bind(listen).await?;

    info!("skyscraper-api listening on {listen}");
    axum::serve(listener, app).await?;

    Ok(())
}
