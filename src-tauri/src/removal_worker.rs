//! Worker tasks for removal form submission.
//!
//! Handles async removal submission with retry logic, CAPTCHA detection,
//! and database state management.

use spectral_broker::definition::RemovalMethod;
use spectral_broker::removal::{RemovalOutcome, WebFormSubmitter};
use spectral_broker::BrokerRegistry;
use spectral_browser::{BrowserActions, BrowserEngine};
use spectral_core::BrokerId;
use spectral_db::removal_attempts::{self, RemovalStatus};
use spectral_db::Database;
use spectral_vault::UserProfile;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use tracing::{error, info, warn};

/// Result of a removal submission worker task.
#[derive(Debug)]
pub struct WorkerResult {
    pub removal_attempt_id: String,
    pub outcome: RemovalOutcome,
}

/// Map profile and finding data to form fields.
///
/// Extracts required fields from profile and finding for form submission.
#[allow(deprecated)]
pub fn map_fields_for_submission(
    profile: &UserProfile,
    finding_listing_url: &str,
    key: &[u8; 32],
) -> Result<HashMap<String, String>, String> {
    let mut fields = HashMap::new();

    // listing_url from finding
    fields.insert("listing_url".to_string(), finding_listing_url.to_string());

    // Email from profile (required)
    let email = profile
        .email
        .as_ref()
        .ok_or("Missing required field: email")?
        .decrypt(key)
        .map_err(|e| format!("Failed to decrypt email: {}", e))?;
    fields.insert("email".to_string(), email);

    // First name (required)
    let first_name = profile
        .first_name
        .as_ref()
        .ok_or("Missing required field: first_name")?
        .decrypt(key)
        .map_err(|e| format!("Failed to decrypt first_name: {}", e))?;
    fields.insert("first_name".to_string(), first_name);

    // Last name (required)
    let last_name = profile
        .last_name
        .as_ref()
        .ok_or("Missing required field: last_name")?
        .decrypt(key)
        .map_err(|e| format!("Failed to decrypt last_name: {}", e))?;
    fields.insert("last_name".to_string(), last_name);

    Ok(fields)
}

/// Retry a task with exponential backoff.
///
/// Attempts the task up to `max_attempts` times with increasing delays:
/// - After 1st failure: 30 seconds
/// - After 2nd failure: 2 minutes
/// - After 3rd+ failure: 5 minutes
///
/// Returns `Ok(T)` on success or `Err(E)` if all attempts are exhausted.
pub async fn retry_with_backoff<F, Fut, T, E>(mut task_fn: F, max_attempts: u32) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let delays = [
        Duration::from_secs(30),     // 30 seconds
        Duration::from_secs(2 * 60), // 2 minutes
        Duration::from_secs(5 * 60), // 5 minutes
    ];

    for attempt in 1..=max_attempts {
        match task_fn().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt >= max_attempts {
                    error!(
                        "Task failed after {} attempts (max: {})",
                        attempt, max_attempts
                    );
                    return Err(e);
                }

                let delay = if attempt == 1 {
                    delays[0]
                } else if attempt == 2 {
                    delays[1]
                } else {
                    delays[2]
                };

                warn!(
                    "Task failed on attempt {}/{}. Retrying in {:?}...",
                    attempt, max_attempts, delay
                );

                tokio::time::sleep(delay).await;
            }
        }
    }

    unreachable!("Loop should have returned via Ok or Err")
}

