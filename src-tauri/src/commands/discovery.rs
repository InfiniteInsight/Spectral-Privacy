//! Discovery commands for local PII scanning

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use spectral_discovery::PiiPatterns;
use tauri::{Emitter, State};
use tracing::{error, info};

/// Discovery finding response
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryFinding {
    pub id: String,
    pub source: String,
    pub source_detail: String,
    pub finding_type: String,
    pub risk_level: String,
    pub description: String,
    pub recommended_action: Option<String>,
    pub remediated: bool,
    pub found_at: String,
}

/// Start a discovery scan of local files
///
/// Scans common user directories (Documents, Downloads, Desktop) for PII
/// and stores findings in the database. Runs in background and emits
/// `discovery:complete` event when done.
#[tauri::command]
pub async fn start_discovery_scan<R: tauri::Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
    vault_id: String,
) -> Result<String, String> {
    info!("start_discovery_scan: vault_id={}", vault_id);

    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Clone the pool for background task
    let pool = db.pool().clone();
    let vault_id_clone = vault_id.clone();

    // Spawn background scan task
    tokio::spawn(async move {
        info!("Starting filesystem scan for vault {}", vault_id_clone);
        let patterns = PiiPatterns::new();

        // Get user home directory
        let home_dir = match directories::UserDirs::new() {
            Some(dirs) => dirs.home_dir().to_path_buf(),
            None => {
                error!("Failed to get user home directory");
                let _ = app.emit(
                    "discovery:error",
                    serde_json::json!({
                        "error": "Failed to get user home directory"
                    }),
                );
                return;
            }
        };

        // Directories to scan
        let scan_dirs = vec![
            home_dir.join("Documents"),
            home_dir.join("Downloads"),
            home_dir.join("Desktop"),
        ];

        let mut total_findings = 0;

        for dir in scan_dirs {
            if !dir.exists() {
                continue;
            }

            info!("Scanning directory: {:?}", dir);
            let results = spectral_discovery::scan_directory(&dir, &patterns).await;

            for result in results {
                for pii_match in result.matches {
                    // Insert finding into database
                    let source = "filesystem".to_string();
                    let source_detail = result.path.to_string_lossy().to_string();
                    let finding_type = "pii_exposure".to_string();
                    let risk_level = pii_match.risk_level().to_string();
                    let file_name = match result.path.file_name() {
                        Some(name) => name.to_string_lossy().to_string(),
                        None => {
                            tracing::warn!(
                                "Could not extract filename from path: {:?}",
                                result.path
                            );
                            result.path.to_string_lossy().to_string() // Use full path as fallback
                        }
                    };

                    let description =
                        format!("{} found in file: {}", pii_match.description(), file_name);
                    let recommended_action = Some(
                        "Review file and remove sensitive information if no longer needed"
                            .to_string(),
                    );

                    match spectral_db::discovery_findings::insert_discovery_finding(
                        &pool,
                        spectral_db::discovery_findings::CreateDiscoveryFinding {
                            vault_id: vault_id_clone.clone(),
                            source,
                            source_detail,
                            finding_type,
                            risk_level,
                            description,
                            recommended_action,
                        },
                    )
                    .await
                    {
                        Ok(_) => {
                            total_findings += 1;
                        }
                        Err(e) => {
                            error!("Failed to insert discovery finding: {}", e);
                        }
                    }
                }
            }
        }

        info!("Discovery scan complete: {} findings", total_findings);

        // Emit completion event
        let _ = app.emit(
            "discovery:complete",
            serde_json::json!({
                "vault_id": vault_id_clone,
                "findings_count": total_findings
            }),
        );
    });

    Ok("Scan started".to_string())
}

/// Get all discovery findings for a vault
#[tauri::command]
pub async fn get_discovery_findings(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<DiscoveryFinding>, String> {
    info!("get_discovery_findings: vault_id={}", vault_id);

    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Query findings
    let findings = spectral_db::discovery_findings::get_discovery_findings(db.pool(), &vault_id)
        .await
        .map_err(|e| format!("Failed to get discovery findings: {}", e))?;

    // Convert to response format
    let response: Vec<DiscoveryFinding> = findings
        .into_iter()
        .map(|f| DiscoveryFinding {
            id: f.id,
            source: f.source,
            source_detail: f.source_detail,
            finding_type: f.finding_type,
            risk_level: f.risk_level,
            description: f.description,
            recommended_action: f.recommended_action,
            remediated: f.remediated,
            found_at: f.found_at,
        })
        .collect();

    Ok(response)
}

/// Mark a finding as remediated
#[tauri::command]
pub async fn mark_finding_remediated(
    state: State<'_, AppState>,
    vault_id: String,
    finding_id: String,
) -> Result<(), String> {
    info!(
        "mark_finding_remediated: vault_id={}, finding_id={}",
        vault_id, finding_id
    );

    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{}' is not unlocked", vault_id))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {}", e))?;

    // Update finding
    spectral_db::discovery_findings::update_finding_remediated(db.pool(), &finding_id, true)
        .await
        .map_err(|e| format!("Failed to mark finding as remediated: {}", e))?;

    Ok(())
}
