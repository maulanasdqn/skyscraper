pub mod admin;
pub mod auth;
pub mod error;
pub mod health;
pub mod jobs;
pub mod tenants;
pub mod usage;

use std::sync::Arc;

use axum::routing::{get, patch, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::domain::port::{
    CarrierClient, JobQueue, JobRepository, ScrapeService, TenantRepository, UsageRepository,
};

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
    pub parallax_config: ParallaxConfig,
    pub admin_key: String,
}

pub fn router(state: Arc<AppState>) -> Router {
    let admin_routes = Router::new()
        .route("/tenants", get(tenants::list_tenants).post(tenants::create_tenant))
        .route("/tenants/{id}", patch(tenants::update_tenant))
        .route(
            "/tenants/{id}/regenerate-key",
            post(tenants::regenerate_key),
        )
        .route("/queue/stats", get(admin::queue_stats))
        .route("/queue/retry-dead", post(admin::retry_dead))
        .route("/carriers", get(admin::list_carriers));

    let authenticated_routes = Router::new()
        .route("/scrape", post(jobs::scrape_sync))
        .route("/jobs", get(jobs::list_jobs).post(jobs::create_job))
        .route("/jobs/batch", post(jobs::create_batch))
        .route("/jobs/{id}", get(jobs::get_job).delete(jobs::cancel_job))
        .route("/usage", get(usage::get_usage));

    Router::new()
        .route("/api/v1/health", get(health::health))
        .nest("/api/v1/admin", admin_routes)
        .nest("/api/v1", authenticated_routes)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
