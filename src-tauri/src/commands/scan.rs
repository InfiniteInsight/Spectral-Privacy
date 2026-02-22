use crate::removal_worker::submit_removal_task;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use spectral_broker::{BrokerRegistry, RemovalMethod, ScanPriority};
use spectral_browser::BrowserEngine;
use spectral_core::types::{BrokerId, ProfileId};
use spectral_scanner::{BrokerFilter, ScanOrchestrator};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, State};
use tauri_plugin_shell::ShellExt;
use tokio::sync::Semaphore;
use tracing::info;
use uuid::Uuid;

/// Scan tier for filtering brokers by priority
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ScanTier {
    /// Top ~10 brokers (AutoScanTier1)
    Tier1,
    /// Top ~30 brokers (AutoScanTier1 + AutoScanTier2)
    Tier2,
    /// All brokers except ManualOnly
    All,
    /// Custom broker selection (use broker_ids parameter)
    Custom,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartScanRequest {
    pub profile_id: String,
    pub broker_filter: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScanJobResponse {
    pub id: String,
    pub status: String,
    pub completed_brokers: u32,
    pub total_brokers: u32,
}

#[derive(Debug, Serialize)]
pub struct BatchSubmissionResult {
    pub job_id: String,
    pub total_count: usize,
    pub queued_count: usize,
}

#[derive(Debug, Serialize)]
pub struct FindingResponse {
    pub id: String,
    pub broker_id: String,
    pub listing_url: String,
    pub verification_status: String,
    pub extracted_data: ExtractedDataResponse,
    pub discovered_at: String,
}

#[derive(Debug, Serialize)]
pub struct ExtractedDataResponse {
    pub name: Option<String>,
    pub age: Option<u32>,
    pub addresses: Vec<String>,
    pub phone_numbers: Vec<String>,
    pub relatives: Vec<String>,
    pub emails: Vec<String>,
}

/// Convert database Finding to API response.
fn finding_to_response(finding: spectral_db::findings::Finding) -> FindingResponse {
    // Extract fields from JSON extracted_data
    let name = finding
        .extracted_data
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let age = finding
        .extracted_data
        .get("age")
        .and_then(|v| v.as_u64())
        .map(|a| a as u32);

    let addresses = finding
        .extracted_data
        .get("addresses")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let phone_numbers = finding
        .extracted_data
        .get("phone_numbers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let relatives = finding
        .extracted_data
        .get("relatives")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let emails = finding
        .extracted_data
        .get("emails")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    FindingResponse {
        id: finding.id,
        broker_id: finding.broker_id,
        listing_url: finding.listing_url,
        verification_status: finding.verification_status.to_string(),
        extracted_data: ExtractedDataResponse {
            name,
            age,
            addresses,
            phone_numbers,
            relatives,
            emails,
        },
        discovered_at: finding.discovered_at.to_rfc3339(),
    }
}

#[tauri::command]
pub async fn start_scan(
    state: State<'_, AppState>,
    vault_id: String,
    profile_id: String,
    _broker_filter: Option<String>, // Deprecated: use tier parameter instead
    tier: Option<ScanTier>,
    broker_ids: Option<Vec<String>>,
) -> Result<ScanJobResponse, String> {
    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get the profile from the vault
    let profile_id =
        ProfileId::new(&profile_id).map_err(|e| format!("Invalid profile ID: {}", e))?;

    let profile = vault
        .load_profile(&profile_id)
        .await
        .map_err(|e| format!("Failed to load profile: {}", e))?;

    // Get the vault's encryption key
    let vault_key = vault
        .encryption_key()
        .map_err(|e| format!("Failed to get vault key: {}", e))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Create orchestrator for this scan
    // TODO: These should be cached/shared across scans
    // Note: We can't clone EncryptedPool (it contains Zeroizing secrets),
    // but Pool<Sqlite> itself is Arc-based and can be cloned.
    // For now, we create a temporary EncryptedPool from the existing pool.
    // In production, the orchestrator should be a singleton in AppState.
    let broker_registry = state.broker_registry.clone();
    let browser_engine = Arc::new(
        BrowserEngine::new()
            .await
            .map_err(|e| format!("Failed to create browser engine: {}", e))?,
    );

    // Get the underlying Pool<Sqlite> which can be cloned
    let pool = db.pool().clone();
    let vault_key_vec = vault_key.to_vec();

    // Create a new EncryptedPool with the same pool and key
    // This is safe because both point to the same underlying connection pool
    use spectral_db::{Database, EncryptedPool};
    let encrypted_pool = EncryptedPool::from_pool(pool, vault_key_vec);
    let database = Database::from_encrypted_pool(encrypted_pool);
    let db = Arc::new(database);

    let orchestrator = ScanOrchestrator::new(broker_registry.clone(), browser_engine, db.clone())
        .with_max_concurrent_scans(4);

    // Filter brokers based on tier or custom IDs
    let all_brokers = broker_registry.get_all();

    let selected_brokers: Vec<_> = match (&tier, &broker_ids) {
        (_, Some(ids)) => {
            // Custom broker selection takes precedence
            all_brokers
                .iter()
                .filter(|b| ids.contains(&b.broker.id.to_string()))
                .cloned()
                .collect()
        }
        (Some(ScanTier::Tier1), _) => {
            // Only Tier 1 brokers
            all_brokers
                .iter()
                .filter(|b| b.broker.scan_priority == ScanPriority::AutoScanTier1)
                .cloned()
                .collect()
        }
        (Some(ScanTier::Tier2), _) => {
            // Tier 1 and Tier 2 brokers
            all_brokers
                .iter()
                .filter(|b| {
                    matches!(
                        b.broker.scan_priority,
                        ScanPriority::AutoScanTier1 | ScanPriority::AutoScanTier2
                    )
                })
                .cloned()
                .collect()
        }
        _ => {
            // All brokers except ManualOnly (default)
            all_brokers
                .iter()
                .filter(|b| b.broker.scan_priority != ScanPriority::ManualOnly)
                .cloned()
                .collect()
        }
    };

    // If tier or broker_ids filtering was applied but resulted in empty list, return error
    if (tier.is_some() || broker_ids.is_some()) && selected_brokers.is_empty() {
        return Err("No brokers matched the specified tier or IDs".to_string());
    }

    // Convert selected brokers to IDs for filtering
    let broker_ids_filter: Vec<String> = selected_brokers
        .iter()
        .map(|b| b.broker.id.to_string())
        .collect();

    // Create appropriate filter based on selected brokers
    let filter = if broker_ids_filter.is_empty() {
        BrokerFilter::All
    } else {
        BrokerFilter::Specific(broker_ids_filter)
    };

    // Start the scan with tier-based filter
    let job_id = orchestrator
        .start_scan(&profile, filter, vault_key)
        .await
        .map_err(|e| format!("Failed to start scan: {}", e))?;

    // Query the job to get complete information including total_brokers
    let job = sqlx::query_as::<_, (String, String, i64, i64)>(
        "SELECT id, status, completed_brokers, total_brokers FROM scan_jobs WHERE id = ?",
    )
    .bind(&job_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Failed to get scan status: {}", e))?;

    Ok(ScanJobResponse {
        id: job.0,
        status: job.1,
        completed_brokers: job.2 as u32,
        total_brokers: job.3 as u32,
    })
}

#[tauri::command]
pub async fn get_scan_status(
    state: State<'_, AppState>,
    vault_id: String,
    scan_job_id: String,
) -> Result<ScanJobResponse, String> {
    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Query with all required fields for progress tracking
    let job = sqlx::query_as::<_, (String, String, i64, i64)>(
        "SELECT id, status, completed_brokers, total_brokers FROM scan_jobs WHERE id = ?",
    )
    .bind(scan_job_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Failed to get scan status: {}", e))?;

    Ok(ScanJobResponse {
        id: job.0,
        status: job.1,
        completed_brokers: job.2 as u32,
        total_brokers: job.3 as u32,
    })
}

/// Get findings for a scan job with optional verification status filter.
#[tauri::command]
pub async fn get_findings(
    state: State<'_, AppState>,
    vault_id: String,
    scan_job_id: String,
    filter: Option<String>,
) -> Result<Vec<FindingResponse>, String> {
    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Get all findings for this scan job
    let mut findings = spectral_db::findings::get_by_scan_job(db.pool(), &scan_job_id)
        .await
        .map_err(|e| format!("Failed to get findings: {}", e))?;

    // Filter by verification status if requested
    if let Some(filter_status) = filter {
        findings.retain(|f| f.verification_status.to_string() == filter_status);
    }

    // Convert to response format
    let responses: Vec<FindingResponse> = findings.into_iter().map(finding_to_response).collect();

    Ok(responses)
}

/// Update the verification status of a finding.
#[tauri::command]
pub async fn verify_finding(
    state: State<'_, AppState>,
    vault_id: String,
    finding_id: String,
    is_match: bool,
) -> Result<(), String> {
    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Update verification status
    spectral_db::findings::verify_finding(
        db.pool(),
        &finding_id,
        is_match,
        true, // verified_by_user = true
    )
    .await
    .map_err(|e| format!("Failed to verify finding: {}", e))?;

    Ok(())
}

/// Submit removal requests for confirmed findings
#[tauri::command]
pub async fn submit_removals_for_confirmed(
    state: State<'_, AppState>,
    vault_id: String,
    scan_job_id: String,
) -> Result<Vec<String>, String> {
    // Get unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not found or locked".to_string())?;

    // Get database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Query all findings for this scan
    let findings = spectral_db::findings::get_by_scan_job(db.pool(), &scan_job_id)
        .await
        .map_err(|e| e.to_string())?;

    // Filter to confirmed findings
    let confirmed_findings = findings
        .into_iter()
        .filter(|f| f.verification_status == spectral_db::findings::VerificationStatus::Confirmed)
        .collect::<Vec<_>>();

    // Create removal attempt for each confirmed finding
    let mut removal_ids = Vec::new();
    for finding in confirmed_findings {
        let removal_attempt = spectral_db::removal_attempts::create_removal_attempt(
            db.pool(),
            finding.id,
            finding.broker_id,
        )
        .await
        .map_err(|e| e.to_string())?;

        removal_ids.push(removal_attempt.id);
    }

    Ok(removal_ids)
}

/// Process a batch of removal attempts with parallel workers.
///
/// Spawns async worker tasks for each removal_attempt_id (max 3 concurrent).
/// Returns immediately with a job_id. Real-time events are emitted as tasks complete.
///
/// # Events
/// - `removal:started`: When task begins processing
/// - `removal:success`: When removal is submitted successfully
/// - `removal:captcha`: When CAPTCHA is required
/// - `removal:failed`: When removal fails
#[tauri::command]
pub async fn process_removal_batch<R: tauri::Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
    vault_id: String,
    removal_attempt_ids: Vec<String>,
) -> Result<BatchSubmissionResult, String> {
    // Get unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not found or locked".to_string())?;

    // Get database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Get the underlying Pool<Sqlite> which can be cloned
    let pool = db.pool().clone();
    let vault_key = vault
        .encryption_key()
        .map_err(|e| format!("Failed to get vault key: {}", e))?;
    let vault_key_vec = vault_key.to_vec();

    // Create a new EncryptedPool with the same pool and key
    use spectral_db::{Database, EncryptedPool};
    let encrypted_pool = EncryptedPool::from_pool(pool, vault_key_vec);
    let database = Database::from_encrypted_pool(encrypted_pool);
    let db = Arc::new(database);

    // Create shared resources
    let broker_registry = Arc::new(BrokerRegistry::new());
    let semaphore = Arc::new(Semaphore::new(3)); // Max 3 concurrent
    let browser_engine = state.browser_engine.clone();

    // Generate job_id
    let job_id = Uuid::new_v4().to_string();

    // Count of removal attempts
    let total_count = removal_attempt_ids.len();
    let queued_count = total_count; // All are queued for processing

    // Spawn worker tasks for each removal attempt
    for attempt_id in removal_attempt_ids {
        let db_clone = db.clone();
        let vault_clone = Arc::clone(&vault);
        let broker_registry_clone = broker_registry.clone();
        let semaphore_clone = semaphore.clone();
        let browser_engine_clone = browser_engine.clone();
        let job_id_clone = job_id.clone();
        let app_handle = app.clone();
        let attempt_id_clone = attempt_id.clone();

        tokio::spawn(async move {
            // Emit started event
            let _ = app_handle.emit(
                "removal:started",
                serde_json::json!({
                    "job_id": job_id_clone,
                    "attempt_id": attempt_id_clone
                }),
            );

            // Execute worker task
            let result = submit_removal_task(
                db_clone,
                vault_clone,
                attempt_id_clone.clone(),
                broker_registry_clone,
                semaphore_clone,
                browser_engine_clone,
            )
            .await;

            // Emit result event based on outcome
            match result {
                Ok(worker_result) => match worker_result.outcome {
                    spectral_broker::removal::RemovalOutcome::Submitted
                    | spectral_broker::removal::RemovalOutcome::RequiresEmailVerification {
                        ..
                    } => {
                        let _ = app_handle.emit(
                            "removal:success",
                            serde_json::json!({
                                "job_id": job_id_clone,
                                "attempt_id": attempt_id_clone,
                                "outcome": format!("{:?}", worker_result.outcome)
                            }),
                        );
                    }
                    spectral_broker::removal::RemovalOutcome::RequiresCaptcha { .. } => {
                        let _ = app_handle.emit(
                            "removal:captcha",
                            serde_json::json!({
                                "job_id": job_id_clone,
                                "attempt_id": attempt_id_clone,
                                "outcome": format!("{:?}", worker_result.outcome)
                            }),
                        );
                    }
                    spectral_broker::removal::RemovalOutcome::Failed { .. }
                    | spectral_broker::removal::RemovalOutcome::RequiresAccountCreation => {
                        let _ = app_handle.emit(
                            "removal:failed",
                            serde_json::json!({
                                "job_id": job_id_clone,
                                "attempt_id": attempt_id_clone,
                                "error": format!("{:?}", worker_result.outcome)
                            }),
                        );
                    }
                },
                Err(error) => {
                    let _ = app_handle.emit(
                        "removal:failed",
                        serde_json::json!({
                            "job_id": job_id_clone,
                            "attempt_id": attempt_id_clone,
                            "error": error
                        }),
                    );
                }
            }
        });
    }

    // Return immediately with job info
    Ok(BatchSubmissionResult {
        job_id,
        total_count,
        queued_count,
    })
}

/// Get all removal attempts in the CAPTCHA queue.
///
/// Returns removal attempts that require CAPTCHA resolution, ordered oldest first.
#[tauri::command]
pub async fn get_captcha_queue(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<spectral_db::removal_attempts::RemovalAttempt>, String> {
    // Get unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Get CAPTCHA queue
    spectral_db::removal_attempts::get_captcha_queue(db.pool())
        .await
        .map_err(|e| format!("Failed to get CAPTCHA queue: {}", e))
}

/// Get all removal attempts in the failed queue.
///
/// Returns removal attempts that have failed, ordered newest first.
#[tauri::command]
pub async fn get_failed_queue(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<spectral_db::removal_attempts::RemovalAttempt>, String> {
    // Get unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Get failed queue
    spectral_db::removal_attempts::get_failed_queue(db.pool())
        .await
        .map_err(|e| format!("Failed to get failed queue: {}", e))
}

/// Get all removal attempts for a scan job.
///
/// Returns all removal attempts for findings associated with the given scan job.
#[tauri::command]
pub async fn get_removal_attempts_by_scan_job(
    state: State<'_, AppState>,
    vault_id: String,
    scan_job_id: String,
) -> Result<Vec<spectral_db::removal_attempts::RemovalAttempt>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not found or not unlocked".to_string())?;

    let db = vault
        .database()
        .map_err(|e| format!("Failed to access database: {}", e))?;

    spectral_db::removal_attempts::get_by_scan_job_id(db.pool(), &scan_job_id)
        .await
        .map_err(|e| format!("Failed to query removal attempts: {}", e))
}

/// Get job history: removal attempts grouped by scan job, newest first.
///
/// Returns one summary per scan job that has at least one removal attempt.
#[tauri::command]
pub async fn get_removal_job_history(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<spectral_db::removal_attempts::RemovalJobSummary>, String> {
    // Get unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Get job history
    spectral_db::removal_attempts::get_job_history(db.pool())
        .await
        .map_err(|e| format!("Failed to get job history: {}", e))
}

/// Retry a failed removal attempt.
///
/// Resets the removal attempt to Pending status and spawns a new worker task
/// to reprocess the submission. Returns immediately while the retry runs in background.
///
/// # Events
/// - `removal:retry`: When retry begins
/// - `removal:success`: When removal is submitted successfully
/// - `removal:captcha`: When CAPTCHA is required
/// - `removal:failed`: When removal fails
#[tauri::command]
pub async fn retry_removal<R: tauri::Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
    vault_id: String,
    removal_attempt_id: String,
) -> Result<(), String> {
    // Get unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Reset status to Pending, clear timestamps and error
    spectral_db::removal_attempts::update_status(
        db.pool(),
        &removal_attempt_id,
        spectral_db::removal_attempts::RemovalStatus::Pending,
        None, // Clear submitted_at
        None, // Clear completed_at
        None, // Clear error_message
    )
    .await
    .map_err(|e| format!("Failed to reset removal attempt: {}", e))?;

    // Get the underlying Pool<Sqlite> which can be cloned
    let pool = db.pool().clone();
    let vault_key = vault
        .encryption_key()
        .map_err(|e| format!("Failed to get vault key: {}", e))?;
    let vault_key_vec = vault_key.to_vec();

    // Create a new EncryptedPool with the same pool and key
    use spectral_db::{Database, EncryptedPool};
    let encrypted_pool = EncryptedPool::from_pool(pool, vault_key_vec);
    let database = Database::from_encrypted_pool(encrypted_pool);
    let db = Arc::new(database);

    // Create shared resources
    let broker_registry = Arc::new(BrokerRegistry::new());
    let semaphore = Arc::new(Semaphore::new(3)); // Max 3 concurrent
    let vault_clone = Arc::clone(&vault);
    let browser_engine = state.browser_engine.clone();

    // Spawn background worker task
    let attempt_id_clone = removal_attempt_id.clone();
    tokio::spawn(async move {
        // Emit retry event
        let _ = app.emit(
            "removal:retry",
            serde_json::json!({
                "attempt_id": attempt_id_clone
            }),
        );

        // Execute worker task
        let result = submit_removal_task(
            db,
            vault_clone,
            attempt_id_clone.clone(),
            broker_registry,
            semaphore,
            browser_engine,
        )
        .await;

        // Emit result event based on outcome
        match result {
            Ok(worker_result) => match worker_result.outcome {
                spectral_broker::removal::RemovalOutcome::Submitted
                | spectral_broker::removal::RemovalOutcome::RequiresEmailVerification { .. } => {
                    let _ = app.emit(
                        "removal:success",
                        serde_json::json!({
                            "attempt_id": attempt_id_clone,
                            "outcome": format!("{:?}", worker_result.outcome)
                        }),
                    );
                }
                spectral_broker::removal::RemovalOutcome::RequiresCaptcha { .. } => {
                    let _ = app.emit(
                        "removal:captcha",
                        serde_json::json!({
                            "attempt_id": attempt_id_clone,
                            "outcome": format!("{:?}", worker_result.outcome)
                        }),
                    );
                }
                spectral_broker::removal::RemovalOutcome::Failed { .. }
                | spectral_broker::removal::RemovalOutcome::RequiresAccountCreation => {
                    let _ = app.emit(
                        "removal:failed",
                        serde_json::json!({
                            "attempt_id": attempt_id_clone,
                            "error": format!("{:?}", worker_result.outcome)
                        }),
                    );
                }
            },
            Err(error) => {
                let _ = app.emit(
                    "removal:failed",
                    serde_json::json!({
                        "attempt_id": attempt_id_clone,
                        "error": error
                    }),
                );
            }
        }
    });

    // Return immediately
    Ok(())
}

/// Activity event for the dashboard feed.
#[derive(Debug, serde::Serialize)]
pub struct ActivityEvent {
    pub id: String,
    pub event_type: String,
    pub timestamp: String,
    pub description: String,
}

/// Removal attempt counts broken down by status.
#[derive(Debug, serde::Serialize)]
pub struct RemovalCounts {
    pub submitted: i64,
    pub pending: i64,
    pub failed: i64,
}

/// Aggregated dashboard summary for the home page.
#[derive(Debug, serde::Serialize)]
pub struct DashboardSummary {
    pub privacy_score: Option<u8>,
    pub brokers_scanned: i64,
    pub brokers_total: i64,
    pub last_scan_at: Option<String>,
    pub active_removals: RemovalCounts,
    pub recent_events: Vec<ActivityEvent>,
}

/// Return a dashboard summary for the given vault.
///
/// Aggregates:
/// - Privacy score (if any findings or removals exist)
/// - Count of distinct brokers with at least one finding
/// - Timestamp of the most recent scan job
/// - Removal attempt counts by status
/// - Up to 10 recent activity events (last 5 scans + last 5 removals)
///
/// All queries are pool-scoped; no vault_id WHERE clause is needed.
#[tauri::command]
pub async fn get_dashboard_summary(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<DashboardSummary, String> {
    info!("get_dashboard_summary: vault_id={}", vault_id);
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;
    let pool = db.pool();

    // Count distinct brokers with at least one finding.
    let brokers_scanned: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT broker_id) FROM findings")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Failed to count brokers scanned: {}", e))?;

    // Timestamp of the most recently started scan job.
    let last_scan_at: Option<String> = sqlx::query_scalar("SELECT MAX(started_at) FROM scan_jobs")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Failed to get last scan timestamp: {}", e))?;

    // Removal counts by status.
    let submitted: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM removal_attempts WHERE status = 'Submitted'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count submitted removals: {}", e))?;

    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM removal_attempts WHERE status = 'Pending'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count pending removals: {}", e))?;

    let failed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM removal_attempts WHERE status = 'Failed'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count failed removals: {}", e))?;

    // Compute score only when there is something to base it on.
    let has_data = brokers_scanned > 0 || submitted > 0 || failed > 0;
    let privacy_score = if has_data {
        // Unresolved = confirmed findings with no removal yet.
        let unresolved: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM findings WHERE verification_status = 'Confirmed'",
        )
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Failed to count confirmed findings: {}", e))?;

        Some(calculate_privacy_score(
            unresolved as u32,
            submitted as u32,
            failed as u32,
            0,
        ))
    } else {
        None
    };

    // Last 5 scan jobs as activity events.
    let scan_rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, started_at, status FROM scan_jobs ORDER BY started_at DESC LIMIT 5",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch recent scan jobs: {}", e))?;

    let mut events: Vec<ActivityEvent> = scan_rows
        .into_iter()
        .map(|(id, started_at, status)| ActivityEvent {
            id: id.clone(),
            event_type: "scan".to_string(),
            timestamp: started_at,
            description: format!("Scan {} ({})", &id[..8.min(id.len())], status),
        })
        .collect();

    // Last 5 removal attempts as activity events.
    let removal_rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT id, broker_id, created_at, status FROM removal_attempts ORDER BY created_at DESC LIMIT 5",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch recent removal attempts: {}", e))?;

    for (id, broker_id, created_at, status) in removal_rows {
        events.push(ActivityEvent {
            id: id.clone(),
            event_type: "removal".to_string(),
            timestamp: created_at,
            description: format!(
                "Removal {} for {} ({})",
                &id[..8.min(id.len())],
                broker_id,
                status
            ),
        });
    }

    // Sort all events by timestamp descending, keep top 10.
    events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    events.truncate(10);

    Ok(DashboardSummary {
        privacy_score,
        brokers_scanned,
        brokers_total: 0, // Placeholder — populated in Task 21 (broker explorer)
        last_scan_at,
        active_removals: RemovalCounts {
            submitted,
            pending,
            failed,
        },
        recent_events: events,
    })
}

