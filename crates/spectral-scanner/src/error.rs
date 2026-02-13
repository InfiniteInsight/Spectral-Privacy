//! Error types for the scanner module.

use spectral_core::BrokerId;
use thiserror::Error;

/// Result type for scanner operations.
pub type Result<T> = std::result::Result<T, ScanError>;

/// Errors that can occur during scanning operations.
#[derive(Debug, Error)]
pub enum ScanError {
    /// Browser automation error
    #[error("browser error: {0}")]
    Browser(#[from] spectral_browser::BrowserError),

    /// Database error
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Broker configuration error
    #[error("broker error: {0}")]
    Broker(#[from] spectral_broker::BrokerError),

    /// Parsing/scraping error
    #[error("parse error: {0}")]
    Parse(String),

    /// CAPTCHA challenge detected
    #[error("CAPTCHA required for broker {broker_id}")]
    CaptchaRequired {
        /// The broker that requires CAPTCHA
        broker_id: BrokerId,
    },

    /// Rate limit exceeded
    #[error("rate limited for broker {broker_id}, retry after {retry_after:?}")]
    RateLimited {
        /// The broker that rate limited us
        broker_id: BrokerId,
        /// Duration to wait before retrying
        retry_after: std::time::Duration,
    },

    /// Profile missing required fields
    #[error("profile missing required fields: {0:?}")]
    MissingRequiredFields(Vec<String>),

    /// No result selectors configured
    #[error("no result selectors configured for broker {0}")]
    NoResultSelectors(BrokerId),
}
