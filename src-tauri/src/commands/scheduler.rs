//! Scheduler command handlers.

use crate::error::CommandError;
use crate::state::AppState;
use spectral_scheduler::{next_run_timestamp, ScheduledJob};
use tracing::info;

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
    _state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    info!("Manual job trigger: {} for vault {}", job_type, vault_id);
    // TODO: dispatch job to worker - stub for now
    Ok(())
}
