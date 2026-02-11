use crate::actions::{extract_domain, BrowserActions};
use crate::error::{BrowserError, Result};
use crate::fingerprint::FingerprintConfig;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::page::{Page, ScreenshotParams};
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

    /// Create a new page
    async fn new_page(&self) -> Result<Page> {
        self.browser
            .new_page("about:blank")
            .await
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))
    }
}

#[async_trait::async_trait]
impl BrowserActions for BrowserEngine {
    async fn navigate(&self, url: &str) -> Result<()> {
        // Check rate limit
        let domain = extract_domain(url)?;
        self.rate_limiter
            .write()
            .await
            .check_and_update(&domain)
            .await?;

        let page = self.new_page().await?;

        page.goto(url)
            .await
            .map_err(|e| BrowserError::NavigationError(e.to_string()))?;

        Ok(())
    }

    async fn fill_field(&self, selector: &str, value: &str) -> Result<()> {
        let page = self.new_page().await?;

        let element = page
            .find_element(selector)
            .await
            .map_err(|e| BrowserError::SelectorNotFound(e.to_string()))?;

        element
            .type_str(value)
            .await
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

        Ok(())
    }

    async fn click(&self, selector: &str) -> Result<()> {
        let page = self.new_page().await?;

        let element = page
            .find_element(selector)
            .await
            .map_err(|e| BrowserError::SelectorNotFound(e.to_string()))?;

        element
            .click()
            .await
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

        Ok(())
    }

    async fn wait_for_selector(&self, selector: &str, timeout_ms: u64) -> Result<()> {
        let page = self.new_page().await?;

        tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            page.find_element(selector),
        )
        .await
        .map_err(|_| BrowserError::Timeout(format!("Selector {} not found", selector)))?
        .map_err(|e| BrowserError::SelectorNotFound(e.to_string()))?;

        Ok(())
    }

    async fn extract_text(&self, selector: &str) -> Result<String> {
        let page = self.new_page().await?;

        let element = page
            .find_element(selector)
            .await
            .map_err(|e| BrowserError::SelectorNotFound(e.to_string()))?;

        let text = element
            .inner_text()
            .await
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?
            .unwrap_or_default();

        Ok(text)
    }

    async fn screenshot(&self) -> Result<Vec<u8>> {
        let page = self.new_page().await?;

        let screenshot = page
            .screenshot(ScreenshotParams::builder().build())
            .await
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

        Ok(screenshot)
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
