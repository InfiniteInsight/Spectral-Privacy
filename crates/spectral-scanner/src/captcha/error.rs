//! CAPTCHA solving error types.

use std::str::FromStr;

use thiserror::Error;

/// Result type for CAPTCHA solving operations.
pub type Result<T> = std::result::Result<T, CaptchaError>;

/// Supported CAPTCHA solving services.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(missing_docs)]
pub enum CaptchaService {
    #[default]
    None,
    /// 2Captcha API (see <https://2captcha.com>)
    TwoCaptcha,
    /// Anti-Captcha API (see <https://anti-captcha.com>)
    AntiCaptcha,
}

impl FromStr for CaptchaService {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "2captcha" | "2-captcha" | "2captcha.com" => Ok(Self::TwoCaptcha),
            "antigate" | "anti-captcha" | "antigate.com" | "anti-captcha.com" => {
                Ok(Self::AntiCaptcha)
            }
            _ => Err(()),
        }
    }
}

impl CaptchaService {
    /// Parse from a string slice.
    pub fn parse(s: &str) -> Self {
        Self::from_str(s).unwrap_or_default()
    }

    /// Returns the service name for API endpoints.
    #[must_use]
    pub fn api_name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::TwoCaptcha => "2captcha",
            Self::AntiCaptcha => "antigate",
        }
    }
}

impl std::fmt::Display for CaptchaService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.api_name())
    }
}

/// Errors that can occur during CAPTCHA solving.
#[derive(Debug, Error)]
pub enum CaptchaError {
    /// No CAPTCHA solving service configured.
    #[error("no CAPTCHA solving service configured; set SPECTRAL_CAPTCHA_SERVICE and SPECTRAL_CAPTCHA_API_KEY")]
    NoServiceConfigured,

    /// API key is missing for the configured service.
    #[error("SPECTRAL_CAPTCHA_API_KEY environment variable is not set")]
    MissingApiKey,

    /// The CAPTCHA solving service returned an error.
    #[error("captcha service ({service}) error: {message}")]
    ServiceError {
        /// The service that returned the error.
        service: String,
        /// The error message from the service.
        message: String,
    },

    /// The CAPTCHA solve request timed out.
    #[error("CAPTCHA solve timed out after {timeout_secs}s")]
    Timeout {
        /// Timeout duration in seconds.
        timeout_secs: u64,
    },

    /// The CAPTCHA solving service returned an invalid or missing token.
    #[error("invalid CAPTCHA token from service")]
    InvalidToken,

    /// The browser failed to inject the CAPTCHA token.
    #[error("failed to inject token into page: {0}")]
    InjectionError(String),

    /// Underlying HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Failed to parse the CAPTCHA solving service response.
    #[error("failed to parse service response: {0}")]
    ParseError(#[source] Box<dyn std::error::Error + Send + Sync>),
}
