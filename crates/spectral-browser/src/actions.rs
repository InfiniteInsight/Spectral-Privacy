use crate::error::{BrowserError, Result};

/// Browser actions for automation
#[async_trait::async_trait]
pub trait BrowserActions {
    /// Navigate to a URL
    async fn navigate(&self, url: &str) -> Result<()>;

    /// Fill a form field by selector
    async fn fill_field(&self, selector: &str, value: &str) -> Result<()>;

    /// Click an element by selector
    async fn click(&self, selector: &str) -> Result<()>;

    /// Wait for a selector to appear
    async fn wait_for_selector(&self, selector: &str, timeout_ms: u64) -> Result<()>;

    /// Extract text from an element
    async fn extract_text(&self, selector: &str) -> Result<String>;

    /// Take a screenshot
    async fn screenshot(&self) -> Result<Vec<u8>>;
}

/// Helper to extract domain from URL
pub fn extract_domain(url: &str) -> Result<String> {
    let url = url::Url::parse(url)
        .map_err(|e| BrowserError::NavigationError(format!("Invalid URL: {}", e)))?;

    url.host_str()
        .ok_or_else(|| BrowserError::NavigationError("No host in URL".to_string()))
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://example.com/path").unwrap(),
            "example.com"
        );
        assert_eq!(
            extract_domain("http://subdomain.example.com:8080/path").unwrap(),
            "subdomain.example.com"
        );
    }

    #[test]
    fn test_extract_domain_invalid() {
        assert!(extract_domain("not-a-url").is_err());
    }
}
