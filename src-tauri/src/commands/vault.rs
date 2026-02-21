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

/// List all available vaults.
///
/// Scans vault directory and returns metadata for each vault.
#[tauri::command]
pub async fn list_vaults(state: State<'_, AppState>) -> Result<Vec<VaultInfo>, CommandError> {
    info!("Listing all vaults");

    let mut vaults = Vec::new();

    // Scan vaults directory
    let entries = match std::fs::read_dir(&state.vaults_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Vaults directory doesn't exist yet, return empty list
            return Ok(vaults);
        }
        Err(e) => return Err(e.into()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Read metadata
        let metadata_path = path.join("metadata.json");
        let metadata = match VaultMetadata::read_from_file(&metadata_path) {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to read metadata for {:?}: {}", path, e);
                continue;
            }
        };

        // Check if unlocked
        let unlocked = state.is_vault_unlocked(&metadata.vault_id);

        vaults.push(VaultInfo {
            vault_id: metadata.vault_id,
            display_name: metadata.display_name,
            created_at: metadata.created_at.to_rfc3339(),
            last_accessed: metadata.last_accessed.to_rfc3339(),
            unlocked,
        });
    }

    info!("Found {} vaults", vaults.len());
    Ok(vaults)
}

/// Rename a vault by updating its display name in metadata.
///
/// Reads `metadata.json` from the vault directory, updates `display_name`,
/// and writes it back.
#[tauri::command]
pub async fn rename_vault(
    state: State<'_, AppState>,
    vault_id: String,
    new_name: String,
) -> Result<(), CommandError> {
    info!("Renaming vault: {vault_id}");
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() {
        return Err(CommandError::new(
            "INVALID_NAME",
            "Display name cannot be empty",
        ));
    }

    if !state.vault_exists(&vault_id) {
        return Err(CommandError::new(
            "VAULT_NOT_FOUND",
            format!("Vault '{}' does not exist", vault_id),
        ));
    }

    let metadata_path = state.vault_metadata_path(&vault_id);
    let mut metadata = VaultMetadata::read_from_file(&metadata_path)?;
    metadata.display_name = new_name.clone();
    metadata.write_to_file(&metadata_path)?;

    info!("Renamed vault '{}' to '{}'", vault_id, new_name);
    Ok(())
}

