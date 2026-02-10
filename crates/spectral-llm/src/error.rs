//! Error types for the LLM subsystem.

use thiserror::Error;

/// Errors that can occur during LLM operations.
#[derive(Error, Debug)]
pub enum LlmError {
    /// Provider not found
    #[error("provider not found: {provider_id}")]
    ProviderNotFound {
        /// Provider identifier
        provider_id: String,
    },

    /// No suitable provider available
    #[error("no suitable provider available for this request")]
    NoProviderAvailable,

    /// Provider communication error
    #[error("provider error ({provider}): {message}")]
    ProviderError {
        /// Provider name
        provider: String,
        /// Error message
        message: String,
    },

    /// API error with status code
    #[error("API error ({provider}): status {status}, {message}")]
    ApiError {
        /// Provider name
        provider: String,
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
    },

    /// Rate limit exceeded
    #[error("rate limit exceeded for {provider}: {message}")]
    RateLimitExceeded {
        /// Provider name
        provider: String,
        /// Error message
        message: String,
    },

    /// Invalid API key or authentication failure
    #[error("authentication failed for {provider}: {message}")]
    AuthenticationFailed {
        /// Provider name
        provider: String,
        /// Error message
        message: String,
    },

    /// Request validation error
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Response parsing error
    #[error("failed to parse response from {provider}: {message}")]
    ParseError {
        /// Provider name
        provider: String,
        /// Error message
        message: String,
    },

    /// PII detected when blocking strategy is enabled
    #[error("PII detected in request, blocking send: {details}")]
    PiiBlocked {
        /// Details about detected PII
        details: String,
    },

    /// Network error
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Serialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Timeout error
    #[error("request timed out after {seconds}s")]
    Timeout {
        /// Timeout duration in seconds
        seconds: u64,
    },

    /// Internal error
    #[error("internal error: {0}")]
    Internal(String),
}

/// Result type alias for LLM operations.
pub type Result<T> = std::result::Result<T, LlmError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = LlmError::ProviderNotFound {
            provider_id: "ollama".to_string(),
        };
        assert_eq!(err.to_string(), "provider not found: ollama");

        let err = LlmError::ApiError {
            provider: "anthropic".to_string(),
            status: 429,
            message: "Too Many Requests".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "API error (anthropic): status 429, Too Many Requests"
        );
    }

    #[test]
    fn test_pii_blocked_error() {
        let err = LlmError::PiiBlocked {
            details: "email detected: test@example.com".to_string(),
        };
        assert!(err.to_string().contains("PII detected"));
    }
}
