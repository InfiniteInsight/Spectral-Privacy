use thiserror::Error;

pub type Result<T> = std::result::Result<T, BrowserError>;

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("chromium error: {0}")]
    ChromiumError(String),

    #[error("navigation failed: {0}")]
    NavigationError(String),

    #[error("selector not found: {0}")]
    SelectorNotFound(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("rate limit exceeded for domain: {0}")]
    RateLimitExceeded(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = BrowserError::NavigationError("page not found".to_string());
        assert_eq!(err.to_string(), "navigation failed: page not found");
    }

    #[test]
    fn test_rate_limit_error() {
        let err = BrowserError::RateLimitExceeded("example.com".to_string());
        assert!(err.to_string().contains("example.com"));
    }
}