/// Calculate a privacy score from 0–100 based on finding and removal counts.
///
/// Penalties:
/// - Each unresolved people-search finding: -8 points
/// - Each failed removal attempt: -3 points
/// - Each reappeared listing: -5 points
///
/// Bonuses:
/// - Each confirmed submitted removal: +2 points
///
/// The result is clamped to [0, 100].
pub(crate) fn calculate_privacy_score(
    unresolved_people_search: u32,
    confirmed_removals: u32,
    failed_removals: u32,
    reappeared: u32,
) -> u8 {
    let penalty = (unresolved_people_search * 8) + (failed_removals * 3) + (reappeared * 5); // nosemgrep: llm-prompt-injection-risk
    let bonus = confirmed_removals * 2;
    let raw = 100i32 - penalty as i32 + bonus as i32; // nosemgrep: llm-prompt-injection-risk
    raw.clamp(0, 100) as u8
}

/// Map a privacy score to a human-readable descriptor.
pub(crate) fn score_descriptor(score: u8) -> &'static str {
    match score {
        0..=39 => "At Risk",
        40..=69 => "Improving",
        70..=89 => "Good",
        _ => "Well Protected",
    }
}

/// Result returned by `get_privacy_score`.
#[derive(Debug, serde::Serialize)]
pub struct PrivacyScoreResult {
    pub score: u8,
    pub descriptor: String,
    pub unresolved_count: i64,
    pub confirmed_count: i64,
    pub failed_count: i64,
}

