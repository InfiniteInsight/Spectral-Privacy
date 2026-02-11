use crate::error::{BrowserError, Result};
use crate::fingerprint::FingerprintConfig;
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiter per domain
#[derive(Debug)]
struct RateLimiter {
    last_access: HashMap<String, Instant>,
    min_delay: Duration,
}

impl RateLimiter {
    fn new(min_delay_ms: u64) -> Self {
        Self {
            last_access: HashMap::new(),
            min_delay: Duration::from_millis(min_delay_ms),
        }
    }

    #[allow(dead_code)]
    async fn check_and_update(&mut self, domain: &str) -> Result<()> {
        if let Some(last) = self.last_access.get(domain) {
            let elapsed = last.elapsed();
            if elapsed < self.min_delay {
                return Err(BrowserError::RateLimitExceeded(domain.to_string()));
            }
        }
        self.last_access.insert(domain.to_string(), Instant::now());
        Ok(())
    }
}

/// Browser automation engine
pub struct BrowserEngine {
    #[allow(dead_code)]
    browser: Browser,
    #[allow(dead_code)]
    fingerprint: FingerprintConfig,
    #[allow(dead_code)]
    rate_limiter: Arc<RwLock<RateLimiter>>,
}

impl BrowserEngine {
    /// Create a new browser engine with default configuration
    pub async fn new() -> Result<Self> {
        Self::with_fingerprint(FingerprintConfig::randomized()).await
    }

    /// Create a new browser engine with specific fingerprint
    pub async fn with_fingerprint(fingerprint: FingerprintConfig) -> Result<Self> {
        let config = BrowserConfig::builder()
            .no_sandbox()
            .build()
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

        // Spawn browser handler
        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                // Handle events if needed
                let _ = event;
            }
        });

        Ok(Self {
            browser,
            fingerprint,
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new(1000))), // 1 second default
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(100);

        // First access should succeed
        assert!(limiter.check_and_update("example.com").await.is_ok());

        // Immediate second access should fail
        assert!(limiter.check_and_update("example.com").await.is_err());

        // After delay, should succeed
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(limiter.check_and_update("example.com").await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_different_domains() {
        let mut limiter = RateLimiter::new(100);

        // Different domains should not interfere
        assert!(limiter.check_and_update("example.com").await.is_ok());
        assert!(limiter.check_and_update("other.com").await.is_ok());
    }
}
