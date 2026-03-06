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
    browser: Browser,
    #[allow(dead_code)]
    fingerprint: FingerprintConfig,
    rate_limiter: Arc<RwLock<RateLimiter>>,
    current_page: Arc<RwLock<Option<Page>>>,
}

/// Helper: Detect Chrome installation path on Windows
#[cfg(target_os = "windows")]
fn detect_chrome_path() -> Option<String> {
    let possible_paths = vec![
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Users\%USERNAME%\AppData\Local\Google\Chrome\Application\chrome.exe",
    ];

    for path_template in possible_paths {
        let path =
            path_template.replace("%USERNAME%", &std::env::var("USERNAME").unwrap_or_default());
        if std::path::Path::new(&path).exists() {
            tracing::info!("Auto-detected Chrome at: {}", path);
            return Some(path);
        }
    }
    None
}

/// Helper: Check if running in WSL2 environment
#[cfg(target_os = "linux")]
fn is_wsl2() -> bool {
    std::fs::read_to_string("/proc/version")
        .map(|release| {
            let lower = release.to_lowercase();
            lower.contains("microsoft") || lower.contains("wsl")
        })
        .unwrap_or(false)
}

/// Helper: Apply platform-specific browser configuration
#[allow(unused_mut)] // mut only needed on Linux/WSL2
fn apply_platform_config(
    mut config: chromiumoxide::browser::BrowserConfigBuilder,
) -> chromiumoxide::browser::BrowserConfigBuilder {
    #[cfg(target_os = "linux")]
    if is_wsl2() {
        tracing::info!("Detected WSL2 environment, using single-process mode");
        config = config
            .arg("--single-process")
            .arg("--no-zygote")
            .arg("--mute-audio")
            .arg("--disable-software-rasterizer");
    }
    config
}

/// Helper: Convert browser launch error to helpful error message
fn handle_browser_launch_error(msg: String) -> BrowserError {
    if msg.contains("Could not auto detect") || msg.contains("chrome executable") {
        #[cfg(target_os = "windows")]
        let install_help = "Chrome not found. Please install Google Chrome from:\n\
            https://www.google.com/chrome/\n\
            Or set CHROME_PATH environment variable to your Chrome installation.";

        #[cfg(not(target_os = "windows"))]
        let install_help = "Chrome/Chromium not found. Please install:\n\
            Ubuntu/Debian: sudo apt-get install chromium-browser\n\
            Fedora: sudo dnf install chromium\n\
            Arch: sudo pacman -S chromium\n\
            Or set CHROME_PATH environment variable.";

        BrowserError::ChromiumError(format!("{}\nOriginal error: {}", install_help, msg))
    } else if msg.contains("ExitStatus(21)") || msg.contains("ExitStatus(ExitStatus(21))") {
        #[cfg(target_os = "windows")]
        let help_text = "Chrome failed to launch (exit status 21).\n\
            Possible causes:\n\
            1. Anti-virus software blocking Chrome\n\
            2. Chrome is already running - try closing all Chrome instances\n\
            3. Missing Visual C++ Runtime - install from Microsoft\n\
            4. Corrupted Chrome installation - try reinstalling Chrome\n\
            \n\
            You can also set CHROME_PATH to specify Chrome location:\n\
            set CHROME_PATH=\"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe\"";

        #[cfg(not(target_os = "windows"))]
        let help_text = "Chrome failed to launch (exit status 21).\n\
            Try setting CHROME_PATH to your Chrome installation:\n\
            export CHROME_PATH=/usr/bin/google-chrome";

        BrowserError::ChromiumError(format!("{}\nOriginal error: {}", help_text, msg))
    } else {
        BrowserError::ChromiumError(msg)
    }
}

impl BrowserEngine {
    /// Create a new browser engine with default configuration
    pub async fn new() -> Result<Self> {
        Self::with_fingerprint(FingerprintConfig::randomized()).await
    }

    /// Create a new browser engine with specific fingerprint
    pub async fn with_fingerprint(fingerprint: FingerprintConfig) -> Result<Self> {
        // Build minimal browser config to avoid snap Chromium incompatibilities
        let mut config = BrowserConfig::builder().no_sandbox().disable_default_args();

        // Set Chrome path from environment variable or auto-detect
        if let Ok(chrome_path) = std::env::var("CHROME_PATH") {
            tracing::info!("Using Chrome from CHROME_PATH: {}", chrome_path);
            config = config.chrome_executable(&chrome_path);
        } else {
            #[cfg(target_os = "windows")]
            if let Some(path) = detect_chrome_path() {
                config = config.chrome_executable(&path);
            }
        }

        // Add essential args
        config = config
            .arg("--headless")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-extensions")
            .arg("--disable-sync");

        // Apply platform-specific configuration
        config = apply_platform_config(config);

        let config = config
            .build()
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| handle_browser_launch_error(e.to_string()))?;

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
            current_page: Arc::new(RwLock::new(None)),
        })
    }

    /// Get or create the current page
    async fn get_page(&self) -> Result<Page> {
        let mut page_lock = self.current_page.write().await;

        if page_lock.is_none() {
            let page = self
                .browser
                .new_page("about:blank")
                .await
                .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;
            *page_lock = Some(page);
        }

        // SAFETY: We just ensured page_lock is Some in the if block above
        Ok(page_lock
            .as_ref()
            .expect("current_page should be Some after initialization")
            .clone())
    }

    /// Fetch a page and return its HTML content
    pub async fn fetch_page_content(&self, url: &str) -> Result<String> {
        // Navigate to the URL
        self.navigate(url).await?;

        // Get the page HTML
        let page = self.get_page().await?;
        let html = page
            .content()
            .await
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

        Ok(html)
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

        let page = self.get_page().await?;

        page.goto(url)
            .await
            .map_err(|e| BrowserError::NavigationError(e.to_string()))?;

        Ok(())
    }

    async fn fill_field(&self, selector: &str, value: &str) -> Result<()> {
        let page = self.get_page().await?;

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
        let page = self.get_page().await?;

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
        let page = self.get_page().await?;

        tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            page.find_element(selector),
        )
        .await
        .map_err(|_| BrowserError::Timeout(format!("Selector {selector} not found")))?
        .map_err(|e| BrowserError::SelectorNotFound(e.to_string()))?;

        Ok(())
    }

    async fn extract_text(&self, selector: &str) -> Result<String> {
        let page = self.get_page().await?;

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
        let page = self.get_page().await?;

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