/// Return the current privacy score for the given vault.
///
/// The score is derived from:
/// - Unresolved findings (verification_status = 'Confirmed' but not yet removed)
/// - Submitted removal attempts (status = 'Submitted')
/// - Failed removal attempts (status = 'Failed')
///
/// Note: `removal_attempts` has no `vault_id` column.  The vault's pool is
/// already vault-scoped, so all queries run against that vault's database
/// without an extra WHERE clause on vault identity.
#[tauri::command]
pub async fn get_privacy_score(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<PrivacyScoreResult, String> {
    info!("get_privacy_score: vault_id={}", vault_id);
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;
    let pool = db.pool();

    // Count all confirmed findings. The penalty applies to all Confirmed findings
    // until the listing is verified removed (a future feature).
    // verification_status = 'Confirmed' means the user has verified this is them.
    let unresolved: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM findings WHERE verification_status = 'Confirmed'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count unresolved findings: {}", e))?;

    // Count submitted removal attempts via JOIN (removal_attempts has no vault_id).
    let confirmed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM removal_attempts WHERE status = 'Submitted'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count submitted removals: {}", e))?;

    // Count failed removal attempts.
    let failed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM removal_attempts WHERE status = 'Failed'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count failed removals: {}", e))?;

    let score = calculate_privacy_score(
        unresolved as u32,
        confirmed as u32,
        failed as u32,
        0, // reappeared — tracked in Phase 6 Task 19
    );

    Ok(PrivacyScoreResult {
        score,
        descriptor: score_descriptor(score).to_string(),
        unresolved_count: unresolved,
        confirmed_count: confirmed,
        failed_count: failed,
    })
}

/// Evidence record captured during browser-form removal submissions.
#[derive(Debug, serde::Serialize)]
pub struct RemovalEvidence {
    pub id: String,
    pub attempt_id: String,
    pub screenshot_bytes: Vec<u8>,
    pub captured_at: String,
}

/// Get screenshot evidence for a removal attempt.
///
/// Returns the evidence row associated with the given removal attempt ID,
/// or `None` if no evidence has been captured yet (e.g. HTTP-form removals).
#[tauri::command]
pub async fn get_removal_evidence(
    state: State<'_, AppState>,
    vault_id: String,
    attempt_id: String,
) -> Result<Option<RemovalEvidence>, String> {
    info!(
        "get_removal_evidence: vault_id={}, attempt_id={}",
        vault_id, attempt_id
    );
    let vault = state.get_vault(&vault_id).ok_or("Vault not unlocked")?;
    let db = vault.database().map_err(|e| e.to_string())?;

    use sqlx::Row;
    let row = sqlx::query(
        "SELECT id, attempt_id, screenshot_bytes, captured_at FROM removal_evidence WHERE attempt_id = ? ORDER BY captured_at DESC LIMIT 1"
    )
    .bind(&attempt_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|r| RemovalEvidence {
        id: r.get("id"),
        attempt_id: r.get("attempt_id"),
        screenshot_bytes: r.get("screenshot_bytes"),
        captured_at: r.get("captured_at"),
    }))
}

