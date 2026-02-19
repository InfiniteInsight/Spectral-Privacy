//! Removal submission commands.

use crate::error::CommandError;
use crate::state::AppState;
use spectral_broker::removal::RemovalOutcome;
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
        .map_err(|e| format!("Failed to access database: {}", e))?;

    // Get removal attempt to retrieve broker_id
    let removal_attempt = spectral_db::removal_attempts::get_by_id(db.pool(), &attempt_id)
        .await
        .map_err(|e| format!("Failed to get removal attempt: {}", e))?
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
    .map_err(|e| format!("Failed to update status: {}", e))?;

    // Emit removal:verified event
    app_handle
        .emit(
            "removal:verified",
            serde_json::json!({
                "attempt_id": attempt_id,
                "broker_id": removal_attempt.broker_id
            }),
        )
        .map_err(|e| format!("Failed to emit event: {}", e))?;

    info!("Marked attempt {} as verified", attempt_id);
    Ok(())
}
