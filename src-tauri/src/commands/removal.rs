//! Removal submission commands.

use crate::error::CommandError;
use crate::state::AppState;
use spectral_broker::removal::RemovalOutcome;
use tauri::State;
use tracing::warn;

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
