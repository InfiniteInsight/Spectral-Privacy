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

/// Get findings for a scan job (stub implementation - Phase 4)
#[tauri::command]
pub async fn get_findings(
    _state: State<'_, AppState>,
    _vault_id: String,
    _scan_job_id: String,
    _filter: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    // Stub: Return empty array for now
    // Phase 4 will implement actual findings retrieval
    Ok(vec![])
}

/// Verify a finding (stub implementation - Phase 4)
#[tauri::command]
pub async fn verify_finding(
    _state: State<'_, AppState>,
    _vault_id: String,
    _finding_id: String,
    _is_match: bool,
) -> Result<(), String> {
    // Stub: No-op for now
    // Phase 4 will implement actual verification
    Ok(())
}

/// Submit removal requests for confirmed findings (stub implementation - Phase 4)
#[tauri::command]
pub async fn submit_removals_for_confirmed(
    _state: State<'_, AppState>,
    _vault_id: String,
    _scan_job_id: String,
) -> Result<Vec<String>, String> {
    // Stub: Return empty array for now
    // Phase 4 will implement actual removal submission
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    // Tests will be added when we implement the actual logic
}
