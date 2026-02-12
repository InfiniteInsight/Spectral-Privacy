//! Removal submission commands.

use crate::error::CommandError;
use crate::state::AppState;
use spectral_broker::removal::{RemovalOutcome, WebFormSubmitter};
use spectral_broker::BrokerRegistry;
use spectral_core::BrokerId;
use spectral_db::removal_attempts::RemovalAttempt;
use std::collections::HashMap;
use tauri::State;
use tracing::info;

/// Submit a removal request for a search result.
#[tauri::command]
pub async fn submit_removal(
    state: State<'_, AppState>,
    vault_id: String,
    broker_result_id: String,
) -> Result<RemovalOutcome, CommandError> {
    info!("Submitting removal for result: {}", broker_result_id);

    // TODO: Load search result from database to get:
    // - broker_id
    // - found_listing_url
    // - profile data for field values

    // For now, use hardcoded test data
    let broker_id = BrokerId::new("spokeo")
        .map_err(|e| CommandError::new("INVALID_BROKER_ID", e.to_string()))?;

    // Load broker definition
    let registry = BrokerRegistry::new();
    let broker_def = registry
        .get(&broker_id)
        .map_err(|e| CommandError::new("BROKER_NOT_FOUND", e.to_string()))?;

    // Prepare field values
    let mut field_values = HashMap::new();
    field_values.insert(
        "listing_url".to_string(),
        "https://www.spokeo.com/John-Doe/CA/San-Francisco".to_string(),
    );
    field_values.insert("email".to_string(), "user@example.com".to_string());

    // Create submitter and submit
    let submitter = WebFormSubmitter::new()
        .await
        .map_err(|e| CommandError::new("BROWSER_ERROR", e.to_string()))?;

    let outcome = submitter
        .submit(&broker_def, field_values)
        .await
        .map_err(|e| CommandError::new("SUBMISSION_ERROR", e.to_string()))?;

    // Save attempt to database
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new(
            "VAULT_NOT_UNLOCKED",
            format!("Vault '{}' is not unlocked", vault_id),
        )
    })?;

    let db = vault.database()?;

    let attempt = RemovalAttempt::new(
        broker_result_id,
        broker_id.as_str().to_string(),
        outcome.clone(),
    );

    attempt
        .save(db.pool())
        .await
        .map_err(|e| CommandError::new("DB_ERROR", e.to_string()))?;

    info!("Removal submitted successfully: {:?}", outcome);
    Ok(outcome)
}
