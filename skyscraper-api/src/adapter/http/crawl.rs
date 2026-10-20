use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use super::auth::AuthenticatedTenant;
use super::error::ApiResult;
use super::link_extract::{extract_base_url, extract_links};
use super::AppState;
use crate::domain::entity::{CrawlRequest, CrawlResponse, ScrapeJobRequest};
use crate::domain::job_status::JobStatus;

pub async fn create_crawl(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CrawlRequest>,
) -> ApiResult<(StatusCode, Json<CrawlResponse>)> {
    crate::domain::validation::validate_scrape_url(&body.start_url)?;

    let max_pages = body.max_pages.clamp(1, 100);
    let max_depth = body.max_depth.min(5);
    let crawl_id = uuid::Uuid::new_v4();
    let mut job_ids = Vec::with_capacity(max_pages as usize);

    let seed_request = build_request(&body, &body.start_url);
    let seed_job = state
        .scrape_service
        .submit_job(&tenant.0, &seed_request, 5, 2, None)
        .await?;
    job_ids.push(seed_job.id);

    if max_pages <= 1 {
        return Ok(ok(crawl_id, job_ids));
    }

    let html = wait_for_html(&state, seed_job.id).await;
    let base = extract_base_url(&body.start_url);
    let discovered = extract_links(&html, &base, &body.start_url, max_depth);

    for url in discovered {
        if job_ids.len() >= max_pages as usize {
            break;
        }
        let req = build_request(&body, &url);
        match state
            .scrape_service
            .submit_job(&tenant.0, &req, 4, 2, None)
            .await
        {
            Ok(job) => job_ids.push(job.id),
            Err(e) => tracing::warn!("crawl: failed to enqueue {url}: {e}"),
        }
    }

    Ok(ok(crawl_id, job_ids))
}

fn ok(crawl_id: uuid::Uuid, job_ids: Vec<uuid::Uuid>) -> (StatusCode, Json<CrawlResponse>) {
    let pages = job_ids.len() as u32;
    (
        StatusCode::CREATED,
        Json(CrawlResponse {
            crawl_id,
            pages_discovered: pages,
            job_ids,
        }),
    )
}

fn build_request(body: &CrawlRequest, url: &str) -> ScrapeJobRequest {
    ScrapeJobRequest {
        url: url.to_string(),
        extract: body.extract.clone(),
        script: None,
        workflow: vec![],
        screenshot: body.screenshot,
        include_html: body.include_html,
        wait_ms: 3000,
        headful: false,
        geo: body.geo.clone(),
        proxy_strategy: body.proxy_strategy.clone(),
        profile_strategy: body.profile_strategy.clone(),
    }
}

async fn wait_for_html(state: &AppState, job_id: uuid::Uuid) -> String {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(90);
    let mut interval = std::time::Duration::from_millis(200);

    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(interval).await;
        interval = (interval * 2).min(std::time::Duration::from_secs(2));

        if let Ok(Some(job)) = state.job_repo.find_by_id(job_id).await {
            match job.status {
                JobStatus::Completed => {
                    if let Some(result) = &job.result {
                        if let Some(html) = result.get("html").and_then(|v| v.as_str()) {
                            return html.to_string();
                        }
                    }
                    return String::new();
                }
                JobStatus::Failed | JobStatus::Dead => return String::new(),
                _ => {}
            }
        }
    }
    String::new()
}
