//! Vault management commands.

use crate::error::CommandError;
use crate::metadata::VaultMetadata;
use crate::state::AppState;
use serde::Serialize;
use spectral_vault::Vault;
use std::sync::Arc;
use tauri::State;
use tracing::{info, warn};

/// Response for vault_status command.
#[derive(Debug, Serialize)]
#[allow(dead_code)] // Will be used in Task 8
pub struct VaultStatus {
    pub exists: bool,
    pub unlocked: bool,
    pub display_name: Option<String>,
}

/// Response for list_vaults command.
#[derive(Debug, Serialize)]
#[allow(dead_code)] // Will be used in Task 9
pub struct VaultInfo {
    pub vault_id: String,
    pub display_name: String,
    pub created_at: String,
    pub last_accessed: String,
    pub unlocked: bool,
}

/// Create a new vault.
///
/// Creates vault directory, initializes encrypted database, and stores metadata.
#[tauri::command]
#[allow(dead_code)] // Will be registered in Task 10
pub async fn vault_create(
    state: State<'_, AppState>,
    vault_id: String,
    display_name: String,
    password: String,
) -> Result<(), CommandError> {
    info!("Creating vault: {} ({})", display_name, vault_id);

    // Validate vault doesn't already exist
    if state.vault_exists(&vault_id) {
        warn!("Vault already exists: {}", vault_id);
        return Err(CommandError::new(
            "VAULT_ALREADY_EXISTS",
            format!("Vault '{}' already exists", vault_id),
        ));
    }

    // Create vault directory (async)
    let vault_dir = state.vault_dir(&vault_id);
    tokio::fs::create_dir_all(&vault_dir).await?;

    // Create encrypted vault database
    // If this fails, we need to clean up the directory
    let db_path = state.vault_db_path(&vault_id);
    let vault = match Vault::create(&password, &db_path).await {
        Ok(v) => v,
        Err(e) => {
            // Clean up directory on failure
            tokio::fs::remove_dir_all(&vault_dir).await.ok();
            return Err(e.into());
        }
    };

    // Write metadata (with cleanup on failure)
    let metadata = VaultMetadata::new(vault_id.clone(), display_name);
    if let Err(e) = metadata.write_to_file(state.vault_metadata_path(&vault_id)) {
        // Clean up on metadata write failure
        tokio::fs::remove_dir_all(&vault_dir).await.ok();
        return Err(e.into());
    }

    // Insert into unlocked vaults
    state.insert_vault(vault_id.clone(), Arc::new(vault));

    info!("Vault created successfully: {}", vault_id);
    Ok(())
}