/// Submit a removal using browser automation for JS-heavy opt-out flows.
///
/// Initializes the browser engine on first call, navigates to the form URL,
/// fills fields based on the BrowserForm configuration, clicks submit, and
/// captures a screenshot as evidence stored in the database.
///
/// # Arguments
/// * `broker_def` - Broker definition with BrowserForm removal config
/// * `attempt_id` - ID of the removal attempt (for evidence FK)
/// * `field_values` - Decrypted field values mapped from the user profile
/// * `browser_engine_mutex` - Shared lazy-initialized browser engine
/// * `db` - Database for storing screenshot evidence
pub async fn submit_via_browser(
    broker_def: &spectral_broker::definition::BrokerDefinition,
    attempt_id: &str,
    field_values: &HashMap<String, String>,
    browser_engine_mutex: &Mutex<Option<Arc<BrowserEngine>>>,
    db: &Database,
) -> Result<RemovalOutcome, String> {
    let RemovalMethod::BrowserForm {
        url,
        form_selectors,
        ..
    } = &broker_def.removal
    else {
        return Err("submit_via_browser called with non-BrowserForm removal method".to_string());
    };

    // Lock the shared browser engine and initialize if needed
    let mut engine_guard = browser_engine_mutex.lock().await;
    if engine_guard.is_none() {
        info!("Initializing browser engine for first browser-form removal");
        let engine = Arc::new(
            BrowserEngine::new()
                .await
                .map_err(|e| format!("Failed to initialize browser engine: {}", e))?,
        );
        *engine_guard = Some(engine);
    }
    let engine = engine_guard
        .as_ref()
        .expect("engine initialized above")
        .as_ref();

    info!(
        "submit_via_browser: navigating to {} for attempt {}",
        url, attempt_id
    );

    // Navigate to the opt-out form
    engine
        .navigate(url)
        .await
        .map_err(|e| format!("Navigation failed: {}", e))?;

    // Fill listing URL field if selector and value present
    if let (Some(selector), Some(value)) = (
        &form_selectors.listing_url_input,
        field_values.get("listing_url"),
    ) {
        engine
            .fill_field(selector, value)
            .await
            .map_err(|e| format!("Failed to fill listing_url field: {}", e))?;
    }

    // Fill email field
    if let (Some(selector), Some(value)) = (&form_selectors.email_input, field_values.get("email"))
    {
        engine
            .fill_field(selector, value)
            .await
            .map_err(|e| format!("Failed to fill email field: {}", e))?;
    }

    // Fill first name field
    if let (Some(selector), Some(value)) = (
        &form_selectors.first_name_input,
        field_values.get("first_name"),
    ) {
        engine
            .fill_field(selector, value)
            .await
            .map_err(|e| format!("Failed to fill first_name field: {}", e))?;
    }

    // Fill last name field
    if let (Some(selector), Some(value)) = (
        &form_selectors.last_name_input,
        field_values.get("last_name"),
    ) {
        engine
            .fill_field(selector, value)
            .await
            .map_err(|e| format!("Failed to fill last_name field: {}", e))?;
    }

    // Fill full name field (if separate from first/last)
    if let (Some(selector), Some(first), Some(last)) = (
        &form_selectors.full_name_input,
        field_values.get("first_name"),
        field_values.get("last_name"),
    ) {
        let full_name = format!("{} {}", first, last);
        engine
            .fill_field(selector, &full_name)
            .await
            .map_err(|e| format!("Failed to fill full_name field: {}", e))?;
    }

    // Check for CAPTCHA before submitting
    if let Some(captcha_selector) = &form_selectors.captcha_frame {
        // If CAPTCHA element is present, we cannot proceed automatically
        if engine
            .wait_for_selector(captcha_selector, 1000)
            .await
            .is_ok()
        {
            warn!(
                "CAPTCHA detected on browser-form for attempt {}",
                attempt_id
            );
            let screenshot = engine.screenshot().await.unwrap_or_else(|e| {
                warn!(
                    "Screenshot capture failed for attempt {}: {}",
                    attempt_id, e
                );
                vec![]
            });
            store_screenshot_evidence(db, attempt_id, screenshot).await?;
            return Ok(RemovalOutcome::RequiresCaptcha {
                captcha_url: url.clone(),
            });
        }
    }

    // Click the submit button
    if !form_selectors.submit_button.is_empty() {
        engine
            .click(&form_selectors.submit_button)
            .await
            .map_err(|e| format!("Failed to click submit button: {}", e))?;
    }

    // Wait briefly for page response and check for success/error indicators
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Check for error indicator first
    if let Some(error_selector) = &form_selectors.error_indicator {
        if engine.wait_for_selector(error_selector, 500).await.is_ok() {
            let error_text = engine
                .extract_text(error_selector)
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            let screenshot = engine.screenshot().await.unwrap_or_else(|e| {
                warn!(
                    "Screenshot capture failed for attempt {}: {}",
                    attempt_id, e
                );
                vec![]
            });
            store_screenshot_evidence(db, attempt_id, screenshot).await?;
            return Ok(RemovalOutcome::Failed {
                reason: format!("Form error: {}", error_text),
                error_details: None,
            });
        }
    }

    // Take screenshot as evidence
    let screenshot = engine.screenshot().await.unwrap_or_else(|e| {
        warn!(
            "Screenshot capture failed for attempt {}: {}",
            attempt_id, e
        );
        vec![]
    });
    store_screenshot_evidence(db, attempt_id, screenshot).await?;

    info!(
        "submit_via_browser: form submitted successfully for attempt {}",
        attempt_id
    );

    Ok(RemovalOutcome::Submitted)
}