/// Change the master password of a vault.
///
/// The vault must be currently unlocked. This operation is not yet implemented
/// because `spectral_vault::Vault` does not expose a `change_password` method.
#[tauri::command]
pub async fn change_vault_password(
    state: State<'_, AppState>,
    vault_id: String,
    old_password: String,
    new_password: String,
) -> Result<(), CommandError> {
    info!("Changing password for vault: {vault_id}");

    if new_password.len() < 8 {
        return Err(CommandError::new(
            "INVALID_PASSWORD",
            "Password must be at least 8 characters",
        ));
    }

    if !state.is_vault_unlocked(&vault_id) {
        return Err(CommandError::new(
            "VAULT_LOCKED",
            "Vault must be unlocked to change password",
        ));
    }

    // Get the vault before locking it
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new("VAULT_NOT_FOUND", format!("Vault '{}' not found", vault_id))
    })?;

    // Get database path
    let db_path = state.vault_db_path(&vault_id);
    let salt_path = db_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("salt");

    // Read current salt
    let current_salt = tokio::fs::read(&salt_path)
        .await
        .map_err(|e| CommandError::new("FILE_ERROR", format!("Failed to read salt file: {}", e)))?;

    // Verify old password by deriving key and comparing to current vault key
    use spectral_vault::kdf;
    let old_key = kdf::derive_key(&old_password, &current_salt).map_err(|e| {
        CommandError::new(
            "KEY_DERIVATION_ERROR",
            format!("Failed to derive key from old password: {}", e),
        )
    })?;

    let vault_key = vault
        .encryption_key()
        .map_err(|e| CommandError::new("VAULT_ERROR", format!("Failed to get vault key: {}", e)))?;

    if old_key.as_ref() != vault_key.as_ref() {
        return Err(CommandError::new(
            "INVALID_PASSWORD",
            "Old password is incorrect",
        ));
    }

    // Lock the vault temporarily
    state.remove_vault(&vault_id);
    drop(vault); // Explicitly drop to release the Arc

    // Generate new salt and derive new key
    let new_salt = kdf::generate_salt();
    let new_key = kdf::derive_key(&new_password, &new_salt).map_err(|e| {
        CommandError::new(
            "KEY_DERIVATION_ERROR",
            format!("Failed to derive new key: {}", e),
        )
    })?;

    // Connect to database with old key to perform rekey
    let old_key_hex = format!("\"x'{}'\"", hex::encode(old_key.as_ref()));
    let new_key_hex = format!("\"x'{}'\"", hex::encode(new_key.as_ref()));

    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| CommandError::new("FILE_ERROR", "Invalid database path: not valid UTF-8"))?;

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    let connect_options = SqliteConnectOptions::from_str(db_path_str)
        .map_err(|e| {
            CommandError::new(
                "DATABASE_ERROR",
                format!("Invalid connection string: {}", e),
            )
        })?
        .pragma("key", old_key_hex)
        .pragma("cipher_page_size", "4096")
        .pragma("kdf_iter", "256000")
        .pragma("cipher_hmac_algorithm", "HMAC_SHA512")
        .pragma("cipher_kdf_algorithm", "PBKDF2_HMAC_SHA512");

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(connect_options)
        .await
        .map_err(|e| {
            CommandError::new(
                "DATABASE_ERROR",
                format!("Failed to connect to database: {}", e),
            )
        })?;

    // Re-encrypt database with new key
    sqlx::query(&format!("PRAGMA rekey = {}", new_key_hex))
        .execute(&pool)
        .await
        .map_err(|e| {
            CommandError::new(
                "DATABASE_ERROR",
                format!("Failed to re-encrypt database: {}", e),
            )
        })?;

    // Close the pool
    pool.close().await;

    // Write new salt to file
    tokio::fs::write(&salt_path, new_salt).await.map_err(|e| {
        CommandError::new("FILE_ERROR", format!("Failed to write salt file: {}", e))
    })?;

    // Unlock vault with new password (this will verify the rekey worked)
    let new_vault = Vault::unlock(&new_password, &db_path).await.map_err(|e| {
        // Try to restore old state if unlock fails
        warn!("Failed to unlock vault with new password: {}", e);
        CommandError::new(
            "VAULT_ERROR",
            format!(
                "Password change failed - could not unlock with new password: {}",
                e
            ),
        )
    })?;

    // Add the re-unlocked vault back to state
    state.insert_vault(vault_id.clone(), Arc::new(new_vault));

    info!("Password changed successfully for vault: {vault_id}");
    Ok(())
}

/// Delete a vault after verifying the password.
///
/// Verifies the password by attempting to unlock the vault, removes it from
/// the unlocked map, then deletes the vault directory from disk.
#[tauri::command]
pub async fn delete_vault(
    state: State<'_, AppState>,
    vault_id: String,
    password: String,
) -> Result<(), CommandError> {
    info!("Deleting vault: {vault_id}");
    if !state.vault_exists(&vault_id) {
        return Err(CommandError::new(
            "VAULT_NOT_FOUND",
            format!("Vault '{}' does not exist", vault_id),
        ));
    }

    // Verify password by attempting to unlock
    let db_path = state.vault_db_path(&vault_id);
    Vault::unlock(&password, &db_path).await?;

    // Remove from unlocked vaults (Drop impl zeroizes key)
    state.remove_vault(&vault_id);

    // Delete vault directory
    let vault_dir = state.vault_dir(&vault_id);
    tokio::fs::remove_dir_all(&vault_dir).await?;

    info!("Deleted vault: {}", vault_id);
    Ok(())
}
