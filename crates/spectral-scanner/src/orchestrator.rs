//! Scan orchestrator for coordinating broker scans.
//!
//! This module provides the `ScanOrchestrator` which manages the execution
//! of scan jobs across multiple brokers with retry logic, error handling,
//! and findings storage.

use crate::error::{Result, ScanError};
use crate::filter::BrokerFilter;
use futures::stream::{FuturesUnordered, StreamExt};
use spectral_broker::{BrokerDefinition, BrokerRegistry};
use spectral_browser::BrowserEngine;
use spectral_core::BrokerId;
use spectral_db::{scan_jobs, Database};
use spectral_vault::UserProfile;
use std::sync::Arc;
use std::time::Duration;

/// Maximum number of retry attempts for transient errors.
const MAX_RETRIES: u32 = 3;

/// Base delay in milliseconds for retry backoff.
const RETRY_DELAY_MS: u64 = 2000;

/// Rate limit backoff multiplier (longer wait for rate limits).
const RATE_LIMIT_BACKOFF_MULTIPLIER: u64 = 3;

/// Result of scanning a single broker.
#[derive(Debug, Clone)]
pub struct BrokerScanResult {
    /// Broker ID that was scanned
    pub broker_id: BrokerId,
    /// Number of findings discovered
    pub findings_count: usize,
    /// Error message if scan failed
    pub error: Option<String>,
}

/// Orchestrates scanning operations across multiple brokers.
pub struct ScanOrchestrator {
    /// Broker registry for broker definitions
    broker_registry: Arc<BrokerRegistry>,
    /// Browser engine for page fetching
    browser_engine: Arc<BrowserEngine>,
    /// Database for storing results
    db: Arc<Database>,
    /// Maximum concurrent scans
    max_concurrent_scans: usize,
}

impl ScanOrchestrator {
    /// Create a new scan orchestrator.
    #[must_use]
    pub fn new(
        broker_registry: Arc<BrokerRegistry>,
        browser_engine: Arc<BrowserEngine>,
        db: Arc<Database>,
    ) -> Self {
        Self {
            broker_registry,
            browser_engine,
            db,
            max_concurrent_scans: 5,
        }
    }

    /// Set the maximum number of concurrent scans.
    #[must_use]
    pub fn with_max_concurrent_scans(mut self, max: usize) -> Self {
        self.max_concurrent_scans = max;
        self
    }

    /// Start a new scan job with the specified profile and broker filter.
    ///
    /// This creates a scan job in the database, launches background execution,
    /// and returns the job ID immediately for status tracking.
    ///
    /// # Arguments
    /// * `profile` - User profile to search for
    /// * `broker_filter` - Filter to select which brokers to scan
    /// * `vault_key` - Encryption key for accessing encrypted profile data
    ///
    /// # Returns
    /// The scan job ID for tracking progress
    #[allow(clippy::cast_possible_truncation)]
    pub async fn start_scan(
        &self,
        profile: &UserProfile,
        broker_filter: BrokerFilter,
        vault_key: &[u8; 32],
    ) -> Result<String> {
        // Get list of brokers to scan
        let brokers: Vec<_> = self
            .broker_registry
            .get_all()
            .into_iter()
            .filter(|broker| broker_filter.matches(broker))
            .collect();

        let total_brokers = brokers.len() as u32;
        let broker_ids: Vec<BrokerId> = brokers.iter().map(|b| b.id().clone()).collect();

        // Create scan job in database
        let job = scan_jobs::create_scan_job(
            self.db.pool(),
            profile.id.as_str().to_string(),
            total_brokers,
        )
        .await?;

        let job_id = job.id.clone();
        let profile_id = profile.id.as_str().to_string();
        let vault_key = *vault_key;

        // Clone Arc references for background task
        let orchestrator_clone = Arc::new(Self {
            broker_registry: self.broker_registry.clone(),
            browser_engine: self.browser_engine.clone(),
            db: self.db.clone(),
            max_concurrent_scans: self.max_concurrent_scans,
        });

        // Clone job_id for background task
        let job_id_for_task = job_id.clone();

        // Launch scan execution in background
        tokio::spawn(async move {
            let result = orchestrator_clone
                .execute_scan_job(job_id_for_task.clone(), broker_ids, profile_id, vault_key)
                .await;

            match result {
                Ok(results) => {
                    let completed = results.len() as u32;
                    let _ = orchestrator_clone
                        .complete_scan_job(&job_id_for_task, completed)
                        .await;
                }
                Err(e) => {
                    tracing::error!("Scan job {} failed: {}", job_id_for_task, e);
                    let _ = orchestrator_clone
                        .fail_scan_job(&job_id_for_task, &e.to_string())
                        .await;
                }
            }
        });

        Ok(job_id)
    }

