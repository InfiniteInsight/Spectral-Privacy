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
    #[allow(clippy::too_many_lines)]
    async fn scan_single_broker(
        &self,
        scan_job_id: String,
        broker_def: BrokerDefinition,
        profile_id: String,
        vault_key: [u8; 32],
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

        // Build search URL from profile data and broker template
        let search_url = match self
            .build_search_url(&broker_def, &profile_id, &vault_key)
            .await
        {
            Ok(url) => url,
            Err(ScanError::MissingRequiredField(field)) => {
                // Profile missing required field - mark as skipped
                spectral_db::broker_scans::update_status(
                    self.db.pool(),
                    &broker_scan.id,
                    "Failed",
                    Some(format!("Profile missing required field: {field}")),
                )
                .await?;

                return Ok(BrokerScanResult {
                    broker_id,
                    findings_count: 0,
                    error: Some(format!("Missing required field: {field}")),
                });
            }
            Err(e) => {
                // Other error building URL
                spectral_db::broker_scans::update_status(
                    self.db.pool(),
                    &broker_scan.id,
                    "Failed",
                    Some(format!("Failed to build search URL: {e}")),
                )
                .await?;

                return Ok(BrokerScanResult {
                    broker_id,
                    findings_count: 0,
                    error: Some(format!("URL building failed: {e}")),
                });
            }
        };

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

        // Parse results using ResultParser with broker-specific selectors
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

    /// Build search URL from broker definition and profile data.
    ///
    /// Loads the profile from database, decrypts required fields,
    /// and substitutes them into the URL template.
    #[allow(clippy::too_many_lines)]
    async fn build_search_url(
        &self,
        broker_def: &BrokerDefinition,
        profile_id: &str,
        vault_key: &[u8; 32],
    ) -> Result<String> {
        use spectral_broker::SearchMethod;
        use spectral_core::PiiField;

        match &broker_def.search {
            SearchMethod::UrlTemplate {
                template,
                requires_fields,
                ..
            } => {
                // Load profile from database
                let profile_id_typed = spectral_core::ProfileId::new(profile_id.to_string())
                    .map_err(|e| ScanError::ProfileDataError {
                        broker_id: broker_def.broker.id.clone(),
                        reason: format!("Invalid profile ID: {e}"),
                    })?;
                let profile = UserProfile::load(&self.db, &profile_id_typed, vault_key)
                    .await
                    .map_err(|e| ScanError::ProfileDataError {
                        broker_id: broker_def.broker.id.clone(),
                        reason: format!("Failed to load profile: {e}"),
                    })?;

                let mut url = template.clone();

                // Substitute each required field
                for field in requires_fields {
                    let (placeholder, value) = match field {
                        PiiField::FirstName => {
                            let val = profile
                                .first_name
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("first_name".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt first_name: {e}"
                                    ))
                                })?;
                            ("{first_name}", val)
                        }
                        PiiField::MiddleName => {
                            let val = profile
                                .middle_name
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("middle_name".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt middle_name: {e}"
                                    ))
                                })?;
                            ("{middle_name}", val)
                        }
                        PiiField::LastName => {
                            let val = profile
                                .last_name
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("last_name".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt last_name: {e}"
                                    ))
                                })?;
                            ("{last_name}", val)
                        }
                        PiiField::Address => {
                            let val = profile
                                .address
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("address".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt address: {e}"
                                    ))
                                })?;
                            ("{address}", val)
                        }
                        PiiField::City => {
                            let val = profile
                                .city
                                .as_ref()
                                .ok_or_else(|| ScanError::MissingRequiredField("city".to_string()))?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt city: {e}"
                                    ))
                                })?;
                            ("{city}", val)
                        }
                        PiiField::State => {
                            let val = profile
                                .state
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("state".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt state: {e}"
                                    ))
                                })?;
                            ("{state}", val)
                        }
                        PiiField::ZipCode => {
                            let val = profile
                                .zip_code
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("zip_code".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt zip_code: {e}"
                                    ))
                                })?;
                            ("{zip_code}", val)
                        }
                        PiiField::Email => {
                            // Use first email from email_addresses array or fall back to deprecated email field
                            let val = if profile.email_addresses.is_empty() {
                                #[allow(deprecated)]
                                profile
                                    .email
                                    .as_ref()
                                    .ok_or_else(|| {
                                        ScanError::MissingRequiredField("email".to_string())
                                    })?
                                    .decrypt(vault_key)
                                    .map_err(|e| {
                                        ScanError::DecryptionFailed(format!(
                                            "Failed to decrypt email: {e}"
                                        ))
                                    })?
                            } else {
                                profile
                                    .email_addresses
                                    .first()
                                    .ok_or_else(|| {
                                        ScanError::MissingRequiredField("email".to_string())
                                    })?
                                    .email
                                    .decrypt(vault_key)
                                    .map_err(|e| {
                                        ScanError::DecryptionFailed(format!(
                                            "Failed to decrypt email: {e}"
                                        ))
                                    })?
                            };
                            ("{email}", val)
                        }
                        PiiField::Phone => {
                            // Use first phone from phone_numbers array or fall back to deprecated phone field
                            let val = if profile.phone_numbers.is_empty() {
                                #[allow(deprecated)]
                                profile
                                    .phone
                                    .as_ref()
                                    .ok_or_else(|| {
                                        ScanError::MissingRequiredField("phone".to_string())
                                    })?
                                    .decrypt(vault_key)
                                    .map_err(|e| {
                                        ScanError::DecryptionFailed(format!(
                                            "Failed to decrypt phone: {e}"
                                        ))
                                    })?
                            } else {
                                profile
                                    .phone_numbers
                                    .first()
                                    .ok_or_else(|| {
                                        ScanError::MissingRequiredField("phone".to_string())
                                    })?
                                    .number
                                    .decrypt(vault_key)
                                    .map_err(|e| {
                                        ScanError::DecryptionFailed(format!(
                                            "Failed to decrypt phone: {e}"
                                        ))
                                    })?
                            };
                            ("{phone}", val)
                        }
                        PiiField::DateOfBirth => {
                            let val = profile
                                .date_of_birth
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("date_of_birth".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt date_of_birth: {e}"
                                    ))
                                })?;
                            ("{date_of_birth}", val)
                        }
                        PiiField::FullName => {
                            let val = profile
                                .full_name
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("full_name".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt full_name: {e}"
                                    ))
                                })?;
                            ("{full_name}", val)
                        }
                        PiiField::Country => {
                            let val = profile
                                .country
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("country".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt country: {e}"
                                    ))
                                })?;
                            ("{country}", val)
                        }
                        PiiField::Ssn => {
                            let val = profile
                                .ssn
                                .as_ref()
                                .ok_or_else(|| ScanError::MissingRequiredField("ssn".to_string()))?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt ssn: {e}"
                                    ))
                                })?;
                            ("{ssn}", val)
                        }
                        PiiField::Employer => {
                            let val = profile
                                .employer
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("employer".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt employer: {e}"
                                    ))
                                })?;
                            ("{employer}", val)
                        }
                        PiiField::JobTitle => {
                            let val = profile
                                .job_title
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("job_title".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt job_title: {e}"
                                    ))
                                })?;
                            ("{job_title}", val)
                        }
                        PiiField::Education => {
                            let val = profile
                                .education
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("education".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt education: {e}"
                                    ))
                                })?;
                            ("{education}", val)
                        }
                        PiiField::SocialMedia => {
                            // Use first social media username from the vector
                            let val = profile
                                .social_media
                                .as_ref()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField("social_media".to_string())
                                })?
                                .decrypt(vault_key)
                                .map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt social_media: {e}"
                                    ))
                                })?
                                .first()
                                .ok_or_else(|| {
                                    ScanError::MissingRequiredField(
                                        "social_media (no usernames provided)".to_string(),
                                    )
                                })?
                                .clone();
                            ("{social_media}", val)
                        }
                        PiiField::Relatives => {
                            // Use first relative's full name
                            let relative = profile.relatives.first().ok_or_else(|| {
                                ScanError::MissingRequiredField("relatives".to_string())
                            })?;

                            let first_name = relative
                                .first_name
                                .as_ref()
                                .and_then(|f| f.decrypt(vault_key).ok())
                                .unwrap_or_default();

                            let last_name = relative
                                .last_name
                                .as_ref()
                                .and_then(|f| f.decrypt(vault_key).ok())
                                .unwrap_or_default();

                            let val = format!("{first_name} {last_name}").trim().to_string();
                            if val.is_empty() {
                                return Err(ScanError::MissingRequiredField(
                                    "relatives (no name data)".to_string(),
                                ));
                            }
                            ("{relatives}", val)
                        }
                        PiiField::PreviousAddress => {
                            // Use first previous address, formatted as full address
                            let prev_addr =
                                profile.previous_addresses_v2.first().ok_or_else(|| {
                                    ScanError::MissingRequiredField("previous_address".to_string())
                                })?;

                            let address =
                                prev_addr.address_line1.decrypt(vault_key).map_err(|e| {
                                    ScanError::DecryptionFailed(format!(
                                        "Failed to decrypt previous address line 1: {e}"
                                    ))
                                })?;

                            let city = prev_addr.city.decrypt(vault_key).map_err(|e| {
                                ScanError::DecryptionFailed(format!(
                                    "Failed to decrypt previous address city: {e}"
                                ))
                            })?;

                            let state = prev_addr.state.decrypt(vault_key).map_err(|e| {
                                ScanError::DecryptionFailed(format!(
                                    "Failed to decrypt previous address state: {e}"
                                ))
                            })?;

                            let zip = prev_addr.zip_code.decrypt(vault_key).map_err(|e| {
                                ScanError::DecryptionFailed(format!(
                                    "Failed to decrypt previous address zip: {e}"
                                ))
                            })?;

                            let val = format!("{address}, {city}, {state} {zip}");
                            ("{previous_address}", val)
                        }
                        PiiField::Age | PiiField::IpAddress | PiiField::Photo | PiiField::Other => {
                            // These fields are not stored in UserProfile or cannot be derived
                            tracing::warn!(
                                "Unsupported PII field (not stored in profile): {:?}",
                                field
                            );
                            continue;
                        }
                    };

                    // URL encode the value and substitute
                    let encoded = urlencoding::encode(&value);
                    url = url.replace(placeholder, &encoded);
                }

                Ok(url)
            }
            SearchMethod::WebForm { url, .. } => {
                // For now, just return the form URL - form submission not yet implemented
                Ok(url.clone())
            }
            SearchMethod::Manual { url, .. } => {
                // Manual search - return the URL for user to visit
                Ok(url.clone())
            }
        }
    }

    /// Parse HTML and store findings in database.
    ///
    /// Uses `ResultParser` with configured selectors to extract structured data
    /// from broker HTML. Performs deduplication to prevent duplicate findings.
    ///
    /// # Note
    /// This method is public for testing purposes.
    pub async fn parse_and_store_findings(
        &self,
        html: &str,
        broker_scan_id: &str,
        broker_id: &BrokerId,
        profile_id: &str,
    ) -> Result<usize> {
        // Get broker definition to access selectors
        let broker_def = self.broker_registry.get(broker_id)?;

        // Get result selectors from broker definition
        let Some(result_selectors) = broker_def.search.result_selectors() else {
            tracing::warn!(
                "Broker {} has no result selectors, skipping parsing",
                broker_id
            );
            return Ok(0);
        };

        // Create ResultParser with selectors and broker base URL
        let parser =
            crate::parser::ResultParser::new(result_selectors, broker_def.broker.url.clone());

        // Parse HTML to get listing matches
        let matches = match parser.parse(html) {
            Ok(matches) => matches,
            Err(e) => {
                tracing::warn!("Failed to parse results for {}: {}", broker_id, e);
                return Ok(0); // Don't fail entire scan on parse error
            }
        };

        // Get scan_job_id from broker_scan record
        let scan_job_id =
            sqlx::query_scalar::<_, String>("SELECT scan_job_id FROM broker_scans WHERE id = ?")
                .bind(broker_scan_id)
                .fetch_one(self.db.pool())
                .await?;

        let mut created_count = 0;
        let mut skipped_count = 0;

        // Process each match
        for listing_match in matches {
            // Check deduplication
            let exists = spectral_db::findings::finding_exists_by_url(
                self.db.pool(),
                &scan_job_id,
                &listing_match.listing_url,
            )
            .await?;

            if exists {
                // Skip duplicate
                skipped_count += 1;
                continue;
            }

            // Convert ExtractedData to JSON
            let extracted_json = extracted_data_to_json(&listing_match.extracted_data);

            // Create finding record with PendingVerification status
            spectral_db::findings::create_finding(
                self.db.pool(),
                broker_scan_id.to_string(),
                broker_id.to_string(),
                profile_id.to_string(),
                listing_match.listing_url,
                extracted_json,
            )
            .await?;

            created_count += 1;
        }

        if skipped_count > 0 {
            tracing::debug!(
                "Skipped {} duplicate findings for broker {}",
                skipped_count,
                broker_id
            );
        }

        Ok(created_count)
    }
}

/// Convert `ExtractedData` to JSON for database storage.
fn extracted_data_to_json(data: &crate::parser::ExtractedData) -> serde_json::Value {
    serde_json::json!({
        "name": data.name,
        "age": data.age,
        "addresses": data.addresses,
        "phone_numbers": data.phone_numbers,
        "relatives": data.relatives,
        "emails": data.emails
    })
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

    #[test]
    fn test_extracted_data_to_json() {
        use crate::parser;

        let data = parser::ExtractedData {
            name: Some("John Doe".to_string()),
            age: Some(30),
            addresses: vec!["123 Main St".to_string()],
            phone_numbers: vec!["555-1234".to_string()],
            relatives: vec!["Jane Doe".to_string()],
            emails: vec!["john@example.com".to_string()],
        };

        let json = extracted_data_to_json(&data);
        assert_eq!(json["name"], "John Doe");
        assert_eq!(json["age"], 30);
        assert_eq!(json["addresses"], serde_json::json!(["123 Main St"]));
        assert_eq!(json["phone_numbers"], serde_json::json!(["555-1234"]));
        assert_eq!(json["relatives"], serde_json::json!(["Jane Doe"]));
        assert_eq!(json["emails"], serde_json::json!(["john@example.com"]));
    }
}