/// Store screenshot evidence for a removal attempt.
async fn store_screenshot_evidence(
    db: &Database,
    attempt_id: &str,
    screenshot_bytes: Vec<u8>,
) -> Result<(), String> {
    let evidence_id = uuid::Uuid::new_v4().to_string();
    let captured_at = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO removal_evidence (id, attempt_id, screenshot_bytes, captured_at) VALUES (?, ?, ?, ?)"
    )
    .bind(&evidence_id)
    .bind(attempt_id)
    .bind(&screenshot_bytes)
    .bind(&captured_at)
    .execute(db.pool())
    .await
    .map_err(|e| format!("Failed to store screenshot evidence: {}", e))?;

    info!(
        "Stored screenshot evidence {} for attempt {}",
        evidence_id, attempt_id
    );

    Ok(())
}

/// Submit a removal request via email.
///
/// Renders the email template with profile data, then either:
/// - Sends via SMTP if config is available
/// - Logs the mailto: URL for manual sending (stored in error_message field)
///
/// Logs the attempt to the `email_removals` table.
///
/// # Arguments
/// * `broker_def` - Broker definition with Email removal config
/// * `attempt_id` - ID of the removal attempt
/// * `field_values` - Decrypted field values from profile
/// * `smtp_config` - Optional SMTP configuration for sending
/// * `db` - Database for logging
pub async fn submit_via_email(
    broker_def: &spectral_broker::definition::BrokerDefinition,
    attempt_id: &str,
    field_values: &HashMap<String, String>,
    smtp_config: Option<&spectral_mail::SmtpConfig>,
    db: &Database,
) -> Result<RemovalOutcome, String> {
    let RemovalMethod::Email {
        email: to_email,
        body: body_template,
        ..
    } = &broker_def.removal
    else {
        return Err("submit_via_email called with non-Email removal method".to_string());
    };

    // Extract user email from field_values
    let user_email = field_values
        .get("email")
        .ok_or("Missing required field: email")?;

    info!(
        "submit_via_email: rendering template for attempt {}",
        attempt_id
    );

    // Render email template
    let email_template = spectral_mail::templates::render_template(
        body_template,
        user_email,
        to_email,
        field_values,
    );

    // Generate body hash for logging (never store the actual body)
    let body_hash = spectral_mail::sender::body_hash(&email_template.body);

    // Determine send method
    let send_method = if smtp_config.is_some() {
        "smtp"
    } else {
        "mailto"
    };

    // Send via SMTP or log as ready for manual sending
    if let Some(config) = smtp_config {
        info!(
            "submit_via_email: sending via SMTP for attempt {}",
            attempt_id
        );
        spectral_mail::sender::send_smtp(&email_template, user_email, config)
            .await
            .map_err(|e| format!("SMTP send failed: {}", e))?;
    } else {
        info!(
            "submit_via_email: email ready for manual sending for attempt {}",
            attempt_id
        );
        // Note: When SMTP is not configured, the email details are logged to
        // the email_removals table. The frontend (Task 16) will provide a UI
        // to re-generate and send the email via mailto: URL.
        // For now, we mark this as submitted since the email is logged and ready.
    }

    // Log to email_removals table
    let email_removal_id = uuid::Uuid::new_v4().to_string();
    let sent_at = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO email_removals (id, attempt_id, broker_id, sent_at, method, recipient, subject, body_hash) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&email_removal_id)
    .bind(attempt_id)
    .bind(broker_def.broker.id.to_string())
    .bind(&sent_at)
    .bind(send_method)
    .bind(to_email)
    .bind(&email_template.subject)
    .bind(&body_hash)
    .execute(db.pool())
    .await
    .map_err(|e| format!("Failed to log email removal: {}", e))?;

    info!(
        "Logged email removal {} for attempt {}",
        email_removal_id, attempt_id
    );

    Ok(RemovalOutcome::Submitted)
}

