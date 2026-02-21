//! Scheduler command handlers.

use crate::error::CommandError;
use crate::state::AppState;
use spectral_browser::BrowserEngine;
use spectral_db::{Database, EncryptedPool};
use spectral_scanner::{BrokerFilter, ScanOrchestrator};
use spectral_scheduler::{next_run_timestamp, JobType, ScheduledJob};
use std::sync::Arc;
use tracing::{error, info};

/// Interval for disabled jobs (far future to prevent execution)
const DISABLED_JOB_INTERVAL_DAYS: u32 = 365 * 10; // 10 years

#[tauri::command]
pub async fn get_scheduled_jobs(
    vault_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ScheduledJob>, CommandError> {
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault {} not unlocked", vault_id),
        )
    })?;
    let db = vault.database().map_err(|e| {
        CommandError::new(
            "DATABASE_ERROR",
            format!("Failed to access database: {}", e),
        )
    })?;

    db.get_scheduled_jobs().await.map_err(|e| {
        CommandError::new(
            "DATABASE_ERROR",
            format!("Failed to get scheduled jobs: {}", e),
        )
    })
}

#[tauri::command]
pub async fn update_scheduled_job(
    vault_id: String,
    job_id: String,
    interval_days: u32,
    enabled: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    info!(
        "Updating job {} - interval: {}, enabled: {}",
        job_id, interval_days, enabled
    );

    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault {} not unlocked", vault_id),
        )
    })?;
    let db = vault.database().map_err(|e| {
        CommandError::new(
            "DATABASE_ERROR",
            format!("Failed to access database: {}", e),
        )
    })?;

    // Update interval and enabled status
    let next_run = if enabled {
        next_run_timestamp(interval_days)
    } else {
        // If disabled, set next_run far in future
        next_run_timestamp(DISABLED_JOB_INTERVAL_DAYS)
    };

    sqlx::query(
        "UPDATE scheduled_jobs SET interval_days = ?, enabled = ?, next_run_at = ? WHERE id = ?",
    )
    .bind(interval_days as i64)
    .bind(if enabled { 1 } else { 0 })
    .bind(&next_run)
    .bind(&job_id)
    .execute(db.pool())
    .await
    .map_err(|e| CommandError::new("DATABASE_ERROR", format!("Failed to update job: {}", e)))?;

    Ok(())
}

#[tauri::command]
pub async fn run_job_now(
    vault_id: String,
    job_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    info!("Manual job trigger: {} for vault {}", job_type, vault_id);

    // Parse job type
    let job_type: JobType = serde_json::from_value(serde_json::Value::String(job_type.clone()))
        .map_err(|e| {
            CommandError::new(
                "INVALID_JOB_TYPE",
                format!("Invalid job type '{}': {}", job_type, e),
            )
        })?;

    // Get the unlocked vault
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault {} not unlocked", vault_id),
        )
    })?;

    // Get the vault's database
    let db = vault.database().map_err(|e| {
        CommandError::new(
            "DATABASE_ERROR",
            format!("Failed to get vault database: {}", e),
        )
    })?;

    // Get the vault's encryption key
    let vault_key = vault
        .encryption_key()
        .map_err(|e| CommandError::new("VAULT_ERROR", format!("Failed to get vault key: {}", e)))?;

    match job_type {
        JobType::ScanAll => {
            info!("Executing ScanAll job for vault {}", vault_id);

            // Get all profiles in the vault
            let profile_ids = vault.list_profiles().await.map_err(|e| {
                CommandError::new("DATABASE_ERROR", format!("Failed to list profiles: {}", e))
            })?;

            if profile_ids.is_empty() {
                return Err(CommandError::new(
                    "NO_PROFILES",
                    "No profiles found in vault. Create a profile first.".to_string(),
                ));
            }

            // Use the first profile for scheduled scans
            let profile_id = &profile_ids[0];
            info!("Using profile {} for scheduled scan", profile_id);

            // Load the profile data
            let profile = vault.load_profile(profile_id).await.map_err(|e| {
                CommandError::new("DATABASE_ERROR", format!("Failed to load profile: {}", e))
            })?;

            // Create browser engine
            let browser_engine = Arc::new(BrowserEngine::new().await.map_err(|e| {
                CommandError::new(
                    "BROWSER_ERROR",
                    format!("Failed to create browser engine: {}", e),
                )
            })?);

            // Create orchestrator
            let pool = db.pool().clone();
            let vault_key_vec = vault_key.to_vec();
            let encrypted_pool = EncryptedPool::from_pool(pool, vault_key_vec);
            let database = Database::from_encrypted_pool(encrypted_pool);
            let db_arc = Arc::new(database);

            let orchestrator =
                ScanOrchestrator::new(state.broker_registry.clone(), browser_engine, db_arc)
                    .with_max_concurrent_scans(4);

            // Scan all brokers except ManualOnly
            let filter = BrokerFilter::All;

            info!("Starting scheduled scan with all auto-scan brokers");

            // Start the scan
            let _job_id = orchestrator
                .start_scan(&profile, filter, vault_key)
                .await
                .map_err(|e| {
                    error!("Scheduled scan failed: {}", e);
                    CommandError::new("SCAN_ERROR", format!("Scan failed: {}", e))
                })?;

            info!("Scheduled scan started successfully");
            Ok(())
        }
        JobType::VerifyRemovals => {
            // Not yet implemented - requires re-scanning logic
            Err(CommandError::new(
                "NOT_IMPLEMENTED",
                "VerifyRemovals job type not yet implemented. This feature requires re-scanning brokers with submitted/completed removal attempts to verify removal success.".to_string(),
            ))
        }
        JobType::PollImap => {
            // Not yet implemented - requires IMAP poller (Feature 8)
            Err(CommandError::new(
                "NOT_IMPLEMENTED",
                "PollImap job type not yet implemented. This feature requires IMAP configuration and the email verification monitoring system (Feature 8).".to_string(),
            ))
        }
    }
}
