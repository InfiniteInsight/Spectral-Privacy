//! Automatic CAPTCHA solving via external APIs.
//!
//! Implements support for 2Captcha and Anti-Captcha services, which are
//! the two most reliable paid CAPTCHA solving APIs for reCAPTCHA, hCaptcha,
//! and Cloudflare Turnstile.

use super::error::{CaptchaError, CaptchaService, Result};
use super::{DEFAULT_POLL_INTERVAL_SECS, DEFAULT_TIMEOUT_SECS};
use spectral_core::BrokerId;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Configuration for a CAPTCHA solving service.
#[derive(Debug, Clone)]
pub struct CaptchaConfig {
    /// The CAPTCHA solving service to use.
    pub service: CaptchaService,
    /// API key for the selected service.
    pub api_key: Option<String>,
    /// Maximum seconds to wait for a solution.
    pub timeout_secs: u64,
    /// Polling interval in seconds when waiting for a solution.
    pub poll_interval_secs: u64,
    /// Whether to use the interactive fallback when API solving fails.
    pub use_interactive_fallback: bool,
}

impl Default for CaptchaConfig {
    fn default() -> Self {
        Self {
            service: CaptchaService::None,
            api_key: None,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            poll_interval_secs: DEFAULT_POLL_INTERVAL_SECS,
            use_interactive_fallback: true,
        }
    }
}

impl CaptchaConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let service = std::env::var("SPECTRAL_CAPTCHA_SERVICE")
            .ok()
            .as_deref()
            .map(CaptchaService::parse)
            .unwrap_or_default();

        let api_key = std::env::var("SPECTRAL_CAPTCHA_API_KEY").ok();

        let timeout_secs = std::env::var("SPECTRAL_CAPTCHA_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let poll_interval_secs = std::env::var("SPECTRAL_CAPTCHA_POLL_INTERVAL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_POLL_INTERVAL_SECS);

        Self {
            service,
            api_key,
            timeout_secs,
            poll_interval_secs,
            use_interactive_fallback: true,
        }
    }
}

/// Result of solving a CAPTCHA challenge.
#[derive(Debug, Clone)]
pub struct CaptchaSolution {
    /// The solved token (reCAPTCHA response, hCaptcha token, etc.).
    pub token: String,
    /// The type of CAPTCHA that was solved.
    pub captcha_type: String,
}

/// Automatic CAPTCHA solver.
///
/// Coordinates solving via external APIs with a browser-based fallback.
#[derive(Clone)]
pub struct CaptchaSolver {
    config: CaptchaConfig,
}

impl CaptchaSolver {
    /// Create a new solver with the given configuration.
    #[must_use]
    pub fn new(config: CaptchaConfig) -> Self {
        Self { config }
    }

