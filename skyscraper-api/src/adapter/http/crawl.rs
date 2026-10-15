use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use super::auth::AuthenticatedTenant;
use super::error::ApiResult;
use super::AppState;
use crate::domain::entity::{CrawlRequest, CrawlResponse, ScrapeJobRequest};

pub async fn create_crawl(
    tenant: AuthenticatedTenant,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CrawlRequest>,
) -> ApiResult<(StatusCode, Json<CrawlResponse>)> {
    crate::domain::validation::validate_scrape_url(&body.start_url)?;

    let max_pages = body.max_pages.min(100);

    let mut job_ids = Vec::with_capacity(max_pages as usize);
    let crawl_id = uuid::Uuid::new_v4();

    let request = ScrapeJobRequest {
        url: body.start_url.clone(),
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
    };

    let job = state
        .scrape_service
        .submit_job(&tenant.0, &request, 5, 2, None)
        .await?;
    job_ids.push(job.id);

    if max_pages > 1 {
        let discovered = discover_links(&body.start_url, max_pages, body.max_depth);

        for url in discovered {
            if job_ids.len() >= max_pages as usize {
                break;
            }

            let page_request = ScrapeJobRequest {
                url,
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
            };

            match state
                .scrape_service
                .submit_job(&tenant.0, &page_request, 4, 2, None)
                .await
            {
                Ok(job) => job_ids.push(job.id),
                Err(e) => {
                    tracing::warn!("crawl: failed to enqueue {}: {e}", page_request.url);
                }
            }
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(CrawlResponse {
            crawl_id,
            pages_discovered: job_ids.len() as u32,
            job_ids,
        }),
    ))
}

fn discover_links(start_url: &str, max_pages: u32, max_depth: u32) -> Vec<String> {
    let base = extract_base_url(start_url);
    let mut urls = Vec::new();

    let common_paths = [
        "/about",
        "/contact",
        "/products",
        "/services",
        "/blog",
        "/pricing",
        "/faq",
        "/terms",
        "/privacy",
        "/sitemap.xml",
    ];

    for (i, path) in common_paths.iter().enumerate() {
        if urls.len() >= (max_pages as usize).saturating_sub(1) {
            break;
        }
        if max_depth == 0 {
            break;
        }
        let _ = i;
        let url = format!("{base}{path}");
        if url != start_url {
            urls.push(url);
        }
    }

    urls
}

fn extract_base_url(url: &str) -> String {
    let after_scheme = url.split("://").collect::<Vec<_>>();
    if after_scheme.len() < 2 {
        return url.to_string();
    }
    let scheme = after_scheme[0];
    let rest = after_scheme[1];
    let host = rest.split('/').next().unwrap_or(rest);
    format!("{scheme}://{host}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_base_url_simple() {
        assert_eq!(
            extract_base_url("https://example.com/page"),
            "https://example.com"
        );
    }

    #[test]
    fn extract_base_url_with_port() {
        assert_eq!(
            extract_base_url("http://localhost:3000/api"),
            "http://localhost:3000"
        );
    }

    #[test]
    fn discover_links_generates_paths() {
        let links = discover_links("https://example.com", 5, 1);
        assert!(!links.is_empty());
        assert!(links.len() <= 4);
        for link in &links {
            assert!(link.starts_with("https://example.com/"));
        }
    }

    #[test]
    fn discover_links_depth_zero_returns_empty() {
        let links = discover_links("https://example.com", 10, 0);
        assert!(links.is_empty());
    }
}
