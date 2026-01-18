//! Office Suite Shared Networking
//!
//! HTTP/HTTPS client for Q-WEB and Q-MAIL.

use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;
use ureq::Agent;

// =============================================================================
// RESPONSE
// =============================================================================

/// HTTP Response
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub url: String,
}

impl Response {
    /// Get response body as string
    pub fn text(&self) -> Result<String, String> {
        String::from_utf8(self.body.clone()).map_err(|e| format!("Invalid UTF-8: {}", e))
    }

    /// Check if response is successful (2xx)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Get content type
    pub fn content_type(&self) -> Option<&str> {
        self.headers.get("content-type").map(|s| s.as_str())
    }

    /// Get redirect location if any
    pub fn redirect_location(&self) -> Option<&str> {
        if (300..400).contains(&self.status) {
            self.headers.get("location").map(|s| s.as_str())
        } else {
            None
        }
    }
}

// =============================================================================
// HTTP CLIENT
// =============================================================================

/// Simple HTTP client using ureq
pub struct HttpClient {
    agent: Agent,
    user_agent: String,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();

        Self {
            agent,
            user_agent: "Q-WEB/1.0 (Q-DOS Office Suite)".to_string(),
        }
    }

    /// Fetch a URL
    pub fn get(&self, url: &str) -> Result<Response, String> {
        let response = self
            .agent
            .get(url)
            .header("User-Agent", &self.user_agent)
            .call()
            .map_err(|e| format!("Request failed: {}", e))?;

        let status: u16 = response.status().into();
        let final_url = url.to_string();

        // Collect headers
        let mut headers = HashMap::new();
        for (name, value) in response.headers().iter() {
            if let Ok(v) = value.to_str() {
                headers.insert(name.as_str().to_lowercase(), v.to_string());
            }
        }

        // Read body
        let mut body = Vec::new();
        response
            .into_body()
            .into_reader()
            .read_to_end(&mut body)
            .map_err(|e| format!("Failed to read body: {}", e))?;

        Ok(Response {
            status,
            headers,
            body,
            url: final_url,
        })
    }

    /// Fetch with redirect following (up to max_redirects)
    pub fn get_follow_redirects(
        &self,
        url: &str,
        max_redirects: usize,
    ) -> Result<Response, String> {
        let mut current_url = url.to_string();
        let mut redirects = 0;

        loop {
            let response = self.get(&current_url)?;

            if let Some(location) = response.redirect_location() {
                if redirects >= max_redirects {
                    return Err(format!("Too many redirects ({})", max_redirects));
                }

                // Handle relative redirects
                current_url = if location.starts_with('/') {
                    // Relative to host
                    if let Some(host_end) = current_url.find("://").map(|i| {
                        current_url[i + 3..]
                            .find('/')
                            .map(|j| i + 3 + j)
                            .unwrap_or(current_url.len())
                    }) {
                        format!("{}{}", &current_url[..host_end], location)
                    } else {
                        location.to_string()
                    }
                } else if location.starts_with("http://") || location.starts_with("https://") {
                    location.to_string()
                } else {
                    // Relative to current path
                    if let Some(last_slash) = current_url.rfind('/') {
                        format!("{}/{}", &current_url[..last_slash], location)
                    } else {
                        location.to_string()
                    }
                };

                redirects += 1;
                continue;
            }

            return Ok(response);
        }
    }
}

// =============================================================================
// URL UTILITIES
// =============================================================================

/// Parse URL components
pub fn parse_url(url: &str) -> Option<UrlParts> {
    let url = url.trim();

    // Add default scheme if missing
    let url = if !url.contains("://") {
        format!("https://{}", url)
    } else {
        url.to_string()
    };

    // Parse scheme
    let (scheme, rest) = url.split_once("://")?;
    let scheme = scheme.to_lowercase();

    // Parse host and path
    let (host, path) = if let Some(slash_idx) = rest.find('/') {
        (&rest[..slash_idx], &rest[slash_idx..])
    } else {
        (rest, "/")
    };

    // Parse host and port
    let (host, port) = if let Some((h, p)) = host.rsplit_once(':') {
        (h.to_string(), p.parse().ok())
    } else {
        (host.to_string(), None)
    };

    Some(UrlParts {
        scheme,
        host,
        port,
        path: path.to_string(),
    })
}

/// URL components
#[derive(Debug, Clone)]
pub struct UrlParts {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
}

impl std::fmt::Display for UrlParts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let port_str = self.port.map(|p| format!(":{}", p)).unwrap_or_default();
        write!(
            f,
            "{}://{}{}{}",
            self.scheme, self.host, port_str, self.path
        )
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url() {
        let url = parse_url("https://example.com/page").unwrap();
        assert_eq!(url.scheme, "https");
        assert_eq!(url.host, "example.com");
        assert_eq!(url.path, "/page");
    }

    #[test]
    fn test_parse_url_with_port() {
        let url = parse_url("http://localhost:8080/api").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host, "localhost");
        assert_eq!(url.port, Some(8080));
        assert_eq!(url.path, "/api");
    }

    #[test]
    fn test_parse_url_adds_scheme() {
        let url = parse_url("example.com").unwrap();
        assert_eq!(url.scheme, "https");
        assert_eq!(url.host, "example.com");
    }
}
