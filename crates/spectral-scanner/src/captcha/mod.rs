//! CAPTCHA solving integration for automated browser interactions.
//!
//! Supports multiple CAPTCHA solving services (2Captcha, `AntiCaptcha`)
//! via configurable API keys. Falls back to interactive solving when
//! no API key is configured.
//!
//! ## Supported CAPTCHA Types
//! - Google reCAPTCHA v2 (invisible and checkbox)
//! - reCAPTCHA v3 (score-based)
//! - hCaptcha
//! - Cloudflare Turnstile
//! - `FunCaptcha` (Arc2d)
//!
//! ## Configuration
//! Set environment variables or configure in broker TOML:
//! - `SPECTRAL_CAPTCHA_SERVICE`: `2captcha`, `antigate`, or `none`
//! - `SPECTRAL_CAPTCHA_API_KEY`: API key for the selected service
//! - `SPECTRAL_CAPTCHA_TIMEOUT`: Max seconds per solve (default: 120)

mod error;
mod solver;

pub use error::CaptchaError;
pub use error::CaptchaService;
pub use solver::CaptchaSolver;

/// Default timeout for CAPTCHA solving in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Default polling interval in seconds.
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;