/// Submit a removal request for a single attempt.
///
/// Worker task that:
/// 1. Loads removal attempt, finding, and profile data
/// 2. Maps fields for form submission
/// 3. Routes to browser or HTTP form submission based on broker removal method
/// 4. Updates database based on outcome
/// 5. Returns result for event emission
///
/// # Arguments
/// * `db` - Database connection
/// * `vault` - Unlocked vault for profile access
/// * `removal_attempt_id` - ID of removal attempt to process
/// * `broker_registry` - Registry for broker definitions
/// * `semaphore` - Concurrency limiter (max 3 concurrent)
/// * `browser_engine` - Shared lazy-initialized browser engine for browser-form removals
pub async fn submit_removal_task(
    db: Arc<Database>,
    vault: Arc<spectral_vault::Vault>,
    removal_attempt_id: String,
    broker_registry: Arc<BrokerRegistry>,
    semaphore: Arc<Semaphore>,
    browser_engine: Arc<Mutex<Option<Arc<BrowserEngine>>>>,
) -> Result<WorkerResult, String> {
    // Acquire semaphore permit (wait if 3 tasks active)
    let _permit = semaphore
        .acquire()
        .await
        .map_err(|e| format!("Failed to acquire semaphore: {}", e))?;

    info!(
        "Worker acquired permit for removal attempt: {}",
        removal_attempt_id
    );

    // Load removal attempt from database
    let removal_attempt = removal_attempts::get_by_id(db.pool(), &removal_attempt_id)
        .await
        .map_err(|e| format!("Failed to load removal attempt: {}", e))?
        .ok_or_else(|| format!("Removal attempt not found: {}", removal_attempt_id))?;

    // Load associated finding
    let finding = spectral_db::findings::get_by_id(db.pool(), &removal_attempt.finding_id)
        .await
        .map_err(|e| format!("Failed to load finding: {}", e))?
        .ok_or_else(|| format!("Finding not found: {}", removal_attempt.finding_id))?;

    // Load profile
    let profile_id = spectral_core::types::ProfileId::new(&finding.profile_id)
        .map_err(|e| format!("Invalid profile ID: {}", e))?;

    let profile = vault
        .load_profile(&profile_id)
        .await
        .map_err(|e| format!("Failed to load profile: {}", e))?;

    // Get encryption key from vault
    let key = vault
        .encryption_key()
        .map_err(|e| format!("Failed to get encryption key: {}", e))?;

    // Map fields for submission
    let field_values = map_fields_for_submission(&profile, &finding.listing_url, key)?;

    // Load broker definition
    let broker_id = BrokerId::new(&removal_attempt.broker_id)
        .map_err(|e| format!("Invalid broker ID: {}", e))?;

    let broker_def = broker_registry
        .get(&broker_id)
        .map_err(|e| format!("Failed to get broker definition: {}", e))?;

    // Route submission based on broker removal method
    let outcome = match &broker_def.removal {
        RemovalMethod::BrowserForm { .. } => {
            info!(
                "Routing removal attempt {} via browser-form",
                removal_attempt_id
            );
            retry_with_backoff(
                || async {
                    submit_via_browser(
                        &broker_def,
                        &removal_attempt_id,
                        &field_values,
                        &browser_engine,
                        &db,
                    )
                    .await
                },
                3,
            )
            .await?
        }
        RemovalMethod::Email { .. } => {
            info!("Routing removal attempt {} via email", removal_attempt_id);
            // Note: SMTP config is not available yet (added in Task 15)
            // For now, we pass None which will store a mailto: URL
            retry_with_backoff(
                || async {
                    submit_via_email(
                        &broker_def,
                        &removal_attempt_id,
                        &field_values,
                        None, // SMTP config added in Task 15
                        &db,
                    )
                    .await
                },
                3,
            )
            .await?
        }
        _ => {
            info!(
                "Routing removal attempt {} via HTTP form",
                removal_attempt_id
            );
            // Create WebFormSubmitter (creates its own browser engine)
            let submitter = WebFormSubmitter::new()
                .await
                .map_err(|e| format!("Failed to create submitter: {}", e))?;

            retry_with_backoff(
                || async {
                    submitter
                        .submit(&broker_def, field_values.clone())
                        .await
                        .map_err(|e| format!("Submission failed: {}", e))
                },
                3,
            )
            .await?
        }
    };

    // Update database based on outcome
    match &outcome {
        RemovalOutcome::Submitted | RemovalOutcome::RequiresEmailVerification { .. } => {
            let now = chrono::Utc::now();
            removal_attempts::update_status(
                db.pool(),
                &removal_attempt_id,
                RemovalStatus::Submitted,
                Some(now),
                None,
                None,
            )
            .await
            .map_err(|e| format!("Failed to update status to Submitted: {}", e))?;

            info!("Removal submitted successfully: {}", removal_attempt_id);
        }
        RemovalOutcome::RequiresCaptcha { captcha_url } => {
            // Keep status as Pending but set error message for CAPTCHA queue
            removal_attempts::update_status(
                db.pool(),
                &removal_attempt_id,
                RemovalStatus::Pending,
                None,
                None,
                Some(format!("CAPTCHA_REQUIRED:{}", captcha_url)),
            )
            .await
            .map_err(|e| format!("Failed to update for CAPTCHA: {}", e))?;

            warn!("CAPTCHA required for removal: {}", removal_attempt_id);
        }
        RemovalOutcome::Failed { reason, .. } => {
            // Mark as failed with error message
            removal_attempts::update_status(
                db.pool(),
                &removal_attempt_id,
                RemovalStatus::Failed,
                None,
                None,
                Some(reason.clone()),
            )
            .await
            .map_err(|e| format!("Failed to update status to Failed: {}", e))?;

            error!("Removal failed: {} - {}", removal_attempt_id, reason);
        }
        RemovalOutcome::RequiresAccountCreation => {
            // Treat as failed - account creation not supported
            removal_attempts::update_status(
                db.pool(),
                &removal_attempt_id,
                RemovalStatus::Failed,
                None,
                None,
                Some("Account creation required (not supported)".to_string()),
            )
            .await
            .map_err(|e| format!("Failed to update for account creation: {}", e))?;

            warn!(
                "Account creation required (unsupported): {}",
                removal_attempt_id
            );
        }
    }

    // Return result (permit is dropped here, releasing semaphore)
    Ok(WorkerResult {
        removal_attempt_id,
        outcome,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral_core::types::{ProfileId, Timestamp};
    use spectral_vault::cipher::EncryptedField;

    fn test_key() -> [u8; 32] {
        [0u8; 32]
    }

    #[allow(deprecated)]
    fn create_test_profile(key: &[u8; 32]) -> UserProfile {
        UserProfile {
            id: ProfileId::generate(),
            full_name: None,
            first_name: Some(
                EncryptedField::encrypt(&"John".to_string(), key).expect("encrypt first_name"),
            ),
            middle_name: None,
            last_name: Some(
                EncryptedField::encrypt(&"Doe".to_string(), key).expect("encrypt last_name"),
            ),
            email: Some(
                EncryptedField::encrypt(&"john@example.com".to_string(), key)
                    .expect("encrypt email"),
            ),
            phone: None,
            address: None,
            city: None,
            state: None,
            zip_code: None,
            country: None,
            date_of_birth: None,
            ssn: None,
            employer: None,
            job_title: None,
            education: None,
            social_media: None,
            previous_addresses_v1: None,
            phone_numbers: vec![],
            email_addresses: vec![],
            previous_addresses_v2: vec![],
            aliases: vec![],
            relatives: vec![],
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
        }
    }

    #[test]
    fn test_map_fields_success() {
        let key = test_key();
        let profile = create_test_profile(&key);
        let listing_url = "https://spokeo.com/person/123";

        let fields = map_fields_for_submission(&profile, listing_url, &key).expect("map fields");

        assert_eq!(fields.get("listing_url"), Some(&listing_url.to_string()));
        assert_eq!(fields.get("email"), Some(&"john@example.com".to_string()));
        assert_eq!(fields.get("first_name"), Some(&"John".to_string()));
        assert_eq!(fields.get("last_name"), Some(&"Doe".to_string()));
    }

    #[test]
    #[allow(deprecated)]
    fn test_map_fields_missing_email() {
        let key = test_key();
        let mut profile = create_test_profile(&key);
        profile.email = None;
        let listing_url = "https://spokeo.com/person/123";

        let result = map_fields_for_submission(&profile, listing_url, &key);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Missing required field: email"));
    }

    #[tokio::test]
    async fn test_retry_with_backoff_succeeds_on_second_attempt() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let task = || {
            let count = attempt_count_clone.clone();
            async move {
                // nosemgrep: llm-prompt-injection-risk
                let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 2 {
                    Err("Transient error")
                } else {
                    Ok("Success")
                }
            }
        };

        let result = retry_with_backoff(task, 3).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_fails_after_max_attempts() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let task = || {
            let count = attempt_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>("Persistent error")
            }
        };

        let result = retry_with_backoff(task, 3).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Persistent error");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }
}