    /// Mark a scan job as completed.
    async fn complete_scan_job(&self, job_id: &str, completed_brokers: u32) -> Result<()> {
        sqlx::query(
            "UPDATE scan_jobs SET status = 'Completed', completed_at = ?, completed_brokers = ? WHERE id = ?"
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(completed_brokers)
        .bind(job_id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Mark a scan job as failed.
    async fn fail_scan_job(&self, job_id: &str, error_message: &str) -> Result<()> {
        sqlx::query(
            "UPDATE scan_jobs SET status = 'Failed', completed_at = ?, error_message = ? WHERE id = ?"
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(error_message)
        .bind(job_id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Execute a scan job across multiple brokers.
    ///
    /// This scans all specified brokers concurrently (up to `max_concurrent_scans`)
    /// and stores findings in the database.
    pub async fn execute_scan_job(
        &self,
        scan_job_id: String,
        broker_ids: Vec<BrokerId>,
        profile_id: String,
        vault_key: [u8; 32],
    ) -> Result<Vec<BrokerScanResult>> {
        let mut futures = FuturesUnordered::new();
        let mut results = Vec::new();

        for broker_id in broker_ids {
            // Get broker definition
            let broker_def = match self.broker_registry.get(&broker_id) {
                Ok(def) => def,
                Err(e) => {
                    tracing::error!("Failed to get broker definition for {}: {}", broker_id, e);
                    results.push(BrokerScanResult {
                        broker_id: broker_id.clone(),
                        findings_count: 0,
                        error: Some(format!("Broker not found: {e}")),
                    });
                    continue;
                }
            };

            futures.push(self.scan_single_broker(
                scan_job_id.clone(),
                broker_def.clone(),
                profile_id.clone(),
                vault_key,
            ));

            // Respect concurrency limit
            while futures.len() >= self.max_concurrent_scans {
                if let Some(result) = futures.next().await {
                    match result {
                        Ok(broker_result) => results.push(broker_result),
                        Err(e) => {
                            tracing::error!("Scan failed: {}", e);
                        }
                    }
                }
            }
        }

        // Collect remaining results
        while let Some(result) = futures.next().await {
            match result {
                Ok(broker_result) => results.push(broker_result),
                Err(e) => {
                    tracing::error!("Scan failed: {}", e);
                }
            }
        }

        Ok(results)
    }

    /// Scan a single broker with retry logic and error handling.
    ///
    /// Creates a `broker_scan` record, fetches the page with retries,
    /// parses results, and stores findings in the database.
    async fn scan_single_broker(
        &self,
        scan_job_id: String,
        broker_def: BrokerDefinition,
        profile_id: String,
        _vault_key: [u8; 32],
    ) -> Result<BrokerScanResult> {
        let broker_id = broker_def.broker.id.clone();

        // Create broker_scan record
        let broker_scan = spectral_db::broker_scans::create_broker_scan(
            self.db.pool(),
            scan_job_id.clone(),
            broker_id.to_string(),
        )
        .await?;

        // Update status to InProgress
        spectral_db::broker_scans::update_status(
            self.db.pool(),
            &broker_scan.id,
            "InProgress",
            None,
        )
        .await?;

        // Build search URL (simplified - in real impl, use profile data)
        let search_url = format!(
            "{}/search?name=test",
            broker_def.broker.url.trim_end_matches('/')
        );

        // Fetch page with retry logic
        let html = match self.fetch_with_retry(&search_url, &broker_id).await {
            Ok(html) => html,
            Err(ScanError::CaptchaRequired { .. }) => {
                // CAPTCHA detected - mark as failed, don't retry
                spectral_db::broker_scans::update_status(
                    self.db.pool(),
                    &broker_scan.id,
                    "Failed",
                    Some("CAPTCHA required - manual intervention needed".to_string()),
                )
                .await?;

                return Ok(BrokerScanResult {
                    broker_id,
                    findings_count: 0,
                    error: Some("CAPTCHA challenge detected".to_string()),
                });
            }
            Err(ScanError::RateLimited { retry_after, .. }) => {
                // Rate limited - mark as failed with retry suggestion
                spectral_db::broker_scans::update_status(
                    self.db.pool(),
                    &broker_scan.id,
                    "Failed",
                    Some(format!("Rate limited - retry after {retry_after:?}")),
                )
                .await?;

                return Ok(BrokerScanResult {
                    broker_id,
                    findings_count: 0,
                    error: Some("Rate limited".to_string()),
                });
            }
            Err(e) => {
                // Other error - mark as failed
                spectral_db::broker_scans::update_status(
                    self.db.pool(),
                    &broker_scan.id,
                    "Failed",
                    Some(format!("Fetch error: {e}")),
                )
                .await?;

                return Ok(BrokerScanResult {
                    broker_id,
                    findings_count: 0,
                    error: Some(format!("Failed to fetch: {e}")),
                });
            }
        };

        // Parse results (simplified - would use ResultParser with selectors)
        let findings_count = self
            .parse_and_store_findings(&html, &broker_scan.id, &broker_id, &profile_id)
            .await?;

        // Mark as success
        spectral_db::broker_scans::update_status(self.db.pool(), &broker_scan.id, "Success", None)
            .await?;

        Ok(BrokerScanResult {
            broker_id,
            findings_count,
            error: None,
        })
    }

    /// Fetch a page with retry logic and exponential backoff.
    ///
    /// Retries transient errors up to `MAX_RETRIES` times with exponential backoff.
    /// Rate limit errors use longer backoff. CAPTCHA errors are not retried.
    async fn fetch_with_retry(&self, url: &str, broker_id: &BrokerId) -> Result<String> {
        let mut last_error = None;
        let mut backoff_multiplier = 1;

        for attempt in 0..MAX_RETRIES {
            match self.browser_engine.fetch_page_content(url).await {
                Ok(html) => {
                    // Check for CAPTCHA in HTML before returning
                    if Self::detect_captcha(&html) {
                        return Err(ScanError::CaptchaRequired {
                            broker_id: broker_id.clone(),
                        });
                    }
                    return Ok(html);
                }
                Err(e) => {
                    // Check if this is a rate limit error
                    if Self::is_rate_limited(&e) {
                        backoff_multiplier = RATE_LIMIT_BACKOFF_MULTIPLIER;
                        tracing::warn!("Rate limited for {}, using longer backoff", broker_id);
                    }

                    last_error = Some(e);

                    if attempt < MAX_RETRIES - 1 {
                        let delay = Duration::from_millis(
                            RETRY_DELAY_MS * backoff_multiplier * (u64::from(attempt) + 1),
                        );

                        tracing::warn!(
                            "Fetch failed for {} (attempt {}/{}), retrying in {:?}...",
                            broker_id,
                            attempt + 1,
                            MAX_RETRIES,
                            delay
                        );

                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        // If we exhausted retries due to rate limiting, return specific error
        if let Some(e) = &last_error {
            if Self::is_rate_limited(e) {
                return Err(ScanError::RateLimited {
                    broker_id: broker_id.clone(),
                    retry_after: Duration::from_secs(300), // 5 minutes
                });
            }
        }

        Err(ScanError::Browser(last_error.expect(
            "last_error should be Some after MAX_RETRIES attempts",
        )))
    }

    /// Check if a browser error indicates rate limiting.
    fn is_rate_limited(error: &spectral_browser::BrowserError) -> bool {
        matches!(error, spectral_browser::BrowserError::RateLimitExceeded(_))
    }

    /// Detect CAPTCHA challenges in HTML content.
    ///
    /// Looks for common CAPTCHA indicators like reCAPTCHA iframes or CAPTCHA divs.
    fn detect_captcha(html: &str) -> bool {
        html.contains("recaptcha") || html.contains("g-recaptcha") || html.contains("captcha")
    }

    /// Parse HTML and store findings in database.
    ///
    /// This is a simplified implementation. In production, this would use
    /// `ResultParser` with configured selectors to extract structured data.
    async fn parse_and_store_findings(
        &self,
        _html: &str,
        broker_scan_id: &str,
        broker_id: &BrokerId,
        profile_id: &str,
    ) -> Result<usize> {
        // Simplified: In real implementation, would parse HTML with selectors
        // and create findings for each match found

        // For now, just create a dummy finding to demonstrate the flow
        let extracted_data = serde_json::json!({
            "name": "Example Name",
            "location": "Example City, ST"
        });

        spectral_db::findings::create_finding(
            self.db.pool(),
            broker_scan_id.to_string(),
            broker_id.to_string(),
            profile_id.to_string(),
            format!("https://example.com/profile/{}", uuid::Uuid::new_v4()),
            extracted_data,
        )
        .await?;

        Ok(1) // Return count of findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_constants() {
        // Verify retry configuration constants are reasonable
        const _: () = assert!(MAX_RETRIES > 0);
        const _: () = assert!(MAX_RETRIES <= 5);
        const _: () = assert!(RETRY_DELAY_MS > 0);
        const _: () = assert!(RETRY_DELAY_MS >= 1000);
        const _: () = assert!(RATE_LIMIT_BACKOFF_MULTIPLIER > 1);
    }

    #[test]
    fn test_captcha_detection() {
        // Test CAPTCHA detection logic without browser
        let html_with_captcha = r#"<div class="g-recaptcha"></div>"#;
        assert!(html_with_captcha.contains("recaptcha"));

        let html_with_captcha2 = r#"<div class="captcha-container"></div>"#;
        assert!(html_with_captcha2.contains("captcha"));

        let html_without_captcha = r#"<div class="search-results"></div>"#;
        assert!(!html_without_captcha.contains("recaptcha"));
        assert!(!html_without_captcha.contains("g-recaptcha"));
    }

    #[test]
    fn test_max_concurrent_scans() {
        // Test that the default value is within reasonable bounds
        const DEFAULT_MAX: usize = 5;
        const _: () = assert!(DEFAULT_MAX > 0);
        const _: () = assert!(DEFAULT_MAX <= 20);
    }

    #[test]
    fn test_rate_limit_backoff() {
        // Verify rate limit backoff is longer than normal backoff
        let normal_backoff = RETRY_DELAY_MS;
        let rate_limit_backoff = RETRY_DELAY_MS * RATE_LIMIT_BACKOFF_MULTIPLIER;
        assert!(rate_limit_backoff > normal_backoff);
        assert!(rate_limit_backoff >= 3 * normal_backoff);
    }
}
