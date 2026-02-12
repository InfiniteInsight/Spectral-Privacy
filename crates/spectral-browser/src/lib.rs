//! Browser automation engine for JavaScript-heavy sites.
//!
//! Provides headless browser control with anti-fingerprinting
//! and rate limiting for broker interaction.

pub mod actions;
pub mod engine;
pub mod error;
pub mod fingerprint;

pub use actions::BrowserActions;
pub use engine::BrowserEngine;
pub use error::{BrowserError, Result};
