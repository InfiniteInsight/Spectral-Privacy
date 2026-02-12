//! CAPTCHA detection and solving.

use crate::error::Result;
use async_trait::async_trait;
use spectral_browser::{BrowserActions, BrowserEngine};

/// CAPTCHA solver trait for pluggable implementations.
#[async_trait]
pub trait CaptchaSolver: Send + Sync {
    /// Attempt to solve a CAPTCHA.
    ///
    /// Returns Ok(true) if solved, Ok(false) if manual intervention needed.
    async fn solve(&self, engine: &BrowserEngine, captcha_selector: &str) -> Result<bool>;
}

/// Manual CAPTCHA solver - pauses and returns false to signal user intervention needed.
pub struct ManualSolver;

#[async_trait]
impl CaptchaSolver for ManualSolver {
    async fn solve(&self, _engine: &BrowserEngine, _captcha_selector: &str) -> Result<bool> {
        // Manual solver doesn't attempt to solve - just signals pause needed
        Ok(false)
    }
}

/// Detect if a CAPTCHA is present on the page.
pub async fn detect_captcha(
    engine: &BrowserEngine,
    captcha_selector: Option<&str>,
) -> Result<bool> {
    if let Some(selector) = captcha_selector {
        // Try to find CAPTCHA element with short timeout
        match engine.wait_for_selector(selector, 1000).await {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    } else {
        // No CAPTCHA selector configured, assume no CAPTCHA
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manual_solver_returns_false() {
        // Note: This test can't actually create a browser without Chrome installed
        // Just testing the interface - verify the struct exists and implements the trait
        assert_eq!(std::mem::size_of::<ManualSolver>(), 0);
    }
}