/// Decrypt all profile fields into a HashMap for template rendering.
fn decrypt_profile_fields(
    profile: &spectral_vault::UserProfile,
    vault_key: &[u8; 32],
) -> HashMap<String, String> {
    let mut fields = HashMap::new();

    // Macro to simplify field decryption
    macro_rules! decrypt_field {
        ($field:expr, $key:expr) => {
            if let Some(ref field) = $field {
                if let Ok(value) = field.decrypt(vault_key) {
                    fields.insert($key.to_string(), value);
                }
            }
        };
    }

    // Decrypt simple fields
    decrypt_field!(profile.full_name, "full_name");
    decrypt_field!(profile.first_name, "first_name");
    decrypt_field!(profile.middle_name, "middle_name");
    decrypt_field!(profile.last_name, "last_name");
    decrypt_field!(profile.address, "address");
    decrypt_field!(profile.city, "city");
    decrypt_field!(profile.state, "state");
    decrypt_field!(profile.zip_code, "zip_code");
    decrypt_field!(profile.date_of_birth, "date_of_birth");

    // Decrypt email from email_addresses array
    if let Some(email_addr) = profile.email_addresses.first() {
        if let Ok(value) = email_addr.email.decrypt(vault_key) {
            fields.insert("email".to_string(), value);
        }
    }

    // Decrypt phone from phone_numbers array
    if let Some(phone_num) = profile.phone_numbers.first() {
        if let Ok(value) = phone_num.number.decrypt(vault_key) {
            fields.insert("phone".to_string(), value);
        }
    }

    fields
}

