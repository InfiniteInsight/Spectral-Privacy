use spectral_core::BrokerId;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("CAPTCHA required for broker {broker_id}")]
    CaptchaRequired { broker_id: BrokerId },

    #[error("Rate limited for broker {broker_id}, retry after {retry_after:?}")]
    RateLimited {
        broker_id: BrokerId,
        retry_after: Duration,
    },

    #[error("Broker site down: {broker_id}, HTTP {http_status}")]
    BrokerSiteDown {
        broker_id: BrokerId,
        http_status: u16,
    },

    #[error("Selectors outdated for broker {broker_id}: {reason}")]
    SelectorsOutdated { broker_id: BrokerId, reason: String },

    #[error("Insufficient profile data for broker {broker_id}, missing: {missing_fields:?}")]
    InsufficientProfileData {
        broker_id: BrokerId,
        missing_fields: Vec<String>,
    },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Browser error: {0}")]
    Browser(#[from] spectral_browser::BrowserError),

    #[error("Broker error: {0}")]
    Broker(#[from] spectral_broker::BrokerError),
}

pub type Result<T> = std::result::Result<T, ScanError>;
