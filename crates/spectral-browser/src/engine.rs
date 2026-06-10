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
            // WSL2 has no GPU — force software rasterization explicitly.
            // On Windows and macOS the real GPU is used (no --disable-gpu).
            .arg("--disable-gpu")
            .arg("--no-proxy-server")
            // --headless=new uses a different CDP message format that breaks
            // chromiumoxide's WS deserialization under --single-process in WSL2.
            // Override back to --headless=old for this environment.
            .arg("--headless=old");
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

/// JS injected into every new document before any page scripts run.
/// Patches the most common headless-Chrome detection signals so that
/// Cloudflare, `DataDome`, and similar systems see a fingerprint that
/// matches a real desktop Chrome session.
const STEALTH_JS: &str = r#"
// 1. Remove navigator.webdriver (the single most-checked signal)
Object.defineProperty(navigator, 'webdriver', { get: () => undefined, configurable: true });

// 2. Restore navigator.plugins (empty in headless; real Chrome has PDF viewer entries)
(() => {
    const mkMime = (type, suffixes, description) => {
        const mt = Object.create(MimeType.prototype);
        Object.defineProperties(mt, {
            type:          { get: () => type },
            suffixes:      { get: () => suffixes },
            description:   { get: () => description },
            enabledPlugin: { get: () => null },
        });
        return mt;
    };
    const mkPlugin = (name, filename, description, mimes) => {
        const p = Object.create(Plugin.prototype);
        Object.defineProperties(p, {
            name:        { get: () => name },
            filename:    { get: () => filename },
            description: { get: () => description },
            length:      { get: () => mimes.length },
        });
        mimes.forEach((m, i) => { p[i] = m; });
        return p;
    };
    const pdf = mkMime('application/pdf', 'pdf', 'Portable Document Format');
    const tpdf = mkMime('text/pdf', 'pdf', '');
    const plugins = [
        mkPlugin('PDF Viewer',               'internal-pdf-viewer', 'Portable Document Format', [pdf, tpdf]),
        mkPlugin('Chrome PDF Viewer',        'internal-pdf-viewer', '',                          [pdf]),
        mkPlugin('Chromium PDF Viewer',      'internal-pdf-viewer', '',                          [pdf]),
        mkPlugin('Microsoft Edge PDF Viewer','internal-pdf-viewer', '',                          [pdf]),
        mkPlugin('WebKit built-in PDF',      'internal-pdf-viewer', '',                          [pdf]),
    ];
    const mkArr = (proto, items) => {
        const arr = Object.create(proto);
        Object.defineProperty(arr, 'length', { get: () => items.length });
        items.forEach((x, i) => { arr[i] = x; });
        return arr;
    };
    Object.defineProperty(navigator, 'plugins', {
        get: () => {
            const arr = mkArr(PluginArray.prototype, plugins);
            arr.item = (i) => plugins[i] ?? null;
            arr.namedItem = (n) => plugins.find(p => p.name === n) ?? null;
            arr.refresh = () => {};
            return arr;
        },
        configurable: true,
    });
    const allMimes = [pdf, tpdf];
    Object.defineProperty(navigator, 'mimeTypes', {
        get: () => {
            const arr = mkArr(MimeTypeArray.prototype, allMimes);
            arr.item = (i) => allMimes[i] ?? null;
            arr.namedItem = (t) => allMimes.find(m => m.type === t) ?? null;
            return arr;
        },
        configurable: true,
    });
})();

// 3. navigator.languages
Object.defineProperty(navigator, 'languages', {
    get: () => Object.freeze(['en-US', 'en']),
    configurable: true,
});