/// Removal email context loaded from database.
struct RemovalEmailContext {
    email_address: String,
    subject_template: String,
    body_template: String,
    profile: spectral_vault::UserProfile,
}

/// Load and validate removal context (attempt, finding, broker, profile).
async fn load_removal_context(
    state: &tauri::State<'_, AppState>,
    vault: &Arc<spectral_vault::Vault>,
    attempt_id: &str,
) -> Result<RemovalEmailContext, String> {
    let db = vault.database().map_err(|e| e.to_string())?;

    // Get the removal attempt
    let attempt = spectral_db::removal_attempts::get_by_id(db.pool(), attempt_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Removal attempt not found")?;

    // Get the finding to retrieve profile_id
    let finding = spectral_db::findings::get_by_id(db.pool(), &attempt.finding_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Finding not found")?;

    // Convert broker_id and get broker definition
    let broker_id = BrokerId::new(&attempt.broker_id).map_err(|e| e.to_string())?;
    let broker = state
        .broker_registry
        .get(&broker_id)
        .map_err(|e| e.to_string())?;

    // Verify this is an email-based removal and extract email info
    let (email_address, subject_template, body_template) = match &broker.removal {
        RemovalMethod::Email {
            email,
            subject,
            body,
            ..
        } => (email.clone(), subject.clone(), body.clone()),
        _ => {
            return Err(format!(
                "Broker {} does not support email removal",
                broker.name()
            ));
        }
    };

    // Convert profile_id and load profile
    let profile_id = ProfileId::new(&finding.profile_id).map_err(|e| e.to_string())?;
    let profile = vault
        .load_profile(&profile_id)
        .await
        .map_err(|e| format!("Failed to load profile: {}", e))?;

    Ok(RemovalEmailContext {
        email_address,
        subject_template,
        body_template,
        profile,
    })
}

/// Re-trigger email send for a pending email attempt.
///
/// This command is a stub for Task 16 (Email Verification Manual Tab).
/// It will load the removal attempt, broker definition, and profile data,
/// then regenerate and send the email.
#[tauri::command]
pub async fn send_removal_email<R: tauri::Runtime>(
    state: State<'_, AppState>,
    _app: tauri::AppHandle<R>,
    vault_id: String,
    attempt_id: String,
) -> Result<(), String> {
    info!(
        "send_removal_email: vault_id={}, attempt_id={}",
        vault_id, attempt_id
    );

    // Get unlocked vault
    let vault = state.get_vault(&vault_id).ok_or("Vault not unlocked")?;

    // Get vault encryption key for profile decryption
    let vault_key = vault
        .encryption_key()
        .map_err(|e| format!("Failed to get vault key: {}", e))?;

    // Load all removal context (attempt, finding, broker, profile)
    let context = load_removal_context(&state, &vault, &attempt_id).await?;

    // Decrypt profile fields for template rendering
    let fields = decrypt_profile_fields(&context.profile, vault_key);

    // Render email subject and body templates
    let rendered_subject = render_email_template(&context.subject_template, &fields);
    let rendered_body = render_email_template(&context.body_template, &fields);

    // Construct mailto: URL
    let mailto_url = format!(
        "mailto:{}?subject={}&body={}",
        urlencoding::encode(&context.email_address),
        urlencoding::encode(&rendered_subject),
        urlencoding::encode(&rendered_body)
    );

    // Open mailto: URL in default email client
    #[allow(deprecated)]
    _app.shell()
        .open(&mailto_url, None)
        .map_err(|e| format!("Failed to open email client: {}", e))?;

    info!(
        "Opened mailto: for attempt {} to {}",
        attempt_id, context.email_address
    );

    Ok(())
}

/// Render an email template by replacing `{{field_name}}` placeholders with profile values.
fn render_email_template(
    template: &str,
    fields: &std::collections::HashMap<String, String>,
) -> String {
    let mut rendered = template.to_string();
    for (key, value) in fields {
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }
    rendered
}

#[cfg(test)]
mod score_tests {
    use super::calculate_privacy_score;

    #[test]
    fn test_score_starts_at_100() {
        let score = calculate_privacy_score(0, 0, 0, 0);
        assert_eq!(score, 100);
    }

    #[test]
    fn test_score_penalises_people_search_findings() {
        // 1 unresolved people-search finding = -8 points
        let score = calculate_privacy_score(1, 0, 0, 0);
        assert_eq!(score, 92);
    }

    #[test]
    fn test_score_clamped_to_zero() {
        let score = calculate_privacy_score(20, 0, 0, 0);
        assert_eq!(score, 0);
    }
}
