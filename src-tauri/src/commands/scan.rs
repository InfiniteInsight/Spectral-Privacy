use crate::state::AppState;
use serde::{Deserialize, Serialize};
use spectral_broker::BrokerRegistry;
use spectral_browser::BrowserEngine;
use spectral_core::types::ProfileId;
use spectral_scanner::{BrokerFilter, ScanOrchestrator};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct StartScanRequest {
    pub profile_id: String,
    pub broker_filter: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScanJobResponse {
    pub id: String,
    pub status: String,
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
    broker_filter: Option<String>,
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

    // Parse broker filter
    let filter = match broker_filter.as_deref() {
        Some("all") | None => BrokerFilter::All,
        Some(cat) => BrokerFilter::Category(cat.to_string()),
    };

    // Create orchestrator for this scan
    // TODO: These should be cached/shared across scans
    // Note: We can't clone EncryptedPool (it contains Zeroizing secrets),
    // but Pool<Sqlite> itself is Arc-based and can be cloned.
    // For now, we create a temporary EncryptedPool from the existing pool.
    // In production, the orchestrator should be a singleton in AppState.
    let broker_registry = Arc::new(BrokerRegistry::new());
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

    let orchestrator =
        ScanOrchestrator::new(broker_registry, browser_engine, db).with_max_concurrent_scans(4);

    // Start the scan
    let job_id = orchestrator
        .start_scan(&profile, filter, vault_key)
        .await
        .map_err(|e| format!("Failed to start scan: {}", e))?;

    Ok(ScanJobResponse {
        id: job_id,
        status: "InProgress".to_string(),
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

    // Query the scan job status
    let job =
        sqlx::query_as::<_, (String, String)>("SELECT id, status FROM scan_jobs WHERE id = ?")
            .bind(scan_job_id)
            .fetch_one(db.pool())
            .await
            .map_err(|e| format!("Failed to get scan status: {}", e))?;

    Ok(ScanJobResponse {
        id: job.0,
        status: job.1,
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

#[cfg(test)]
mod tests {
    // Tests will be added when we implement the actual logic
}