// 4. window.chrome — missing entirely in headless; presence is checked by many sites
if (!window.chrome) {
    Object.defineProperty(window, 'chrome', {
        value: {
            app: { isInstalled: false },
            runtime: {},
            loadTimes: () => ({ requestTime: Date.now() / 1000 - 1, startLoadTime: Date.now() / 1000 - 0.9, commitLoadTime: Date.now() / 1000 - 0.5, finishDocumentLoadTime: 0, finishLoadTime: 0, firstPaintTime: 0, firstPaintAfterLoadTime: 0, navigationType: 'Other', wasFetchedViaSpdy: true, wasNpnNegotiated: true, npnNegotiatedProtocol: 'h2', wasAlternateProtocolAvailable: false, connectionInfo: 'h2' }),
            csi: () => ({ startE: Date.now() - 100, onloadT: Date.now(), pageT: Date.now(), tran: 15 }),
        },
        writable: false, enumerable: true, configurable: false,
    });
}

// 5. Notification/permission — headless defaults to 'denied'; spoof as 'default'
if (window.Permissions && window.Permissions.prototype && window.Permissions.prototype.query) {
    const _origQuery = window.Permissions.prototype.query;
    window.Permissions.prototype.query = function(params) {
        if (params && params.name === 'notifications') {
            return Promise.resolve(Object.setPrototypeOf({ state: 'default', onchange: null }, PermissionStatus.prototype));
        }
        return _origQuery.apply(this, [params]);
    };
}

