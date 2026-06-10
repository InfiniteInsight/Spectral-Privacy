use thiserror::Error;

pub type Result<T> = std::result::Result<T, BrowserError>;

/// Whether a browser error indicates a transient condition worth retrying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryKind {
    /// Error is transient — retry with backoff.
    Retry,
    /// Error is non-retryable — abort immediately.
    NoRetry,
}

#[derive(Debug, Clone, Error)]
pub enum BrowserError {
    #[error("chromium error: {0}")]
    ChromiumError(String),

    #[error("navigation failed: {0}")]
    NavigationError(String),

    #[error("HTTP {status} from server: {detail}")]
    HttpResponse { status: u16, detail: String },

    #[error("selector not found: {0}")]
    SelectorNotFound(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("rate limit exceeded for domain: {0}")]
    RateLimitExceeded(String),

    #[error("page content unavailable: {0}")]
    ContentUnavailable(String),
}

impl BrowserError {
    /// Returns whether this error is transient and worth retrying.
    pub fn retry_kind(&self) -> RetryKind {
        match self {
            Self::RateLimitExceeded(_) => RetryKind::Retry,
            Self::HttpResponse { status, .. } => match status {
                429 | 502 | 503 | 504 => RetryKind::Retry,
                _ => RetryKind::NoRetry,
            },
            Self::Timeout(_) => RetryKind::Retry,
            Self::ContentUnavailable(_) => RetryKind::Retry,
            Self::ChromiumError(_) | Self::NavigationError(_) | Self::SelectorNotFound(_) => {
                RetryKind::NoRetry
            }
        }
    }
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
