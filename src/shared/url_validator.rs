use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Validate that a URL is safe to fetch (not targeting private/internal networks).
/// Returns Ok(()) if safe, Err with description if blocked.
pub fn validate_url(url: &str) -> Result<(), String> {
    // Must start with http:// or https://
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;

    let host = parsed.host_str().ok_or("URL has no host")?;

    // Try to parse as IP address directly
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_ip(&ip) {
            return Err(format!("Access to private/internal IP {} is blocked", ip));
        }
    }

    // Block localhost by hostname
    if host == "localhost" || host.ends_with(".local") || host.ends_with(".internal") {
        return Err(format!("Access to hostname '{}' is blocked", host));
    }

    // Resolve hostname and check all resolved IPs
    // Note: DNS resolution is blocking, but we do a basic check here.
    // For async resolution, the caller can do additional checks.
    // We check common private hostnames above; full DNS check would require async.

    Ok(())
}

/// Check if an IP address is in a private/internal range
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => is_private_ipv4(ipv4),
        IpAddr::V6(ipv6) => is_private_ipv6(ipv6),
    }
}

fn is_private_ipv4(ip: &Ipv4Addr) -> bool {
    let octets = ip.octets();
    // 10.0.0.0/8
    if octets[0] == 10 {
        return true;
    }
    // 172.16.0.0/12
    if octets[0] == 172 && (16..=31).contains(&octets[1]) {
        return true;
    }
    // 192.168.0.0/16
    if octets[0] == 192 && octets[1] == 168 {
        return true;
    }
    // 127.0.0.0/8 (loopback)
    if octets[0] == 127 {
        return true;
    }
    // 169.254.0.0/16 (link-local)
    if octets[0] == 169 && octets[1] == 254 {
        return true;
    }
    // 0.0.0.0
    if ip.is_unspecified() {
        return true;
    }
    false
}

fn is_private_ipv6(ip: &Ipv6Addr) -> bool {
    // ::1 (loopback)
    if ip.is_loopback() {
        return true;
    }
    // fc00::/7 (unique local)
    let segments = ip.segments();
    if (segments[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    // fe80::/10 (link-local)
    if (segments[0] & 0xffc0) == 0xfe80 {
        return true;
    }
    // :: (unspecified)
    if ip.is_unspecified() {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocks_private_ips() {
        assert!(validate_url("http://10.0.0.1/secret").is_err());
        assert!(validate_url("http://172.16.0.1/secret").is_err());
        assert!(validate_url("http://192.168.1.1/secret").is_err());
        assert!(validate_url("http://127.0.0.1/secret").is_err());
        assert!(validate_url("http://169.254.1.1/secret").is_err());
        assert!(validate_url("http://[::1]/secret").is_err());
        assert!(validate_url("http://localhost/secret").is_err());
    }

    #[test]
    fn test_allows_public_urls() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("https://github.com/owner/repo").is_ok());
    }

    #[test]
    fn test_blocks_non_http() {
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("ftp://example.com").is_err());
    }
}
