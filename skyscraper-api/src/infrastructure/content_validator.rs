use crate::domain::entity::ScrapeJobResult;

const MIN_VALID_HTML_LEN: usize = 1024;

pub struct ValidationResult {
    pub is_valid: bool,
    pub reason: Option<String>,
}

pub fn validate(result: &ScrapeJobResult) -> ValidationResult {
    if result.http_status >= 400 {
        return invalid(format!("HTTP {}", result.http_status));
    }

    if result.html_length < MIN_VALID_HTML_LEN {
        return invalid("page too small (<1KB), likely empty or error".into());
    }

    if let Some(ref html) = result.html {
        if let Some(reason) = detect_block(html) {
            return invalid(reason);
        }
    }

    ValidationResult {
        is_valid: true,
        reason: None,
    }
}

fn invalid(reason: String) -> ValidationResult {
    ValidationResult {
        is_valid: false,
        reason: Some(reason),
    }
}

fn detect_block(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();

    if lower.contains("cf-browser-verification") || lower.contains("cf-challenge-running") {
        return Some("cloudflare browser verification".into());
    }
    if title_contains(&lower, "just a moment") || title_contains(&lower, "attention required") {
        return Some("cloudflare challenge page".into());
    }
    if lower.contains("px-captcha") {
        return Some("perimeterx captcha".into());
    }
    if lower.contains("hcaptcha") || lower.contains("h-captcha") {
        return Some("hcaptcha challenge".into());
    }
    if lower.contains("g-recaptcha") || lower.contains("recaptcha") {
        let has_trigger = lower.contains("challenge") || lower.contains("verify");
        if has_trigger {
            return Some("recaptcha challenge".into());
        }
    }
    if title_contains(&lower, "access denied") || title_contains(&lower, "403 forbidden") {
        return Some("access denied".into());
    }
    if lower.contains("captcha") && (lower.contains("challenge") || lower.contains("verify")) {
        return Some("captcha detected".into());
    }

    None
}

fn title_contains(html: &str, needle: &str) -> bool {
    let Some(start) = html.find("<title") else {
        return false;
    };
    let Some(tag_end) = html[start..].find('>') else {
        return false;
    };
    let content_start = start + tag_end + 1;
    let end = html[content_start..]
        .find("</title")
        .map_or(html.len(), |i| content_start + i);
    html[content_start..end].contains(needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entity::ScrapeJobResult;

    fn make_result(status: i32, html_length: usize, html: Option<&str>) -> ScrapeJobResult {
        ScrapeJobResult {
            final_url: "https://example.com".into(),
            http_status: status,
            html_length,
            html: html.map(String::from),
            extracted: serde_json::Value::Null,
            screenshot_base64: None,
            blocked: false,
            proxy_used: None,
            profile_used: "default".into(),
            elapsed_ms: 100,
            attempts: 1,
        }
    }

    #[test]
    fn valid_page_passes() {
        let html = "x".repeat(2000);
        let r = make_result(200, 2000, Some(&html));
        assert!(validate(&r).is_valid);
    }

    #[test]
    fn http_error_fails() {
        let r = make_result(403, 5000, Some(&"x".repeat(5000)));
        let v = validate(&r);
        assert!(!v.is_valid);
        assert!(v.reason.unwrap().contains("HTTP 403"));
    }

    #[test]
    fn small_page_fails() {
        let r = make_result(200, 100, Some("tiny"));
        assert!(!validate(&r).is_valid);
    }

    #[test]
    fn cloudflare_detected() {
        let html = format!(
            "<html><head><title>Just a moment...</title></head><body>{}</body></html>",
            "x".repeat(2000)
        );
        let r = make_result(200, html.len(), Some(&html));
        let v = validate(&r);
        assert!(!v.is_valid);
        assert!(v.reason.unwrap().contains("cloudflare"));
    }

    #[test]
    fn captcha_with_verify_detected() {
        let html = format!(
            "<html><body><div class='captcha'>Please verify you are human</div>{}</body></html>",
            "x".repeat(2000)
        );
        let r = make_result(200, html.len(), Some(&html));
        assert!(!validate(&r).is_valid);
    }

    #[test]
    fn recaptcha_alone_not_flagged() {
        let html = format!(
            "<html><body>Some page with recaptcha badge{}</body></html>",
            "x".repeat(2000)
        );
        let r = make_result(200, html.len(), Some(&html));
        assert!(validate(&r).is_valid);
    }
}
