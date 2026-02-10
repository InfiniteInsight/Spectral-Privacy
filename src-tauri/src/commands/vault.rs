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

/// Unlock an existing vault with password.
///
/// Loads encrypted database and inserts into unlocked vaults.
/// Idempotent: returns success if already unlocked.
#[tauri::command]
#[allow(dead_code)] // Will be registered in Task 10
pub async fn vault_unlock(
    state: State<'_, AppState>,
    vault_id: String,
    password: String,
) -> Result<(), CommandError> {
    info!("Unlocking vault: {}", vault_id);

    // Check if vault exists
    if !state.vault_exists(&vault_id) {
        warn!("Vault not found: {}", vault_id);
        return Err(CommandError::new(
            "VAULT_NOT_FOUND",
            format!("Vault '{}' does not exist", vault_id),
        ));
    }

    // Check if already unlocked (idempotent)
    if state.is_vault_unlocked(&vault_id) {
        info!("Vault already unlocked: {}", vault_id);
        return Ok(());
    }

    // Unlock vault
    let db_path = state.vault_db_path(&vault_id);
    let vault = Vault::unlock(&password, &db_path).await?;

    // Update last_accessed in metadata
    let metadata_path = state.vault_metadata_path(&vault_id);
    if let Ok(mut metadata) = VaultMetadata::read_from_file(&metadata_path) {
        metadata.touch();
        metadata.write_to_file(&metadata_path).ok();
    }

    // Insert into unlocked vaults
    state.insert_vault(vault_id.clone(), Arc::new(vault));

    info!("Vault unlocked successfully: {}", vault_id);
    Ok(())
}

/// Lock a vault.
///
/// Removes vault from unlocked state. Vault's Drop impl zeroizes keys.
/// Idempotent: returns success if already locked.
#[tauri::command]
#[allow(dead_code)] // Will be registered in Task 10
pub async fn vault_lock(state: State<'_, AppState>, vault_id: String) -> Result<(), CommandError> {
    info!("Locking vault: {}", vault_id);

    // Remove from unlocked vaults (Drop impl zeroizes keys)
    state.remove_vault(&vault_id);

    info!("Vault locked: {}", vault_id);
    Ok(())
}

/// Get status of a specific vault.
///
/// Returns whether vault exists, is unlocked, and display name.
#[tauri::command]
#[allow(dead_code)] // Will be registered in Task 10
pub async fn vault_status(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<VaultStatus, CommandError> {
    let exists = state.vault_exists(&vault_id);
    let unlocked = state.is_vault_unlocked(&vault_id);

    let display_name = if exists {
        let metadata_path = state.vault_metadata_path(&vault_id);
        VaultMetadata::read_from_file(&metadata_path)
            .ok()
            .map(|m| m.display_name)
    } else {
        None
    };

    Ok(VaultStatus {
        exists,
        unlocked,
        display_name,
    })
}
