pub mod admin;
pub mod auth;
pub mod crawl;
pub mod error;
pub mod health;
pub mod jobs;
pub mod jobs_extra;
pub mod middleware;
pub mod pagination;
pub mod results;
pub mod tenants;
pub mod usage;

use std::sync::Arc;

use axum::routing::{get, patch, post};
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::domain::port::{
    CarrierClient, JobQueue, JobRepository, ResponseCache, ScrapeService, TenantRepository,
    UsageRepository,
};
use crate::infrastructure::metrics::MetricsCollector;

pub struct ParallaxConfig {
    pub proxy_host: String,
    pub proxy_port: u16,
    pub proxy_pass: String,
}

pub struct AppState {
    pub tenant_repo: Arc<dyn TenantRepository>,
    pub job_repo: Arc<dyn JobRepository>,
    pub usage_repo: Arc<dyn UsageRepository>,
    pub queue: Arc<dyn JobQueue>,
    pub carrier_client: Arc<dyn CarrierClient>,
    pub scrape_service: Arc<dyn ScrapeService>,
    pub response_cache: Arc<dyn ResponseCache>,
    pub parallax_config: ParallaxConfig,
    pub admin_key: String,
    pub metrics: Arc<MetricsCollector>,
}

pub fn router(state: Arc<AppState>) -> Router {
    let admin_routes = Router::new()
        .route(
            "/tenants",
            get(tenants::list_tenants).post(tenants::create_tenant),
        )
        .route("/tenants/{id}", patch(tenants::update_tenant))
        .route(
            "/tenants/{id}/regenerate-key",
            post(tenants::regenerate_key),
        )
        .route("/queue/stats", get(admin::queue_stats))
        .route("/queue/retry-dead", post(admin::retry_dead))
        .route("/carriers", get(admin::list_carriers))
        .route("/metrics", get(admin::metrics))
        .route("/cache", get(admin::cache_stats).delete(admin::cache_flush))
        .route("/jobs", get(admin::list_all_jobs));

    let authenticated_routes = Router::new()
        .route("/scrape", post(jobs_extra::scrape_sync))
        .route("/jobs", get(jobs::list_jobs).post(jobs::create_job))
        .route("/jobs/batch", post(jobs_extra::create_batch))
        .route("/jobs/{id}", get(jobs::get_job).delete(jobs::cancel_job))
        .route("/jobs/{id}/retry", post(jobs_extra::retry_job))
        .route("/jobs/{id}/priority", patch(jobs_extra::update_priority))
        .route("/results/{job_id}", get(results::get_result))
        .route("/results/{job_id}/screenshot", get(results::get_screenshot))
        .route("/usage", get(usage::get_usage))
        .route("/crawl", post(crawl::create_crawl));

    Router::new()
        .route("/api/v1/health", get(health::health))
        .route("/api/v1/health/ready", get(health::readiness))
        .nest("/api/v1/admin", admin_routes)
        .nest("/api/v1", authenticated_routes)
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn(middleware::request_id))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
