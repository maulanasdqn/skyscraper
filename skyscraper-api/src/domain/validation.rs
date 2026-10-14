use super::error::DomainError;

pub fn validate_scrape_url(url: &str) -> Result<(), DomainError> {
    if url.is_empty() {
        return Err(DomainError::InvalidInput("url is required".into()));
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
}
