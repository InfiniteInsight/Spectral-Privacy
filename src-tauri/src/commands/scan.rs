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

/// Helper to get vault from state.
fn get_vault(state: &AppState, vault_id: &str) -> Result<Arc<spectral_vault::Vault>, String> {
    state
        .get_vault(vault_id)
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))
}

/// Helper to get database with standard error message.
fn get_db(vault: &spectral_vault::Vault) -> Result<&spectral_db::Database, String> {
    vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {e}"))
}

/// Helper to get encryption key with standard error message.
fn get_vault_key(vault: &spectral_vault::Vault) -> Result<&[u8; 32], String> {
    vault
        .encryption_key()
        .map_err(|e| format!("Failed to get vault key: {e}"))
}

/// Scan tier for filtering brokers by priority
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ScanTier {
    /// Top ~10 brokers (`AutoScanTier1`)
    Tier1,
    /// Top ~30 brokers (`AutoScanTier1` + `AutoScanTier2`)
    Tier2,
    /// All brokers except `ManualOnly`
    All,
    /// Custom broker selection (use `broker_ids` parameter)
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
    pub current_broker_name: Option<String>,
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

#[derive(Debug, Serialize)]
pub struct PossibleMatchResponse {
    pub finding: FindingResponse,
    pub similarity_score: f64,
    pub name_similarity: f64,
    pub location_matched: bool,
    pub source_broker_id: String,
}

#[derive(Debug, Serialize)]
pub struct ZeroResultBrokerResponse {
    pub broker_id: String,
    pub possible_matches: Vec<PossibleMatchResponse>,
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
        .and_then(|a| u32::try_from(a).ok());

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
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    // Get the profile from the vault
    let profile_id = ProfileId::new(&profile_id).map_err(|e| format!("Invalid profile ID: {e}"))?;

    let profile = vault
        .load_profile(&profile_id)
        .await
        .map_err(|e| format!("Failed to load profile: {e}"))?;

    // Get the vault's encryption key
    let vault_key = get_vault_key(&vault)?;

    // Get the vault's database
    let db = get_db(&vault)?;

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
            .map_err(|e| format!("Failed to create browser engine: {e}"))?,
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
        .map_err(|e| format!("Failed to start scan: {e}"))?;

    // Log scan start to audit log
    let _ = spectral_db::audit_log::insert_audit_entry(
        db.pool(),
        vault_id.clone(),
        "ScanStarted".to_string(),
        format!(
            "Started scan job {} for profile {} with {} brokers",
            job_id,
            profile_id,
            selected_brokers.len()
        ),
        Some(vec!["name".to_string(), "address".to_string()]),
        "LocalOnly".to_string(),
        "Allowed".to_string(),
    )
    .await;

    // Log scan start to audit log
    let _ = spectral_db::audit_log::insert_audit_entry(
        db.pool(),
        vault_id.clone(),
        "ScanStarted".to_string(),
        format!(
            "Started scan job {} for profile {} with {} brokers",
            job_id,
            profile_id,
            selected_brokers.len()
        ),
        Some(vec!["name".to_string(), "address".to_string()]),
        "LocalOnly".to_string(),
        "Allowed".to_string(),
    )
    .await;

    // Query the job to get complete information including total_brokers
    let job = sqlx::query_as::<_, (String, String, i64, i64, Option<String>)>(
        "SELECT id, status, completed_brokers, total_brokers, current_broker_name FROM scan_jobs WHERE id = ?",
    )
    .bind(&job_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Failed to get scan status: {e}"))?;

    Ok(ScanJobResponse {
        id: job.0,
        status: job.1,
        completed_brokers: job.2.clamp(0, i64::from(u32::MAX)) as u32,
        total_brokers: job.3.clamp(0, i64::from(u32::MAX)) as u32,
        current_broker_name: job.4,
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
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    // Get the vault's database
    let db = get_db(&vault)?;

    // Query with all required fields for progress tracking
    let job = sqlx::query_as::<_, (String, String, i64, i64, Option<String>)>(
        "SELECT id, status, completed_brokers, total_brokers, current_broker_name FROM scan_jobs WHERE id = ?",
    )
    .bind(scan_job_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Failed to get scan status: {e}"))?;

    Ok(ScanJobResponse {
        id: job.0,
        status: job.1,
        completed_brokers: job.2.clamp(0, i64::from(u32::MAX)) as u32,
        total_brokers: job.3.clamp(0, i64::from(u32::MAX)) as u32,
        current_broker_name: job.4,
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
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    // Get the vault's database
    let db = get_db(&vault)?;

    // Get all findings for this scan job
    let mut findings = spectral_db::findings::get_by_scan_job(db.pool(), &scan_job_id)
        .await
        .map_err(|e| format!("Failed to get findings: {e}"))?;

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
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    // Get the vault's database
    let db = get_db(&vault)?;

    // Update verification status
    spectral_db::findings::verify_finding(
        db.pool(),
        &finding_id,
        is_match,
        true, // verified_by_user = true
    )
    .await
    .map_err(|e| format!("Failed to verify finding: {e}"))?;

    // If confirmed as a match, generate Google removal URL
    if is_match {
        // Get the finding to extract data
        let finding = spectral_db::findings::get_by_id(db.pool(), &finding_id)
            .await
            .map_err(|e| format!("Failed to get finding: {e}"))?;

        if let Some(finding) = finding {
            // Extract name, address, and phone from finding
            let name = finding
                .extracted_data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");

            let address = finding
                .extracted_data
                .get("addresses")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str());

            let phone = finding
                .extracted_data
                .get("phone_numbers")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str());

            // Generate Google removal URL
            let google_url =
                spectral_db::google_removal::generate_removal_url(name, address, phone);

            // Create Google removal request (idempotent - will return existing if already created)
            let _ = spectral_db::google_removal::create_request(
                db.pool(),
                finding_id.clone(),
                google_url,
            )
            .await
            .map_err(|e| format!("Failed to create Google removal request: {e}"))?;

            // Log to audit log
            let _ = spectral_db::audit_log::insert_audit_entry(
                db.pool(),
                vault_id.clone(),
                "GoogleRemovalURLGenerated".to_string(),
                format!("Generated Google removal URL for finding {finding_id}"),
                None,
                "LocalOnly".to_string(),
                "Allowed".to_string(),
            )
            .await;
        }
    }

    // Log finding verification to audit log
    let _ = spectral_db::audit_log::insert_audit_entry(
        db.pool(),
        vault_id.clone(),
        "FindingVerified".to_string(),
        format!(
            "User {} finding {}",
            if is_match { "confirmed" } else { "rejected" },
            finding_id
        ),
        None,
        "LocalOnly".to_string(),
        "Allowed".to_string(),
    )
    .await;

    // If confirmed as a match, generate Google removal URL
    if is_match {
        // Get the finding to extract data
        let finding = spectral_db::findings::get_by_id(db.pool(), &finding_id)
            .await
            .map_err(|e| format!("Failed to get finding: {}", e))?;

        if let Some(finding) = finding {
            // Extract name, address, and phone from finding
            let name = finding
                .extracted_data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");

            let address = finding
                .extracted_data
                .get("addresses")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str());

            let phone = finding
                .extracted_data
                .get("phone_numbers")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str());

            // Generate Google removal URL
            let google_url =
                spectral_db::google_removal::generate_removal_url(name, address, phone);

            // Create Google removal request
            let _ = spectral_db::google_removal::create_request(
                db.pool(),
                finding_id.clone(),
                google_url,
            )
            .await
            .map_err(|e| format!("Failed to create Google removal request: {}", e))?;

            // Log to audit log
            let _ = spectral_db::audit_log::insert_audit_entry(
                db.pool(),
                vault_id.clone(),
                "GoogleRemovalURLGenerated".to_string(),
                format!("Generated Google removal URL for finding {}", finding_id),
                None,
                "LocalOnly".to_string(),
                "Allowed".to_string(),
            )
            .await;
        }
    }

    // Log finding verification to audit log
    let _ = spectral_db::audit_log::insert_audit_entry(
        db.pool(),
        vault_id.clone(),
        "FindingVerified".to_string(),
        format!(
            "User {} finding {}",
            if is_match { "confirmed" } else { "rejected" },
            finding_id
        ),
        None,
        "LocalOnly".to_string(),
        "Allowed".to_string(),
    )
    .await;

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
    let db = get_db(&vault)?;

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
            finding.profile_id,
        )
        .await
        .map_err(|e| e.to_string())?;

        removal_ids.push(removal_attempt.id);
    }

    Ok(removal_ids)
}

/// Process a batch of removal attempts with parallel workers.
///
/// Spawns async worker tasks for each `removal_attempt_id` (max 3 concurrent).
/// Returns immediately with a `job_id`. Real-time events are emitted as tasks complete.
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
    let db = get_db(&vault)?;

    // Get the underlying Pool<Sqlite> which can be cloned
    let pool = db.pool().clone();
    let vault_key = get_vault_key(&vault)?;
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
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    // Get database
    let db = get_db(&vault)?;

    // Get CAPTCHA queue
    spectral_db::removal_attempts::get_captcha_queue(db.pool())
        .await
        .map_err(|e| format!("Failed to get CAPTCHA queue: {e}"))
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
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    // Get database
    let db = get_db(&vault)?;

    // Get failed queue
    spectral_db::removal_attempts::get_failed_queue(db.pool())
        .await
        .map_err(|e| format!("Failed to get failed queue: {e}"))
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
        .map_err(|e| format!("Failed to access database: {e}"))?;

    spectral_db::removal_attempts::get_by_scan_job_id(db.pool(), &scan_job_id)
        .await
        .map_err(|e| format!("Failed to query removal attempts: {e}"))
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
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    // Get database
    let db = get_db(&vault)?;

    // Get job history
    spectral_db::removal_attempts::get_job_history(db.pool())
        .await
        .map_err(|e| format!("Failed to get job history: {e}"))
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
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    // Get database
    let db = get_db(&vault)?;

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
    .map_err(|e| format!("Failed to reset removal attempt: {e}"))?;

    // Get the underlying Pool<Sqlite> which can be cloned
    let pool = db.pool().clone();
    let vault_key = get_vault_key(&vault)?;
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
/// All queries are pool-scoped; no `vault_id` WHERE clause is needed.
#[tauri::command]
pub async fn get_dashboard_summary(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<DashboardSummary, String> {
    info!("get_dashboard_summary: vault_id={}", vault_id);
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;
    let db = get_db(&vault)?;
    let pool = db.pool();

    // Count distinct brokers with at least one finding.
    let brokers_scanned: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT broker_id) FROM findings")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Failed to count brokers scanned: {e}"))?;

    // Timestamp of the most recently started scan job.
    let last_scan_at: Option<String> = sqlx::query_scalar("SELECT MAX(started_at) FROM scan_jobs")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Failed to get last scan timestamp: {e}"))?;

    // Removal counts by status.
    let submitted: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM removal_attempts WHERE status = 'Submitted'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count submitted removals: {e}"))?;

    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM removal_attempts WHERE status = 'Pending'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count pending removals: {e}"))?;

    let failed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM removal_attempts WHERE status = 'Failed'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count failed removals: {e}"))?;

    // Unresolved = confirmed findings with no removal yet.
    let unresolved: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM findings WHERE verification_status = 'Confirmed'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count confirmed findings: {e}"))?;

    // Always calculate privacy score for consistency with score page
    let privacy_score = Some(calculate_privacy_score(
        unresolved.clamp(0, i64::from(u32::MAX)) as u32,
        submitted.clamp(0, i64::from(u32::MAX)) as u32,
        failed.clamp(0, i64::from(u32::MAX)) as u32,
        0,
    ));

    // Last 5 scan jobs as activity events.
    let scan_rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, started_at, status FROM scan_jobs ORDER BY started_at DESC LIMIT 5",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch recent scan jobs: {e}"))?;

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
    .map_err(|e| format!("Failed to fetch recent removal attempts: {e}"))?;

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
/// - Unresolved findings (`verification_status` = 'Confirmed' but not yet removed)
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
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;
    let db = get_db(&vault)?;
    let pool = db.pool();

    // Count all confirmed findings. The penalty applies to all Confirmed findings
    // until the listing is verified removed (a future feature).
    // verification_status = 'Confirmed' means the user has verified this is them.
    let unresolved: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM findings WHERE verification_status = 'Confirmed'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count unresolved findings: {e}"))?;

    // Count submitted removal attempts via JOIN (removal_attempts has no vault_id).
    let confirmed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM removal_attempts WHERE status = 'Submitted'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count submitted removals: {e}"))?;

    // Count failed removal attempts.
    let failed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM removal_attempts WHERE status = 'Failed'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to count failed removals: {e}"))?;

    let score = calculate_privacy_score(
        unresolved.clamp(0, i64::from(u32::MAX)) as u32,
        confirmed.clamp(0, i64::from(u32::MAX)) as u32,
        failed.clamp(0, i64::from(u32::MAX)) as u32,
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
    let vault = get_vault(&state, &vault_id)?;
    let db = get_db(&vault)?;

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

/// Decrypt all profile fields into a `HashMap` for template rendering.
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
    let db = get_db(vault)?;

    // Get the removal attempt
    let attempt = spectral_db::removal_attempts::get_by_id(db.pool(), attempt_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Removal attempt not found")?;

    // Get the finding to retrieve profile_id (may be None for standalone removals)
    let finding_id = attempt
        .finding_id
        .as_deref()
        .ok_or("Removal attempt has no associated finding")?;
    let finding = spectral_db::findings::get_by_id(db.pool(), finding_id)
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
        .map_err(|e| format!("Failed to load profile: {e}"))?;

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
    let vault_key = get_vault_key(&vault)?;

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
        .map_err(|e| format!("Failed to open email client: {e}"))?;

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

/// Get possible matches for zero-result brokers.
#[tauri::command]
pub async fn get_possible_matches(
    state: State<'_, AppState>,
    vault_id: String,
    scan_job_id: String,
) -> Result<Vec<ZeroResultBrokerResponse>, String> {
    use crate::matching_service;

    let vault = get_vault(&state, &vault_id)?;
    let db = get_db(&vault)?;
    let vault_key = get_vault_key(&vault)?;

    // Get profile from scan job
    let profile = {
        let profile_id_str: String = // nosemgrep: use-zeroize-for-secrets
            sqlx::query_scalar("SELECT profile_id FROM scan_jobs WHERE id = ?")
                .bind(&scan_job_id)
                .fetch_one(db.pool())
                .await
                .map_err(|e| format!("Failed to get scan job: {e}"))?;

        let profile_id = spectral_core::types::ProfileId::new(&profile_id_str)
            .map_err(|e| format!("Invalid profile ID: {e}"))?;

        vault
            .load_profile(&profile_id)
            .await
            .map_err(|e| format!("Failed to load profile: {e}"))?
    };

    let matches =
        matching_service::find_possible_matches(db, &vault, &scan_job_id, &profile, vault_key)
            .await?;

    let mut response = Vec::new();
    for (broker_id, possible_matches) in matches {
        let matches_response: Vec<PossibleMatchResponse> = possible_matches
            .into_iter()
            .map(|m| PossibleMatchResponse {
                finding: finding_to_response(m.finding),
                similarity_score: m.similarity_score,
                name_similarity: m.name_similarity,
                location_matched: m.location_matched,
                source_broker_id: m.source_broker_id,
            })
            .collect();

        response.push(ZeroResultBrokerResponse {
            broker_id,
            possible_matches: matches_response,
        });
    }

    response.sort_by(|a, b| a.broker_id.cmp(&b.broker_id));
    Ok(response)
}

/// Accept a possible match - create finding for zero-result broker.
#[tauri::command]
pub async fn accept_possible_match(
    state: State<'_, AppState>,
    vault_id: String,
    scan_job_id: String,
    zero_result_broker_id: String,
    matched_finding_id: String,
) -> Result<FindingResponse, String> {
    let vault = get_vault(&state, &vault_id)?;
    let db = get_db(&vault)?;

    let original = spectral_db::findings::get_by_id(db.pool(), &matched_finding_id)
        .await
        .map_err(|e| format!("Failed to get finding: {e}"))?
        .ok_or_else(|| "Finding not found".to_string())?;

    // nosemgrep: use-zeroize-for-secrets
    let broker_scan_id: String = sqlx::query_scalar(
        "SELECT id FROM broker_scans
         WHERE scan_job_id = ? AND broker_id = ?
         LIMIT 1",
    )
    .bind(&scan_job_id)
    .bind(&zero_result_broker_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Failed to get broker scan: {e}"))?;

    let new_finding = spectral_db::findings::create_finding(
        db.pool(),
        broker_scan_id.clone(),
        zero_result_broker_id.clone(),
        original.profile_id,
        original.listing_url,
        original.extracted_data.clone(),
    )
    .await
    .map_err(|e| format!("Failed to create finding: {e}"))?;

    sqlx::query(
        "UPDATE broker_scans
         SET findings_count = findings_count + 1
         WHERE id = ?",
    )
    .bind(&broker_scan_id)
    .execute(db.pool())
    .await
    .map_err(|e| format!("Failed to update broker scan: {e}"))?;

    Ok(finding_to_response(new_finding))
}

/// Dismiss a possible match (client-side only).
#[tauri::command]
pub async fn dismiss_possible_match(
    _state: State<'_, AppState>,
    _vault_id: String,
    _zero_result_broker_id: String,
    _matched_finding_id: String,
) -> Result<(), String> {
    Ok(())
}

/// Response format for Google removal requests.
#[derive(Debug, Serialize)]
pub struct GoogleRemovalRequestResponse {
    pub id: String,
    pub finding_id: String,
    pub status: String,
    pub google_removal_url: String,
    pub generated_at: String,
    pub submitted_at: Option<String>,
    pub completed_at: Option<String>,
}

/// Get Google removal request for a finding.
#[tauri::command]
pub async fn get_google_removal_request(
    state: State<'_, AppState>,
    vault_id: String,
    finding_id: String,
) -> Result<Option<GoogleRemovalRequestResponse>, String> {
    let vault = get_vault(&state, &vault_id)?;
    let db = get_db(&vault)?;

    let request = spectral_db::google_removal::get_by_finding_id(db.pool(), &finding_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(request.map(|r| GoogleRemovalRequestResponse {
        id: r.id,
        finding_id: r.finding_id,
        status: format!("{:?}", r.status),
        google_removal_url: r.google_removal_url,
        generated_at: r.generated_at.to_rfc3339(),
        submitted_at: r.submitted_at.map(|t| t.to_rfc3339()),
        completed_at: r.completed_at.map(|t| t.to_rfc3339()),
    }))
}

/// Mark Google removal as submitted by user.
#[tauri::command]
pub async fn mark_google_removal_submitted(
    state: State<'_, AppState>,
    vault_id: String,
    request_id: String,
    notes: Option<String>,
) -> Result<(), String> {
    let vault = get_vault(&state, &vault_id)?;
    let db = get_db(&vault)?;

    spectral_db::google_removal::update_status(
        db.pool(),
        &request_id,
        spectral_db::google_removal::GoogleRemovalStatus::Submitted,
        notes,
    )
    .await
    .map_err(|e| e.to_string())?;

    // Audit log
    let _ = spectral_db::audit_log::insert_audit_entry(
        db.pool(),
        vault_id,
        "GoogleRemovalSubmitted".to_string(),
        format!("User marked Google removal request {request_id} as submitted"),
        None,
        "ExternalSite:google.com".to_string(),
        "Allowed".to_string(),
    )
    .await;

    Ok(())
}

/// Get scan job history for a profile with statistics.
#[tauri::command]
pub async fn get_scan_job_history(
    state: State<'_, AppState>,
    vault_id: String,
    profile_id: String,
) -> Result<Vec<spectral_db::scan_jobs::ScanJobHistory>, String> {
    let vault = get_vault(&state, &vault_id)?;
    let db = get_db(&vault)?;

    spectral_db::scan_jobs::get_scan_job_history(db.pool(), &profile_id)
        .await
        .map_err(|e| format!("Failed to get scan job history: {e}"))
}

/// Get unified scan history for all scan types.
#[tauri::command]
pub async fn get_unified_scan_history(
    state: State<'_, AppState>,
    vault_id: String,
    profile_id: Option<String>,
) -> Result<Vec<spectral_db::scan_jobs::ScanHistoryEntry>, String> {
    let vault = get_vault(&state, &vault_id)?;
    let db = get_db(&vault)?;

    spectral_db::scan_jobs::get_unified_scan_history(db.pool(), &vault_id, profile_id.as_deref())
        .await
        .map_err(|e| format!("Failed to get unified scan history: {e}"))
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

/// Per-broker result returned for scan debugging.
#[derive(Debug, Serialize)]
pub struct BrokerScanResult {
    pub broker_id: String,
    pub status: String,
    pub findings_count: i64,
    pub error_message: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

/// Return the per-broker scan records for a completed scan job.
///
/// Useful for debugging: shows whether each broker scan succeeded or failed,
/// how many findings were found, and any error details.
#[tauri::command]
pub async fn get_broker_scan_results(
    state: State<'_, AppState>,
    vault_id: String,
    job_id: String,
) -> Result<Vec<BrokerScanResult>, String> {
    let vault = get_vault(&state, &vault_id)?;
    let db = get_db(&vault)?;

    let scans = spectral_db::broker_scans::get_by_scan_job(db.pool(), &job_id)
        .await
        .map_err(|e| format!("Failed to fetch broker scan results: {e}"))?;

    Ok(scans
        .into_iter()
        .map(|s| BrokerScanResult {
            broker_id: s.broker_id,
            status: s.status,
            findings_count: s.findings_count,
            error_message: s.error_message,
            started_at: s.started_at,
            completed_at: s.completed_at,
        })
        .collect())
}

/// Initiate a removal attempt for a single broker that cannot be auto-scanned.
///
/// Creates a standalone removal attempt (not linked to a scan finding),
/// then processes it immediately via the removal worker.
/// Use for brokers with email, web-form, or manual search methods.
///
/// Returns the removal attempt ID.
#[tauri::command]
pub async fn initiate_direct_removal<R: tauri::Runtime>(
    vault_id: String,
    broker_id: String,
    profile_id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    let vault = get_vault(&state, &vault_id)?;
    let db = get_db(&vault)?;

    // Create standalone removal attempt (no finding_id)
    let attempt = spectral_db::removal_attempts::create_standalone_removal_attempt(
        db.pool(),
        &broker_id,
        &profile_id,
    )
    .await
    .map_err(|e| format!("Failed to create removal attempt: {e}"))?;

    let attempt_id = attempt.id.clone();

    // Build cloneable Arc<Database> for the worker task
    use spectral_db::{Database, EncryptedPool};
    let pool = db.pool().clone();
    let vault_key = get_vault_key(&vault)?;
    let encrypted_pool = EncryptedPool::from_pool(pool, vault_key.to_vec());
    let db_arc = Arc::new(Database::from_encrypted_pool(encrypted_pool));

    let vault_arc = Arc::clone(&vault);
    let broker_registry = state.broker_registry.clone();
    let semaphore = Arc::new(Semaphore::new(1));
    let browser_engine = state.browser_engine.clone();

    let attempt_id_clone = attempt_id.clone();
    let app_clone = app.clone();

    tokio::spawn(async move {
        let _ = app_clone.emit(
            "removal:started",
            serde_json::json!({ "attempt_id": attempt_id_clone }),
        );

        let result = submit_removal_task(
            db_arc,
            vault_arc,
            attempt_id_clone.clone(),
            broker_registry,
            semaphore,
            browser_engine,
        )
        .await;

        match result {
            Ok(worker_result) => match worker_result.outcome {
                spectral_broker::removal::RemovalOutcome::Submitted
                | spectral_broker::removal::RemovalOutcome::RequiresEmailVerification { .. } => {
                    let _ = app_clone.emit(
                        "removal:success",
                        serde_json::json!({
                            "attempt_id": attempt_id_clone,
                            "outcome": format!("{:?}", worker_result.outcome)
                        }),
                    );
                }
                spectral_broker::removal::RemovalOutcome::RequiresCaptcha { .. } => {
                    let _ = app_clone.emit(
                        "removal:captcha",
                        serde_json::json!({ "attempt_id": attempt_id_clone }),
                    );
                }
                spectral_broker::removal::RemovalOutcome::Failed { .. }
                | spectral_broker::removal::RemovalOutcome::RequiresAccountCreation => {
                    let _ = app_clone.emit(
                        "removal:failed",
                        serde_json::json!({
                            "attempt_id": attempt_id_clone,
                            "error": format!("{:?}", worker_result.outcome)
                        }),
                    );
                }
            },
            Err(error) => {
                let _ = app_clone.emit(
                    "removal:failed",
                    serde_json::json!({ "attempt_id": attempt_id_clone, "error": error }),
                );
            }
        }
    });

    Ok(attempt_id)
}

/// Initiate bulk removal for all non-scannable brokers in a given list.
///
/// For each broker ID provided that uses a non-`UrlTemplate` search method,
/// creates a standalone removal attempt and processes it via the removal worker.
/// Brokers with `UrlTemplate` search (auto-scannable) are skipped.
///
/// Returns the list of created removal attempt IDs.
#[tauri::command]
pub async fn initiate_bulk_removal<R: tauri::Runtime>(
    vault_id: String,
    profile_id: String,
    broker_ids: Vec<String>,
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<Vec<String>, String> {
    use spectral_broker::definition::SearchMethod;

    let vault = get_vault(&state, &vault_id)?;
    let db = get_db(&vault)?;

    let mut attempt_ids: Vec<String> = Vec::new();

    // Create standalone removal attempts for all non-scannable brokers
    for broker_id_str in &broker_ids {
        let broker_def = match state.get_broker_definition(broker_id_str) {
            Some(def) => def,
            None => {
                tracing::warn!("Skipping unknown broker {broker_id_str} in bulk removal");
                continue;
            }
        };

        // Skip scannable brokers — they need a scan first
        if matches!(broker_def.search, SearchMethod::UrlTemplate { .. }) {
            continue;
        }

        match spectral_db::removal_attempts::create_standalone_removal_attempt(
            db.pool(),
            broker_id_str,
            &profile_id,
        )
        .await
        {
            Ok(attempt) => attempt_ids.push(attempt.id),
            Err(e) => {
                tracing::error!("Failed to create removal attempt for {broker_id_str}: {e}");
            }
        }
    }

    if attempt_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Build cloneable Arc<Database> for worker tasks
    use spectral_db::{Database, EncryptedPool};
    let pool = db.pool().clone();
    let vault_key = get_vault_key(&vault)?;
    let encrypted_pool = EncryptedPool::from_pool(pool, vault_key.to_vec());
    let db_arc = Arc::new(Database::from_encrypted_pool(encrypted_pool));

    let semaphore = Arc::new(Semaphore::new(3));
    let browser_engine = state.browser_engine.clone();
    let broker_registry = state.broker_registry.clone();

    // Emit started events and spawn worker tasks
    for attempt_id in &attempt_ids {
        let _ = app.emit(
            "removal:started",
            serde_json::json!({ "attempt_id": attempt_id }),
        );

        let db_clone = Arc::clone(&db_arc);
        let vault_clone = Arc::clone(&vault);
        let registry_clone = broker_registry.clone();
        let sem_clone = Arc::clone(&semaphore);
        let browser_clone = browser_engine.clone();
        let app_clone = app.clone();
        let attempt_id_clone = attempt_id.clone();

        tokio::spawn(async move {
            let result = submit_removal_task(
                db_clone,
                vault_clone,
                attempt_id_clone.clone(),
                registry_clone,
                sem_clone,
                browser_clone,
            )
            .await;

            match result {
                Ok(worker_result) => match worker_result.outcome {
                    spectral_broker::removal::RemovalOutcome::Submitted
                    | spectral_broker::removal::RemovalOutcome::RequiresEmailVerification {
                        ..
                    } => {
                        let _ = app_clone.emit(
                            "removal:success",
                            serde_json::json!({ "attempt_id": attempt_id_clone }),
                        );
                    }
                    spectral_broker::removal::RemovalOutcome::RequiresCaptcha { .. } => {
                        let _ = app_clone.emit(
                            "removal:captcha",
                            serde_json::json!({ "attempt_id": attempt_id_clone }),
                        );
                    }
                    spectral_broker::removal::RemovalOutcome::Failed { .. }
                    | spectral_broker::removal::RemovalOutcome::RequiresAccountCreation => {
                        let _ = app_clone.emit(
                            "removal:failed",
                            serde_json::json!({
                                "attempt_id": attempt_id_clone,
                                "error": format!("{:?}", worker_result.outcome)
                            }),
                        );
                    }
                },
                Err(error) => {
                    let _ = app_clone.emit(
                        "removal:failed",
                        serde_json::json!({ "attempt_id": attempt_id_clone, "error": error }),
                    );
                }
            }
        });
    }

    Ok(attempt_ids)
}