// 6. Broken image dimensions in headless (returns 0; real Chrome returns natural size)
['height', 'width'].forEach(attr => {
    const desc = Object.getOwnPropertyDescriptor(HTMLImageElement.prototype, attr);
    if (desc) {
        Object.defineProperty(HTMLImageElement.prototype, attr, {
            get: function() {
                const v = desc.get.call(this);
                return (v === 0 && this.complete && this.src && !this.src.startsWith('data:')) ? 16 : v;
            },
        });
    }
});
"#;

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

        // Profile directory strategy:
        // - SPECTRAL_PERSISTENT_PROFILE=1: use a single persistent profile at
        //   ~/.local/share/spectral/chrome-profiles/default/ so that Cloudflare
        //   clearance cookies and DataDome trust scores accumulate across scans.
        //   Only safe when scans run sequentially (one Chrome instance at a time).
        // - Default: unique temp dir per engine instance (concurrent-safe).
        let user_data_dir = if std::env::var("SPECTRAL_PERSISTENT_PROFILE").as_deref() == Ok("1") {
            let base = dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                .join("spectral")
                .join("chrome-profiles")
                .join("default");
            tracing::info!(
                "[browser] Using persistent Chrome profile at {}",
                base.display()
            );
            base
        } else {
            // nosemgrep: semgrep.use-zeroize-for-secrets — this is a directory name, not a secret
            let session_id: String = (0..16)
                .map(|_| format!("{:02x}", rand::random::<u8>()))
                .collect();
            std::env::temp_dir().join(format!("spectral-chrome-{session_id}"))
        };
        // Chrome will fail with exit status 21 if the profile dir doesn't exist yet
        if let Err(e) = std::fs::create_dir_all(&user_data_dir) {
            tracing::warn!("Could not pre-create Chrome profile dir: {e}");
        }
        config = config.user_data_dir(&user_data_dir);

        // Add essential args.
        // Use --headless=old: Chrome 112+ changed the default headless renderer and
        // --headless=new can fail to launch on Windows without a full GPU stack.
        // Spoof User-Agent to look like regular (non-headless) Chrome.
        // The default headless UA contains "HeadlessChrome" which sites use to 403/block bots.
        let ua = fingerprint.user_agent.clone();

        config = config
            // --headless=new (Chrome 112+): uses the real GPU pipeline, producing
            // canvas/WebGL fingerprints indistinguishable from a non-headless session.
            // --disable-gpu is intentionally omitted here; WSL2 adds it in apply_platform_config.
            .arg("--headless=new")
            .arg("--no-first-run")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-extensions")
            .arg("--disable-sync")
            .arg("--disable-default-apps")
            .arg("--disable-hang-monitor")
            .arg("--disable-popup-blocking")
            .arg("--no-default-browser-check")
            .arg("--metrics-recording-only")
            // Stealth: exclude automation-detection flags
            .arg("--disable-blink-features=AutomationControlled")
            .arg("--exclude-switches=enable-automation")
            // Stealth: realistic window size
            .arg("--window-size=1920,1080")
            .arg(format!("--user-agent={ua}"));

        // Proxy support: set SPECTRAL_PROXY to route through a residential or
        // datacenter proxy (e.g. "http://user:pass@proxy.example.com:8080" or
        // "socks5://127.0.0.1:1080"). Residential proxies bypass IP-reputation
        // checks that Cloudflare and DataDome use as part of their fingerprint.
        if let Ok(proxy) = std::env::var("SPECTRAL_PROXY") {
            tracing::info!("[browser] Using proxy: {}", proxy);
            config = config.arg(format!("--proxy-server={proxy}"));
        }

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

    /// Get or create the current page, with stealth patches pre-installed.
    async fn get_page(&self) -> Result<Page> {
        let mut page_lock = self.current_page.write().await;

        if page_lock.is_none() {
            let page = self
                .browser
                .new_page("about:blank")
                .await
                .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;
            // Inject stealth patches before any navigation so they fire on every
            // document load, including redirects and subsequent navigations.
            if let Err(e) = page.evaluate_on_new_document(STEALTH_JS).await {
                tracing::warn!("[stealth] Failed to inject stealth patches: {e}");
            }
            *page_lock = Some(page);
        }

        // SAFETY: We just ensured page_lock is Some in the if block above
        Ok(page_lock
            .as_ref()
            .expect("current_page should be Some after initialization")
            .clone())
    }

    /// Open a visible (non-headless) Chrome window so the user can solve a CAPTCHA,
    /// then return the page HTML once the CAPTCHA challenge is no longer present.
    ///
    /// Polls every 2 seconds for up to `timeout`. The user sees a real Chrome window
    /// and can interact with it normally.
    pub async fn solve_captcha_interactively(url: &str, timeout: Duration) -> Result<String> {
        // WSL2 has no display — skip interactive solving immediately.
        #[cfg(target_os = "linux")]
        if is_wsl2() {
            return Err(BrowserError::Timeout(
                "CAPTCHA solving skipped: no display in WSL2".to_string(),
            ));
        }

        tracing::info!("[captcha] Opening visible browser for CAPTCHA at {}", url);

        let mut config = BrowserConfig::builder().no_sandbox().disable_default_args();

        if let Ok(chrome_path) = std::env::var("CHROME_PATH") {
            config = config.chrome_executable(&chrome_path);
        } else {
            #[cfg(target_os = "windows")]
            if let Some(path) = detect_chrome_path() {
                config = config.chrome_executable(&path);
            }
        }

        // Separate profile dir so the visible window doesn't conflict with headless scans
        let user_data_dir = std::env::temp_dir().join("spectral-chrome-captcha");
        config = config.user_data_dir(&user_data_dir);

        // Visible window — no --headless
        config = config
            .arg("--no-first-run")
            .arg("--disable-extensions")
            .arg("--disable-sync")
            .arg("--window-size=1024,768");

        let config = config
            .build()
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

        let (mut browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| handle_browser_launch_error(e.to_string()))?;

        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                let _ = event;
            }
        });

        let page = browser
            .new_page(url)
            .await
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

        let deadline = Instant::now() + timeout; // nosemgrep: semgrep.llm-prompt-injection-risk
        let result = loop {
            tokio::time::sleep(Duration::from_secs(2)).await;

            match page.content().await {
                Ok(html) => {
                    let lower = html.to_lowercase();
                    if !lower.contains("recaptcha")
                        && !lower.contains("g-recaptcha")
                        && !lower.contains("captcha")
                        && !lower.contains("cf-challenge")
                    {
                        tracing::info!("[captcha] CAPTCHA solved, resuming scan");
                        break Ok(html);
                    }
                }
                Err(e) => {
                    tracing::warn!("[captcha] Could not read page content: {e}");
                }
            }

            if Instant::now() >= deadline {
                break Err(BrowserError::Timeout(
                    "CAPTCHA was not solved within the allowed time".to_string(),
                ));
            }
        };

        let _ = browser.close().await;
        result
    }

    /// Fetch a page using reqwest (plain HTTP client) — used on WSL2 where
    /// headless Chrome's network stack cannot reach external hosts.
    async fn fetch_with_reqwest(&self, url: &str) -> Result<String> {
        let ua = self.fingerprint.user_agent.clone();
        let client = reqwest::Client::builder()
            .user_agent(ua)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| BrowserError::ChromiumError(format!("reqwest build error: {e}")))?;

        let html = client
            .get(url)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
            )
            .header("Accept-Language", "en-US,en;q=0.9")
            // Accept-Encoding is managed by reqwest (gzip auto-decompression).
            // Do NOT override it — setting br/zstd causes the server to send
            // brotli/zstd which reqwest won't decompress, yielding binary content.
            .header("DNT", "1")
            .header("Upgrade-Insecure-Requests", "1")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "none")
            .header("Sec-Fetch-User", "?1")
            .header("Sec-CH-UA", r#""Chromium";v="120", "Google Chrome";v="120", "Not-A.Brand";v="99""#)
            .header("Sec-CH-UA-Mobile", "?0")
            .header("Sec-CH-UA-Platform", r#""Windows""#)
            .header("Cache-Control", "max-age=0")
            .send()
            .await
            .map_err(|e| BrowserError::NavigationError(format!("reqwest fetch error: {e}")))?
            .text()
            .await
            .map_err(|e| BrowserError::ChromiumError(format!("reqwest body error: {e}")))?;

        Ok(html)
    }

    /// Fetch a page and return its HTML content
    pub async fn fetch_page_content(&self, url: &str) -> Result<String> {
        // Rate-check before acquiring the page lock
        let domain = extract_domain(url)?;
        self.rate_limiter
            .write()
            .await
            .check_and_update(&domain)
            .await?;

        // Note: Chrome headless CAN reach external hosts in WSL2. We use Chrome
        // rather than reqwest so that JavaScript-rendered pages load correctly.
        // The SPECTRAL_USE_REQWEST env var provides an escape hatch for offline/CI use.
        #[cfg(target_os = "linux")]
        if is_wsl2() && std::env::var("SPECTRAL_USE_REQWEST").as_deref() == Ok("1") {
            tracing::info!(
                "[browser] WSL2: SPECTRAL_USE_REQWEST=1, using reqwest for {}",
                url
            );
            return self.fetch_with_reqwest(url).await;
        }

        // Hold the write lock for the full navigate+content cycle so that concurrent
        // callers on the same engine cannot swap the page between the two calls.
        // A 45-second overall timeout prevents a hung page from blocking forever.
        let page_fut = async {
            let page = {
                let mut page_lock = self.current_page.write().await;
                if page_lock.is_none() {
                    let p = self
                        .browser
                        .new_page("about:blank")
                        .await
                        .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;
                    if let Err(e) = p.evaluate_on_new_document(STEALTH_JS).await {
                        tracing::warn!("[stealth] Failed to inject stealth patches: {e}");
                    }
                    *page_lock = Some(p);
                }
                page_lock
                    .as_ref()
                    .expect("current_page should be Some")
                    .clone()
            };

            page.goto(url)
                .await
                .map_err(|e| BrowserError::NavigationError(e.to_string()))?;

            // Adaptive bot-protection wait: poll every 500 ms until the page is
            // clear of known challenge markers (Cloudflare "Just a moment",
            // DataDome interstitial, reCAPTCHA). Falls back to a fixed minimum
            // wait. Max wait is controlled by SPECTRAL_PAGE_WAIT_MS (default 8 s).
            let max_wait_ms: u64 = std::env::var("SPECTRAL_PAGE_WAIT_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8000);
            let poll_interval = Duration::from_millis(500);
            let deadline = Instant::now() + Duration::from_millis(max_wait_ms);
            loop {
                tokio::time::sleep(poll_interval).await;
                if let Ok(html) = page.content().await {
                    let lower = html.to_lowercase();
                    // Challenge markers that indicate the page is not ready
                    let has_challenge = lower.contains("just a moment")
                        || lower.contains("checking your browser")
                        || lower.contains("cf-challenge")
                        || lower.contains("datadome")
                        || lower.contains("g-recaptcha")
                        || lower.contains("recaptcha/api.js");
                    if !has_challenge {
                        tracing::debug!("[browser] Page ready (no challenge detected)");
                        break;
                    }
                    tracing::debug!("[browser] Challenge detected, waiting...");
                }
                if Instant::now() >= deadline {
                    tracing::warn!("[browser] Max wait reached for {url}, proceeding anyway");
                    break;
                }
            }

            let html = page
                .content()
                .await
                .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

            Ok::<String, BrowserError>(html)
        };

        tokio::time::timeout(Duration::from_secs(55), page_fut)
            .await
            .map_err(|_| BrowserError::Timeout(format!("fetch_page_content timed out for {url}")))?
    }
}

#[async_trait::async_trait]
impl BrowserActions for BrowserEngine {
    async fn navigate(&self, url: &str) -> Result<()> {
        let page = self.get_page().await?;

        tokio::time::timeout(Duration::from_secs(30), page.goto(url))
            .await
            .map_err(|_| BrowserError::Timeout(format!("Navigation to {url} timed out after 30s")))?
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

    async fn get_current_page_html(&self) -> Result<String> {
        let page = self.get_page().await?;

        let html = page
            .content()
            .await
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

        Ok(html)
    }

    async fn evaluate(&self, js: &str) -> Result<String> {
        let page = self.get_page().await?;

        let _result = page
            .evaluate(js)
            .await
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;

        // EvaluateResult from chromiumoxide doesn't implement Serialize,
        // so we just return a success marker
        Ok("true".to_string())
    }

    async fn inject_captcha_token(&self, token: &str) -> Result<()> {
        let page = self.get_page().await?;

        // Detect CAPTCHA type and inject token appropriately
        let html = page
            .content()
            .await
            .map_err(|e| BrowserError::ChromiumError(e.to_string()))?;
        let lower_html = html.to_lowercase();

        // reCAPTCHA v2/v3 token injection
        if lower_html.contains("recaptcha") || lower_html.contains("g-recaptcha") {
            // reCAPTCHA stores tokens in grecaptcha.getResponse() and internal cache
            let js = format!(
                r#"
                (async () => {{
                    const token = '{token}';
                    try {{
                        // Set the global response that grecaptcha.getResponse() returns
                        if (typeof grecaptcha !== 'undefined') {{
                            // Store token in internal response field
                            grecaptcha._response = token;
                            grecaptcha.getResponse = () => token;

                            // Also set the g-recaptcha-response for form submissions
                            const iframe = document.querySelector('.g-recaptcha > iframe');
                            if (iframe) {{
                                const doc = iframe.contentDocument || iframe.contentWindow.document;
                                const input = doc.querySelector('#g-recaptcha-response');
                                if (input) {{
                                    input.value = token;
                                }}
                            }}

                            // Try to trigger the callback if one exists
                            if (typeof grecaptcha.ready === 'function') {{
                                // Find callbacks registered via grecaptcha.ready()
                                if (grecaptcha._readyFns && grecaptcha._readyFns.length > 0) {{
                                    for (const fn of grecaptcha._readyFns) {{
                                        try {{ fn(); }} catch (e) {{ /* ignore */ }}
                                    }}
                                }}
                            }}
                        }}

                        // Look for any form that might need the token
                        const forms = document.querySelectorAll('form');
                        for (const form of forms) {{
                            let existing = form.querySelector('#g-recaptcha-response');
                            if (!existing) {{
                                existing = document.createElement('input');
                                existing.type = 'hidden';
                                existing.id = 'g-recaptcha-response';
                                form.prepend(existing);
                            }}
                            existing.value = token;
                        }}

                        console.log('reCAPTCHA token injected successfully');
                    }} catch (e) {{
                        console.error('Failed to inject reCAPTCHA token:', e);
                    }}
                }})();
                "#,
                token = token,
            );
            if let Err(e) = page.evaluate(&*js).await {
                tracing::debug!("[captcha] reCAPTCHA token injection failed: {e}");
            }
        }

        // hCaptcha token injection
        if lower_html.contains("hcaptcha") || lower_html.contains("hcaptcha_frame") {
            let js = format!(
                r#"
                (async () => {{
                    const token = '{token}';
                    try {{
                        // hCaptcha stores tokens in hcaptcha.getResponse()
                        if (typeof hcaptcha !== 'undefined') {{
                            hcaptcha._response = token;
                            hcaptcha.getResponse = () => token;
                            hcaptcha.reset = () => token;

                            // Inject into hidden input that hcaptcha uses
                            const input = document.querySelector('#hcaptcha-response');
                            if (input) {{
                                input.value = token;
                            }}

                            // Also try to find it in form hidden inputs
                            const forms = document.querySelectorAll('form');
                            for (const form of forms) {{
                                let existing = form.querySelector('#hcaptcha-response');
                                if (!existing) {{
                                    existing = document.createElement('input');
                                    existing.type = 'hidden';
                                    existing.id = 'hcaptcha-response';
                                    form.prepend(existing);
                                }}
                                existing.value = token;
                            }}

                            console.log('hCaptcha token injected successfully');
                        }}
                    }} catch (e) {{
                        console.error('Failed to inject hCaptcha token:', e);
                    }}
                }})();
                "#,
                token = token,
            );
            if let Err(e) = page.evaluate(&*js).await {
                tracing::debug!("[captcha] hCaptcha token injection failed: {e}");
            }
        }

        // Cloudflare Turnstile token injection
        if lower_html.contains("cf-turnstile")
            || (lower_html.contains("turnstile") && !lower_html.contains("cloudflare"))
        {
            let js = format!(
                r#"
                (async () => {{
                    const token = '{token}';
                    try {{
                        // Cloudflare Turnstile stores tokens in cfturnstile.getResponse()
                        if (typeof cfturnstile !== 'undefined') {{
                            cfturnstile._response = token;
                            cfturnstile.getResponse = () => token;

                            // Inject into hidden input
                            const input = document.querySelector('[name="cf-turnstile-response"]');
                            if (input) {{
                                input.value = token;
                            }}

                            // Also look for any form with hidden response input
                            const forms = document.querySelectorAll('form');
                            for (const form of forms) {{
                                let existing = form.querySelector('[name="cf-turnstile-response"]');
                                if (!existing) {{
                                    existing = document.createElement('input');
                                    existing.type = 'hidden';
                                    existing.name = 'cf-turnstile-response';
                                    form.prepend(existing);
                                }}
                                existing.value = token;
                            }}

                            console.log('Cloudflare Turnstile token injected successfully');
                        }}
                    }} catch (e) {{
                        console.error('Failed to inject Turnstile token:', e);
                    }}
                }})();
                "#,
                token = token,
            );
            if let Err(e) = page.evaluate(&*js).await {
                tracing::debug!("[captcha] Turnstile token injection failed: {e}");
            }
        }

        // If no CAPTCHA type was detected, still try to inject into common patterns
        tracing::debug!("[captcha] Token injection complete for page");

        Ok(())
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
