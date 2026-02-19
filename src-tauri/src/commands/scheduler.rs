//! Scheduler command handlers.

use crate::state::AppState;
use spectral_scheduler::{next_run_timestamp, ScheduledJob};
use tracing::info;

#[tauri::command]
pub async fn get_scheduled_jobs(
    vault_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ScheduledJob>, String> {
    let vault = state.get_vault(&vault_id).ok_or("Vault not unlocked")?;
    let db = vault.database().map_err(|e| e.to_string())?;

    db.get_scheduled_jobs()
        .await
        .map_err(|e| format!("Failed to get scheduled jobs: {}", e))
}

#[tauri::command]
pub async fn update_scheduled_job(
    vault_id: String,
    job_id: String,
    interval_days: u32,
    enabled: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    info!(
        "Updating job {} - interval: {}, enabled: {}",
        job_id, interval_days, enabled
    );

    let vault = state.get_vault(&vault_id).ok_or("Vault not unlocked")?;
    let db = vault.database().map_err(|e| e.to_string())?;

    // Update interval and enabled status
    let next_run = if enabled {
        next_run_timestamp(interval_days)
    } else {
        // If disabled, set next_run far in future
        next_run_timestamp(365 * 10) // 10 years
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
    .map_err(|e| format!("Failed to update job: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn run_job_now(
    vault_id: String,
    job_type: String,
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    info!("Manual job trigger: {} for vault {}", job_type, vault_id);
    // TODO: dispatch job to worker - stub for now
    Ok(())
}
