//! Removal submission commands.

use crate::error::CommandError;
use crate::state::AppState;
use spectral_broker::removal::RemovalOutcome;
use spectral_db::RemovalFollowup;
use tauri::{Emitter, State};
use tracing::{info, warn};

/// Submit a removal request for a search result.
///
/// Note: This is a legacy stub command. The new workflow uses
/// `submit_removals_for_confirmed` which works with findings.
#[tauri::command]
pub async fn submit_removal(
    _state: State<'_, AppState>,
    _vault_id: String,
    broker_result_id: String,
) -> Result<RemovalOutcome, CommandError> {
    warn!(
        "submit_removal is deprecated - use submit_removals_for_confirmed instead. \
        Attempted removal for broker_result_id: {}",
        broker_result_id
    );

    // Return a stub response indicating this workflow is not yet implemented
    Err(CommandError::new(
        "NOT_IMPLEMENTED",
        "This legacy removal workflow is not yet implemented. \
        Please use the findings-based workflow with submit_removals_for_confirmed."
            .to_string(),
    ))
}

/// Mark an email removal attempt as verified.
///
/// This is a stub command that marks a removal attempt as completed after the user
/// manually verifies they received the confirmation email. Full IMAP integration
/// will be added in Task 17.
#[tauri::command]
pub async fn mark_attempt_verified(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    vault_id: String,
    attempt_id: String,
) -> Result<(), String> {
    info!("mark_attempt_verified: attempt_id={}", attempt_id);

    // Get vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not found or not unlocked".to_string())?;

    let db = vault
        .database()
        .map_err(|e| format!("Failed to access database: {e}"))?;

    // Get removal attempt to retrieve broker_id
    let removal_attempt = spectral_db::removal_attempts::get_by_id(db.pool(), &attempt_id)
        .await
        .map_err(|e| format!("Failed to get removal attempt: {e}"))?
        .ok_or_else(|| "Removal attempt not found".to_string())?;

    // Update status to Completed
    spectral_db::removal_attempts::update_status(
        db.pool(),
        &attempt_id,
        spectral_db::removal_attempts::RemovalStatus::Completed,
        None,
        Some(chrono::Utc::now()),
        None,
    )
    .await
    .map_err(|e| format!("Failed to update status: {e}"))?;

    // Emit removal:verified event
    app_handle
        .emit(
            "removal:verified",
            serde_json::json!({
                "attempt_id": attempt_id,
                "broker_id": removal_attempt.broker_id
            }),
        )
        .map_err(|e| format!("Failed to emit event: {e}"))?;

    info!("Marked attempt {} as verified", attempt_id);
    Ok(())
}

/// Follow-up reminder data returned to the frontend.
#[derive(serde::Serialize)]
pub struct FollowupDto {
    /// Follow-up row ID.
    pub id: String,
    /// ID of the associated removal attempt.
    pub attempt_id: String,
    /// Broker identifier.
    pub broker_id: String,
    /// Broker email address the follow-up targets.
    pub recipient: String,
    /// ISO-8601 timestamp when the follow-up is due.
    pub follow_up_at: String,
}

impl From<RemovalFollowup> for FollowupDto {
    fn from(f: RemovalFollowup) -> Self {
        Self {
            id: f.id,
            attempt_id: f.attempt_id,
            broker_id: f.broker_id,
            recipient: f.recipient,
            follow_up_at: f.follow_up_at,
        }
    }
}

/// Return all pending (unsent, undismissed) follow-up reminders for the vault.
#[tauri::command]
pub async fn get_pending_followups(
    vault_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FollowupDto>, CommandError> {
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault {vault_id} not unlocked"),
        )
    })?;
    let db = vault.database().map_err(|e| {
        CommandError::new("DATABASE_ERROR", format!("Failed to access database: {e}"))
    })?;

    spectral_db::get_pending_removal_followups(db.pool())
        .await
        .map(|rows| rows.into_iter().map(FollowupDto::from).collect())
        .map_err(|e| CommandError::new("DATABASE_ERROR", format!("Failed to get follow-ups: {e}")))
}

/// Dismiss a follow-up reminder (user handled it manually).
#[tauri::command]
pub async fn dismiss_followup(
    vault_id: String,
    followup_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault {vault_id} not unlocked"),
        )
    })?;
    let db = vault.database().map_err(|e| {
        CommandError::new("DATABASE_ERROR", format!("Failed to access database: {e}"))
    })?;

    spectral_db::dismiss_removal_followup(db.pool(), &followup_id)
        .await
        .map_err(|e| {
            CommandError::new(
                "DATABASE_ERROR",
                format!("Failed to dismiss follow-up: {e}"),
            )
        })
}
