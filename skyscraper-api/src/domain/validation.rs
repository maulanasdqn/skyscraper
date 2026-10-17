use super::error::DomainError;

const MAX_URL_LENGTH: usize = 4096;
const MAX_EMAIL_LENGTH: usize = 320;
const MAX_WEBHOOK_URL_LENGTH: usize = 2048;

pub fn validate_scrape_url(url: &str) -> Result<(), DomainError> {
    if url.is_empty() {
        return Err(DomainError::InvalidInput("url is required".into()));
    }

    if url.len() > MAX_URL_LENGTH {
        return Err(DomainError::InvalidInput(
            format!("url exceeds maximum length of {MAX_URL_LENGTH}"),
        ));
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(DomainError::InvalidInput(
            "url must use http or https scheme".into(),
        ));
    }

    let host = extract_host(url).ok_or_else(|| DomainError::InvalidInput("invalid url".into()))?;

    if is_private_host(&host) {
        return Err(DomainError::InvalidInput(
            "scraping private/internal addresses is not allowed".into(),
        ));
    }

    Ok(())
}

pub fn validate_email(email: &str) -> Result<(), DomainError> {
    if email.is_empty() {
        return Err(DomainError::InvalidInput("email is required".into()));
    }
    if email.len() > MAX_EMAIL_LENGTH {
        return Err(DomainError::InvalidInput("email too long".into()));
    }
    if !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return Err(DomainError::InvalidInput("invalid email format".into()));
    }
    if email.contains(' ') || email.contains('\n') || email.contains('\r') {
        return Err(DomainError::InvalidInput("email contains invalid characters".into()));
    }
    Ok(())
}

pub fn validate_webhook_url(url: &str) -> Result<(), DomainError> {
    if url.is_empty() {
        return Ok(());
    }
    if url.len() > MAX_WEBHOOK_URL_LENGTH {
        return Err(DomainError::InvalidInput("webhook url too long".into()));
    }
    if !url.starts_with("https://") {
        return Err(DomainError::InvalidInput(
            "webhook url must use https".into(),
        ));
    }
    let host = extract_host(url)
        .ok_or_else(|| DomainError::InvalidInput("invalid webhook url".into()))?;
    if is_private_host(&host) {
        return Err(DomainError::InvalidInput(
            "webhook to private/internal addresses is not allowed".into(),
        ));
    }
    Ok(())
}

fn extract_host(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1)?;
    let host_port = after_scheme.split('/').next()?;
    let host = host_port.split(':').next()?;
    if host.is_empty() {
        return None;
    }
    Some(host.to_lowercase())
}

fn is_private_host(host: &str) -> bool {
    if host == "localhost" || host == "[::1]" {
        return true;
    }

    if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
        return ip.is_loopback()
            || ip.is_private()
            || ip.is_link_local()
            || ip.is_broadcast()
            || ip.is_unspecified()
            || is_cgnat(ip);
    }

    if is_internal_tld(host) {
        return true;
    }

    false
}

fn is_internal_tld(host: &str) -> bool {
    const INTERNAL_SUFFIXES: &[&str] = &[".local", ".internal", ".localhost"];
    INTERNAL_SUFFIXES
        .iter()
        .any(|s| host.len() >= s.len() && host[host.len() - s.len()..].eq_ignore_ascii_case(s))
}

fn is_cgnat(ip: std::net::Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (64..=127).contains(&octets[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_urls() {
        assert!(validate_scrape_url("https://example.com").is_ok());
        assert!(validate_scrape_url("http://example.com/path?q=1").is_ok());
        assert!(validate_scrape_url("https://sub.domain.co.uk/page").is_ok());
    }

    #[test]
    fn empty_url_rejected() {
        let err = validate_scrape_url("").unwrap_err();
        assert!(matches!(err, DomainError::InvalidInput(_)));
    }

    #[test]
    fn non_http_scheme_rejected() {
        assert!(validate_scrape_url("ftp://example.com").is_err());
        assert!(validate_scrape_url("file:///etc/passwd").is_err());
        assert!(validate_scrape_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn localhost_rejected() {
        assert!(validate_scrape_url("http://localhost/admin").is_err());
        assert!(validate_scrape_url("http://localhost:3000").is_err());
    }

    #[test]
    fn private_ip_rejected() {
        assert!(validate_scrape_url("http://127.0.0.1").is_err());
        assert!(validate_scrape_url("http://10.0.0.1").is_err());
        assert!(validate_scrape_url("http://192.168.1.1").is_err());
        assert!(validate_scrape_url("http://172.16.0.1").is_err());
    }

    #[test]
    fn cgnat_rejected() {
        assert!(validate_scrape_url("http://100.64.0.1").is_err());
        assert!(validate_scrape_url("http://100.127.255.255").is_err());
    }

    #[test]
    fn link_local_rejected() {
        assert!(validate_scrape_url("http://169.254.1.1").is_err());
    }

    #[test]
    fn internal_tlds_rejected() {
        assert!(validate_scrape_url("http://service.local").is_err());
        assert!(validate_scrape_url("http://api.internal").is_err());
        assert!(validate_scrape_url("http://test.localhost").is_err());
    }

    #[test]
    fn public_ips_allowed() {
        assert!(validate_scrape_url("http://8.8.8.8").is_ok());
        assert!(validate_scrape_url("http://1.1.1.1").is_ok());
    }

    #[test]
    fn url_length_limit() {
        let long = format!("https://example.com/{}", "a".repeat(5000));
        assert!(validate_scrape_url(&long).is_err());
    }

    #[test]
    fn valid_email() {
        assert!(validate_email("user@example.com").is_ok());
    }

    #[test]
    fn invalid_emails() {
        assert!(validate_email("").is_err());
        assert!(validate_email("noatsign").is_err());
        assert!(validate_email("@leading").is_err());
        assert!(validate_email("trailing@").is_err());
        assert!(validate_email("has space@x.com").is_err());
    }

    #[test]
    fn webhook_must_be_https() {
        assert!(validate_webhook_url("https://hooks.example.com/cb").is_ok());
        assert!(validate_webhook_url("http://hooks.example.com/cb").is_err());
        assert!(validate_webhook_url("").is_ok());
    }

    #[test]
    fn webhook_ssrf_blocked() {
        assert!(validate_webhook_url("https://localhost/cb").is_err());
        assert!(validate_webhook_url("https://127.0.0.1/cb").is_err());
        assert!(validate_webhook_url("https://10.0.0.1/cb").is_err());
        assert!(validate_webhook_url("https://169.254.169.254/metadata").is_err());
    }
}
