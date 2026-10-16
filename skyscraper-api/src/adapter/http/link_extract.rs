use std::collections::HashSet;

pub fn extract_links(
    html: &str,
    base_url: &str,
    start_url: &str,
    max_depth: u32,
) -> Vec<String> {
    if max_depth == 0 || html.is_empty() {
        return Vec::new();
    }

    let mut seen = HashSet::new();
    seen.insert(start_url.to_string());

    let base_lower = base_url.to_lowercase();

    for cap in regex_href(html) {
        let href = cap.trim();
        if href.is_empty()
            || href.starts_with('#')
            || href.starts_with("javascript:")
            || href.starts_with("mailto:")
            || href.starts_with("tel:")
        {
            continue;
        }

        let absolute = if href.starts_with("http://") || href.starts_with("https://") {
            href.to_string()
        } else if href.starts_with("//") {
            format!("https:{href}")
        } else if href.starts_with('/') {
            format!("{base_url}{href}")
        } else {
            format!("{base_url}/{href}")
        };

        let cleaned = absolute.split('#').next().unwrap_or(&absolute);
        let cleaned = cleaned.trim_end_matches('/').to_string();

        if !cleaned.to_lowercase().starts_with(&base_lower) {
            continue;
        }

        if is_asset(&cleaned) {
            continue;
        }

        seen.insert(cleaned);
    }

    seen.remove(start_url);
    let without_slash = start_url.trim_end_matches('/');
    seen.remove(without_slash);

    seen.into_iter().collect()
}

pub fn extract_base_url(url: &str) -> String {
    let after_scheme: Vec<&str> = url.split("://").collect();
    if after_scheme.len() < 2 {
        return url.to_string();
    }
    let scheme = after_scheme[0];
    let rest = after_scheme[1];
    let host = rest.split('/').next().unwrap_or(rest);
    format!("{scheme}://{host}")
}

fn regex_href(html: &str) -> Vec<&str> {
    let mut results = Vec::new();
    let search = "href=\"";
    let mut pos = 0;
    while let Some(start) = html[pos..].find(search) {
        let href_start = pos + start + search.len();
        if let Some(end) = html[href_start..].find('"') {
            results.push(&html[href_start..href_start + end]);
            pos = href_start + end + 1;
        } else {
            break;
        }
    }
    pos = 0;
    let search2 = "href='";
    while let Some(start) = html[pos..].find(search2) {
        let href_start = pos + start + search2.len();
        if let Some(end) = html[href_start..].find('\'') {
            results.push(&html[href_start..href_start + end]);
            pos = href_start + end + 1;
        } else {
            break;
        }
    }
    results
}

fn is_asset(url: &str) -> bool {
    let lower = url.to_lowercase();
    let exts = [
        ".jpg", ".jpeg", ".png", ".gif", ".svg", ".webp", ".ico", ".css",
        ".js", ".pdf", ".zip", ".woff", ".woff2", ".ttf", ".otf", ".mp4",
        ".mp3", ".avi", ".xml",
    ];
    exts.iter().any(|ext| lower.ends_with(ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_base_url() {
        assert_eq!(
            extract_base_url("https://example.com/page"),
            "https://example.com"
        );
        assert_eq!(
            extract_base_url("http://localhost:3000/api"),
            "http://localhost:3000"
        );
    }

    #[test]
    fn extracts_links_from_html() {
        let html = r##"
            <a href="/about">About</a>
            <a href="/products">Products</a>
            <a href="https://other.com/page">External</a>
            <a href="#section">Anchor</a>
            <a href="/image.png">Image</a>
            <a href='/single-quotes'>Single</a>
        "##;
        let links = extract_links(html, "https://example.com", "https://example.com", 2);
        assert!(links.contains(&"https://example.com/about".to_string()));
        assert!(links.contains(&"https://example.com/products".to_string()));
        assert!(links.contains(&"https://example.com/single-quotes".to_string()));
        assert!(!links.iter().any(|l| l.contains("other.com")));
        assert!(!links.iter().any(|l| l.ends_with(".png")));
    }

    #[test]
    fn depth_zero_returns_empty() {
        let html = r##"<a href="/page">Page</a>"##;
        let links = extract_links(html, "https://example.com", "https://example.com", 0);
        assert!(links.is_empty());
    }

    #[test]
    fn deduplicates_links() {
        let html = r##"
            <a href="/about">About</a>
            <a href="/about">About again</a>
            <a href="/about/">Trailing slash</a>
        "##;
        let links = extract_links(html, "https://example.com", "https://example.com", 2);
        let about_count = links.iter().filter(|l| l.contains("about")).count();
        assert_eq!(about_count, 1);
    }
}