    /// Create a solver from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(CaptchaConfig::from_env())
    }

    /// Solve a CAPTCHA challenge on the given page.
    ///
    /// Tries the configured API service first, then falls back to interactive
    /// solving if the service is not configured or fails.
    ///
    /// # Arguments
    /// * `page_html` - The current page HTML containing the CAPTCHA.
    /// * `page_url` - The URL of the page (used for CAPTCHA sitekey resolution).
    /// * `broker_id` - The broker being scanned (for logging).
    ///
    /// # Returns
    /// The solved token on success, or an error if all solving methods fail.
    pub async fn solve(
        &self,
        page_html: &str,
        page_url: &str,
        broker_id: &BrokerId,
    ) -> Result<CaptchaSolution> {
        // Determine the CAPTCHA type from the page content
        let captcha_type = self.detect_captcha_type(page_html);

        info!(
            "[captcha] Detected {} on {} at {}",
            captcha_type, broker_id, page_url
        );

        // Try API-based solving first if configured
        if self.config.service != CaptchaService::None {
            if let Some(api_key) = &self.config.api_key {
                match self
                    .solve_with_api(&captcha_type, page_html, page_url, api_key)
                    .await
                {
                    Ok(solution) => {
                        info!(
                            "[captcha] Solved {} via {} API for {}",
                            captcha_type, self.config.service, broker_id
                        );
                        return Ok(solution);
                    }
                    Err(e) => {
                        warn!(
                            "[captcha] API solving failed for {}: {} (will use interactive fallback)",
                            broker_id, e
                        );
                    }
                }
            } else {
                warn!(
                    "[captcha] {} service configured but no API key — using interactive fallback",
                    self.config.service
                );
            }
        }

        // Fall back to interactive solving
        if self.config.use_interactive_fallback {
            return self
                .solve_interactively(page_url, broker_id)
                .await
                .map(|_| {
                    // Interactive solve doesn't return a token directly
                    CaptchaSolution {
                        token: String::new(),
                        captcha_type: "interactive".to_string(),
                    }
                });
        }

        Err(CaptchaError::NoServiceConfigured)
    }

    /// Detect what type of CAPTCHA is present in the page.
    #[allow(clippy::unused_self)]
    fn detect_captcha_type(&self, html: &str) -> String {
        let lower = html.to_lowercase();

        if lower.contains("recaptcha")
            || lower.contains("g-recaptcha")
            || lower.contains("recaptcha_api")
        {
            if lower.contains("recaptcha/invisible") || lower.contains("'invisible'") {
                "reCAPTCHA v2 (invisible)".to_string()
            } else if lower.contains("recaptcha/api.js") {
                "reCAPTCHA v2 (checkbox)".to_string()
            } else if lower.contains("recaptcha__3") || lower.contains("recaptcha/enterprise") {
                "reCAPTCHA v3".to_string()
            } else {
                "reCAPTCHA".to_string()
            }
        } else if lower.contains("hcaptcha") || lower.contains("hcaptcha_frame") {
            "hCaptcha".to_string()
        } else if lower.contains("cf-turnstile")
            || (lower.contains("turnstile") && !lower.contains("cloudflare"))
        {
            "Cloudflare Turnstile".to_string()
        } else if lower.contains("fun captcha") || lower.contains("arc2d") {
            "FunCaptcha".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Solve using an external API service (2Captcha or `AntiCaptcha`).
    async fn solve_with_api(
        &self,
        captcha_type: &str,
        page_html: &str,
        page_url: &str,
        api_key: &str,
    ) -> Result<CaptchaSolution> {
        let client = reqwest::Client::new();

        // Extract the sitekey from the page HTML
        let sitekey = self.extract_sitekey(page_html, captcha_type);

        match self.config.service {
            CaptchaService::TwoCaptcha => {
                self.solve_2captcha(&client, api_key, page_url, sitekey.as_deref(), captcha_type)
                    .await
            }
            CaptchaService::AntiCaptcha => {
                self.solve_antigate(&client, api_key, page_url, sitekey.as_deref(), captcha_type)
                    .await
            }
            CaptchaService::None => Err(CaptchaError::NoServiceConfigured),
        }
    }

    /// Solve using the 2Captcha API.
    async fn solve_2captcha(
        &self,
        client: &reqwest::Client,
        api_key: &str,
        page_url: &str,
        sitekey: Option<&str>,
        captcha_type: &str,
    ) -> Result<CaptchaSolution> {
        // Determine task type based on CAPTCHA type (for documentation only)
        let _task_type = match captcha_type {
            "reCAPTCHA v3" => "RecaptchaV3Tasks",
            "hCaptcha" => "HCaptchaTasks",
            "Cloudflare Turnstile" => "TurnstileTasks",
            _ => "RecaptchaTasks",
        };

        // Submit the CAPTCHA for solving
        let submit_url = "https://2captcha.com/in.php";
        // Determine the correct method name based on CAPTCHA type
        let method = if captcha_type.starts_with("reCAPTCHA")
            || captcha_type.starts_with("hCaptcha")
            || captcha_type.starts_with("Cloudflare")
        {
            "userrecaptcha"
        } else {
            "base64"
        };

        let params = serde_json::json!({
            "key": api_key,
            "method": method,
            "googlekey": sitekey.unwrap_or(""),
            "pageurl": page_url,
            "json": 1,
        });

        debug!("[captcha] 2Captcha: submitting task");

        let response = match client.post(submit_url).form(&params).send().await {
            Ok(resp) => resp,
            Err(e) => {
                return Err(CaptchaError::ServiceError {
                    service: "2captcha".to_string(),
                    message: format!("submit failed: {e}"),
                });
            }
        };

        let submit_result: serde_json::Value = match response.json().await {
            Ok(v) => v,
            Err(e) => {
                return Err(CaptchaError::ParseError(Box::new(e)));
            }
        };

        let request_id = match submit_result.get("request") {
            Some(serde_json::Value::String(s)) if !s.is_empty() && s != "ERROR" => s.clone(),
            _ => {
                let msg = submit_result
                    .get("request")
                    .map_or_else(|| "unknown error".to_string(), serde_json::Value::to_string);
                return Err(CaptchaError::ServiceError {
                    service: "2captcha".to_string(),
                    message: format!("submit error: {msg}"),
                });
            }
        };

        // Poll for the solution
        let poll_url = "https://2captcha.com/res.php";
        let poll_params = serde_json::json!({
            "key": api_key,
            "action": "get",
            "id": request_id,
            "json": 1,
        });

        let solution = self
            .poll_for_solution(client, poll_url, &poll_params)
            .await?;

        Ok(CaptchaSolution {
            token: solution,
            captcha_type: captcha_type.to_string(),
        })
    }

    /// Solve using the Anti-Captcha (`AntiGate`) API.
    async fn solve_antigate(
        &self,
        client: &reqwest::Client,
        api_key: &str,
        page_url: &str,
        sitekey: Option<&str>,
        captcha_type: &str,
    ) -> Result<CaptchaSolution> {
        // Task type is used in the task creation below
        let _task_type = match captcha_type {
            "hCaptcha" => "HCaptchaTaskProxyLess",
            "Cloudflare Turnstile" => "TurnstileTaskProxyLess",
            _ => "RecaptchaV2TaskProxyLess",
        };

        let task: serde_json::Value = match (captcha_type.starts_with("reCAPTCHA"), sitekey) {
            (true, Some(sk)) => serde_json::json!({
                "websiteURL": page_url,
                "websiteKey": sk,
                "type": if captcha_type.contains("v3") { "ReCaptchaV3TaskProxyLess" } else { "RecaptchaV2TaskProxyLess" },
            }),
            _ => serde_json::json!({
                "websiteURL": page_url,
                "type": "RecaptchaV2TaskProxyLess",
            }),
        };

        let submit_url = "https://api.anti.captcha.com/createTask";
        let params = serde_json::json!({
            "clientKey": api_key,
            "task": task,
        });

        debug!("[captcha] AntiCaptcha: submitting task");

        let response = match client.post(submit_url).json(&params).send().await {
            Ok(resp) => resp,
            Err(e) => {
                return Err(CaptchaError::ServiceError {
                    service: "antigate".to_string(),
                    message: format!("submit failed: {e}"),
                });
            }
        };

        let submit_result: serde_json::Value = match response.json().await {
            Ok(v) => v,
            Err(e) => {
                return Err(CaptchaError::ParseError(Box::new(e)));
            }
        };

        if submit_result
            .get("errorId")
            .and_then(serde_json::Value::as_i64)
            == Some(0)
        {
            // Task created successfully
        } else {
            let error_msg = submit_result
                .get("errorDescription")
                .map_or_else(|| "unknown error".to_string(), serde_json::Value::to_string);
            return Err(CaptchaError::ServiceError {
                service: "antigate".to_string(),
                message: error_msg,
            });
        }

        let task_id = match submit_result.get("taskId") {
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => {
                return Err(CaptchaError::ServiceError {
                    service: "antigate".to_string(),
                    message: "no taskId in response".to_string(),
                });
            }
        };

        // Poll for the solution
        let poll_url = "https://api.anti.captcha.com/getTaskResult";
        let poll_params = serde_json::json!({
            "clientKey": api_key,
            "taskId": task_id,
        });

        let solution = self
            .poll_for_solution(client, poll_url, &poll_params)
            .await?;

        Ok(CaptchaSolution {
            token: solution,
            captcha_type: captcha_type.to_string(),
        })
    }

    /// Poll the CAPTCHA solving API until a solution is ready.
    async fn poll_for_solution(
        &self,
        client: &reqwest::Client,
        poll_url: &str,
        poll_params: &serde_json::Value,
    ) -> Result<String> {
        let poll_interval = Duration::from_secs(self.config.poll_interval_secs);
        let timeout = Duration::from_secs(self.config.timeout_secs);
        let deadline = std::time::Instant::now() + timeout;

        loop {
            if std::time::Instant::now() >= deadline {
                return Err(CaptchaError::Timeout {
                    timeout_secs: self.config.timeout_secs,
                });
            }

            tokio::time::sleep(poll_interval).await;

            let response = match client.get(poll_url).json(poll_params).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    debug!("[captcha] poll HTTP error: {e}, retrying");
                    continue;
                }
            };

            let result: serde_json::Value = match response.json().await {
                Ok(v) => v,
                Err(e) => {
                    debug!("[captcha] poll parse error: {e}, retrying");
                    continue;
                }
            };

            // Check for pending status
            if result.get("status").and_then(|v| v.as_str()) == Some("ready") {
                if let Some(solution) = result.get("solution") {
                    // Try gRecaptchaResponse first (reCAPTCHA)
                    if let Some(token) = solution.get("gRecaptchaResponse").and_then(|v| v.as_str())
                    {
                        return Ok(token.to_string());
                    }
                    // Then token field (hCaptcha, Turnstile)
                    if let Some(token) = solution.get("token").and_then(|v| v.as_str()) {
                        return Ok(token.to_string());
                    }
                    // Also try recaptchaResponse
                    if let Some(token) = solution.get("recaptchaResponse").and_then(|v| v.as_str())
                    {
                        return Ok(token.to_string());
                    }
                }

                // For Turnstile and hCaptcha, the solution might be a string directly
                if let Some(token) = result.get("solution").and_then(|s| s.as_str()) {
                    return Ok(token.to_string());
                }

                warn!("[captcha] poll got ready but no token in response: {result}");
            }

            // Check for errors from the service
            if let Some(error_code) = result.get("error_code") {
                return Err(CaptchaError::ServiceError {
                    service: "captcha-api".to_string(),
                    message: format!("API error code: {error_code}"),
                });
            }

            debug!("[captcha] poll: still pending");
        }
    }

    /// Extract the CAPTCHA sitekey from the page HTML.
    fn extract_sitekey(&self, html: &str, captcha_type: &str) -> Option<String> {
        let lower = html.to_lowercase();

        if captcha_type.starts_with("reCAPTCHA") {
            // Look for data-sitekey attribute
            if let Some(start) = lower.find("data-sitekey") {
                let rest = &html[start..];
                if let Some(eq_pos) = rest.find('=') {
                    let value_start = eq_pos + 1;
                    let quote_char = rest.as_bytes().get(value_start)?;
                    let quote = char::from(*quote_char);
                    if let Some(open) = rest[value_start..].find(quote) {
                        let close = rest[value_start + open + 1..].find(quote)?;
                        return Some(
                            rest[value_start + open + 1..value_start + open + 1 + close]
                                .to_string(),
                        );
                    }
                }
            }
        } else if captcha_type == "hCaptcha" {
            if let Some(start) = lower.find("data-sitekey") {
                let rest = &html[start..];
                if let Some(eq_pos) = rest.find('=') {
                    let value_start = eq_pos + 1;
                    let quote_char = rest.as_bytes().get(value_start)?;
                    let quote = char::from(*quote_char);
                    if let Some(open) = rest[value_start..].find(quote) {
                        let close = rest[value_start + open + 1..].find(quote)?;
                        return Some(
                            rest[value_start + open + 1..value_start + open + 1 + close]
                                .to_string(),
                        );
                    }
                }
            }
        }

        None
    }

    /// Solve interactively by opening a visible browser window.
    async fn solve_interactively(
        &self,
        page_url: &str,
        broker_id: &spectral_core::BrokerId,
    ) -> std::result::Result<CaptchaSolution, CaptchaError> {
        info!(
            "[captcha] Opening interactive solver for {} at {}",
            broker_id, page_url
        );

        let timeout = Duration::from_secs(self.config.timeout_secs);
        let result = BrowserEngine::solve_captcha_interactively(page_url, timeout).await;

        match result {
            Ok(_html) => {
                // After interactive solving, try to extract the CAPTCHA token from the page
                // This is a best-effort approach
                info!(
                    "[captcha] Interactive solve appeared successful for {}",
                    broker_id
                );
                Err(CaptchaError::ServiceError {
                    service: "interactive".to_string(),
                    message:
                        "interactive solving completed but no token extracted; scan will continue"
                            .to_string(),
                })
            }
            Err(e) => {
                warn!(
                    "[captcha] Interactive solve failed for {}: {}",
                    broker_id, e
                );
                Err(CaptchaError::ServiceError {
                    service: "interactive".to_string(),
                    message: e.to_string(),
                })
            }
        }
    }
}

use spectral_browser::BrowserEngine;
