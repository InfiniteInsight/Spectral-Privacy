# Vault Commands Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expose spectral-vault functionality to SvelteKit frontend via Tauri IPC commands.

**Architecture:** Multi-vault support with explicit vault_id parameters for security. AppState tracks unlocked vaults in HashMap with RwLock for concurrent reads. Five commands (create, unlock, lock, status, list) enable full vault lifecycle management. XDG-compliant storage at ~/.local/share/spectral/vaults/.

**Tech Stack:** Tauri 2.10, spectral-vault, spectral-db, directories crate, tokio async

---

## Task 1: CommandError Type

**Files:**
- Create: `src-tauri/src/error.rs`
- Test: Unit tests inline

**Step 1: Write CommandError type with conversion tests**

```rust
//! Error types for Tauri commands.

use serde::Serialize;
use spectral_vault::VaultError;

/// Serializable error for Tauri IPC commands.
#[derive(Debug, Serialize)]
pub struct CommandError {
    /// Error code for frontend handling (e.g., "VAULT_LOCKED")
    pub code: String,
    /// User-friendly error message
    pub message: String,
    /// Optional debugging context (never contains sensitive data)
    pub details: Option<serde_json::Value>,
}

impl CommandError {
    /// Create a new command error.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create a command error with details.
    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: Some(details),
        }
    }
}

/// Convert VaultError to CommandError for IPC serialization.
impl From<VaultError> for CommandError {
    fn from(err: VaultError) -> Self {
        match err {
            VaultError::Locked => {
                Self::new("VAULT_LOCKED", "Vault is locked and must be unlocked first")
            }
            VaultError::InvalidPassword => {
                Self::new("INVALID_PASSWORD", "Invalid password")
            }
            VaultError::VaultNotFound(path) => {
                Self::with_details(
                    "VAULT_NOT_FOUND",
                    "Vault does not exist",
                    serde_json::json!({ "path": path }),
                )
            }
            VaultError::KeyDerivation(msg) => {
                Self::new("KEY_DERIVATION_FAILED", format!("Key derivation failed: {msg}"))
            }
            VaultError::Encryption(msg) => {
                Self::new("ENCRYPTION_FAILED", format!("Encryption failed: {msg}"))
            }
            VaultError::Decryption(msg) => {
                Self::new("DECRYPTION_FAILED", format!("Decryption failed: {msg}"))
            }
            VaultError::Database(err) => {
                Self::new("DATABASE_ERROR", format!("Database error: {err}"))
            }
            VaultError::InvalidData(msg) => {
                Self::new("INVALID_DATA", format!("Invalid vault data: {msg}"))
            }
            VaultError::NotFound(field) => {
                Self::new("FIELD_NOT_FOUND", format!("Field not found: {field}"))
            }
            VaultError::Serialization(msg) => {
                Self::new("SERIALIZATION_ERROR", format!("Serialization error: {msg}"))
            }
        }
    }
}

/// Convert std::io::Error to CommandError.
impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        Self::new("FILESYSTEM_ERROR", format!("Filesystem error: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_error_new() {
        let err = CommandError::new("TEST_CODE", "Test message");
        assert_eq!(err.code, "TEST_CODE");
        assert_eq!(err.message, "Test message");
        assert!(err.details.is_none());
    }

    #[test]
    fn test_command_error_with_details() {
        let details = serde_json::json!({ "key": "value" });
        let err = CommandError::with_details("TEST_CODE", "Test message", details.clone());
        assert_eq!(err.code, "TEST_CODE");
        assert_eq!(err.message, "Test message");
        assert_eq!(err.details, Some(details));
    }

    #[test]
    fn test_vault_error_locked_conversion() {
        let err: CommandError = VaultError::Locked.into();
        assert_eq!(err.code, "VAULT_LOCKED");
        assert!(err.message.contains("locked"));
    }

    #[test]
    fn test_vault_error_invalid_password_conversion() {
        let err: CommandError = VaultError::InvalidPassword.into();
        assert_eq!(err.code, "INVALID_PASSWORD");
        assert_eq!(err.message, "Invalid password");
    }

    #[test]
    fn test_vault_error_not_found_conversion() {
        let err: CommandError = VaultError::VaultNotFound("/path/to/vault".to_string()).into();
        assert_eq!(err.code, "VAULT_NOT_FOUND");
        assert!(err.details.is_some());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: CommandError = io_err.into();
        assert_eq!(err.code, "FILESYSTEM_ERROR");
        assert!(err.message.contains("Filesystem error"));
    }

    #[test]
    fn test_error_serialization() {
        let err = CommandError::new("TEST_CODE", "Test message");
        let json = serde_json::to_string(&err).expect("serialize error");
        assert!(json.contains("TEST_CODE"));
        assert!(json.contains("Test message"));
    }
}
```

**Step 2: Run tests**

```bash
cargo test -p spectral-app error::tests
```

Expected: All tests pass (8 tests)

**Step 3: Commit**

```bash
git add src-tauri/src/error.rs
git commit -m "feat(tauri): add CommandError type with VaultError conversion

- Serializable error structure (code, message, details)
- Convert VaultError variants to frontend-friendly error codes
- Never expose sensitive data (passwords) in error messages
- All 8 unit tests passing"
```

---

## Task 2: AppState Structure

**Files:**
- Create: `src-tauri/src/state.rs`
- Modify: `src-tauri/Cargo.toml` (add dependencies)
- Test: Unit tests inline

**Step 1: Add dependencies to Cargo.toml**

```toml
# In [dependencies] section, uncomment and add:
spectral-vault = { path = "../crates/spectral-vault" }
spectral-db = { path = "../crates/spectral-db" }
directories.workspace = true
```

**Step 2: Write AppState with tests**

```rust
//! Application state management.

use spectral_vault::Vault;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Global application state shared across all Tauri commands.
pub struct AppState {
    /// Base directory for all vaults (~/.local/share/spectral/vaults/)
    pub vaults_dir: PathBuf,

    /// Currently unlocked vaults: vault_id -> Vault
    /// RwLock allows concurrent reads (status checks)
    pub unlocked_vaults: RwLock<HashMap<String, Arc<Vault>>>,
}

impl AppState {
    /// Create new application state with XDG-compliant paths.
    ///
    /// Creates vaults directory if it doesn't exist.
    pub fn new() -> Self {
        let dirs = directories::ProjectDirs::from("com", "spectral", "spectral")
            .expect("failed to determine project directories");
        let vaults_dir = dirs.data_dir().join("vaults");

        // Create vaults directory if it doesn't exist
        std::fs::create_dir_all(&vaults_dir)
            .unwrap_or_else(|e| tracing::warn!("Failed to create vaults directory: {}", e));

        tracing::info!("Vaults directory: {}", vaults_dir.display());

        Self {
            vaults_dir,
            unlocked_vaults: RwLock::new(HashMap::new()),
        }
    }

    /// Get the directory path for a specific vault.
    pub fn vault_dir(&self, vault_id: &str) -> PathBuf {
        self.vaults_dir.join(vault_id)
    }

    /// Get the database path for a specific vault.
    pub fn vault_db_path(&self, vault_id: &str) -> PathBuf {
        self.vault_dir(vault_id).join("vault.db")
    }

    /// Get the metadata path for a specific vault.
    pub fn vault_metadata_path(&self, vault_id: &str) -> PathBuf {
        self.vault_dir(vault_id).join("metadata.json")
    }

    /// Check if a vault exists on disk.
    pub fn vault_exists(&self, vault_id: &str) -> bool {
        self.vault_dir(vault_id).exists()
    }

    /// Check if a vault is currently unlocked.
    pub fn is_vault_unlocked(&self, vault_id: &str) -> bool {
        self.unlocked_vaults
            .read()
            .map(|vaults| vaults.contains_key(vault_id))
            .unwrap_or(false)
    }

    /// Insert an unlocked vault into the state.
    pub fn insert_vault(&self, vault_id: String, vault: Arc<Vault>) {
        self.unlocked_vaults
            .write()
            .unwrap()
            .insert(vault_id, vault);
    }

    /// Remove a vault from the unlocked state (locks it).
    pub fn remove_vault(&self, vault_id: &str) -> Option<Arc<Vault>> {
        self.unlocked_vaults.write().unwrap().remove(vault_id)
    }

    /// Get a reference to an unlocked vault.
    pub fn get_vault(&self, vault_id: &str) -> Option<Arc<Vault>> {
        self.unlocked_vaults
            .read()
            .unwrap()
            .get(vault_id)
            .cloned()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_appstate_new() {
        let state = AppState::new();
        assert!(state.vaults_dir.ends_with("spectral/vaults"));
        assert_eq!(state.unlocked_vaults.read().unwrap().len(), 0);
    }

    #[test]
    fn test_vault_dir() {
        let state = AppState::new();
        let vault_dir = state.vault_dir("test-vault-id");
        assert!(vault_dir.ends_with("vaults/test-vault-id"));
    }

    #[test]
    fn test_vault_db_path() {
        let state = AppState::new();
        let db_path = state.vault_db_path("test-vault-id");
        assert!(db_path.ends_with("vaults/test-vault-id/vault.db"));
    }

    #[test]
    fn test_vault_metadata_path() {
        let state = AppState::new();
        let metadata_path = state.vault_metadata_path("test-vault-id");
        assert!(metadata_path.ends_with("vaults/test-vault-id/metadata.json"));
    }

    #[test]
    fn test_vault_exists() {
        let state = AppState::new();
        // Non-existent vault should return false
        assert!(!state.vault_exists("nonexistent-vault"));
    }

    #[test]
    fn test_is_vault_unlocked() {
        let state = AppState::new();
        // Vault not in HashMap should return false
        assert!(!state.is_vault_unlocked("test-vault-id"));
    }
}
```

**Step 3: Run tests**

```bash
cargo test -p spectral-app state::tests
```

Expected: All 6 tests pass

**Step 4: Commit**

```bash
git add src-tauri/src/state.rs src-tauri/Cargo.toml
git commit -m "feat(tauri): add AppState with multi-vault support

- XDG-compliant vault storage (~/.local/share/spectral/vaults/)
- HashMap<String, Arc<Vault>> for concurrent unlocked vaults
- RwLock for thread-safe concurrent reads
- Helper methods for path construction and vault status
- All 6 unit tests passing"
```

---

## Task 3: Vault Metadata Schema

**Files:**
- Create: `src-tauri/src/metadata.rs`
- Test: Unit tests inline

**Step 1: Write VaultMetadata type**

```rust
//! Vault metadata schema.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata stored in plaintext for each vault.
///
/// Stored in `{vault_dir}/metadata.json` for quick discovery without decryption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMetadata {
    /// Unique vault identifier (UUID v4)
    pub vault_id: String,
    /// Display name for the vault (e.g., "Alice", "Bob")
    pub display_name: String,
    /// When the vault was created
    pub created_at: DateTime<Utc>,
    /// When the vault was last accessed (unlocked)
    pub last_accessed: DateTime<Utc>,
}

impl VaultMetadata {
    /// Create new vault metadata.
    pub fn new(vault_id: String, display_name: String) -> Self {
        let now = Utc::now();
        Self {
            vault_id,
            display_name,
            created_at: now,
            last_accessed: now,
        }
    }

    /// Update the last_accessed timestamp to now.
    pub fn touch(&mut self) {
        self.last_accessed = Utc::now();
    }

    /// Read metadata from a file.
    pub fn read_from_file(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Write metadata to a file.
    pub fn write_to_file(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(path, contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_vault_metadata_new() {
        let metadata = VaultMetadata::new("test-id".to_string(), "Test Vault".to_string());
        assert_eq!(metadata.vault_id, "test-id");
        assert_eq!(metadata.display_name, "Test Vault");
        assert!(metadata.created_at <= Utc::now());
        assert_eq!(metadata.created_at, metadata.last_accessed);
    }

    #[test]
    fn test_vault_metadata_touch() {
        let mut metadata = VaultMetadata::new("test-id".to_string(), "Test Vault".to_string());
        let original_time = metadata.last_accessed;

        std::thread::sleep(std::time::Duration::from_millis(10));
        metadata.touch();

        assert!(metadata.last_accessed > original_time);
    }

    #[test]
    fn test_vault_metadata_serialization() {
        let metadata = VaultMetadata::new("test-id".to_string(), "Test Vault".to_string());
        let json = serde_json::to_string(&metadata).expect("serialize");
        let deserialized: VaultMetadata = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(metadata.vault_id, deserialized.vault_id);
        assert_eq!(metadata.display_name, deserialized.display_name);
    }

    #[test]
    fn test_vault_metadata_file_roundtrip() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let file_path = temp_dir.path().join("metadata.json");

        let metadata = VaultMetadata::new("test-id".to_string(), "Test Vault".to_string());
        metadata.write_to_file(&file_path).expect("write file");

        let loaded = VaultMetadata::read_from_file(&file_path).expect("read file");
        assert_eq!(metadata.vault_id, loaded.vault_id);
        assert_eq!(metadata.display_name, loaded.display_name);
    }

    #[test]
    fn test_vault_metadata_read_nonexistent() {
        let result = VaultMetadata::read_from_file("/nonexistent/path/metadata.json");
        assert!(result.is_err());
    }
}
```

**Step 2: Add chrono dependency to Cargo.toml**

```toml
# In [dependencies] section:
chrono.workspace = true
```

**Step 3: Run tests**

```bash
cargo test -p spectral-app metadata::tests
```

Expected: All 5 tests pass

**Step 4: Commit**

```bash
git add src-tauri/src/metadata.rs src-tauri/Cargo.toml
git commit -m "feat(tauri): add VaultMetadata schema

- Plaintext metadata (vault_id, display_name, timestamps)
- File I/O for persistence in vault directories
- touch() method for updating last_accessed
- All 5 unit tests passing"
```

---

## Task 4: Vault Commands Module Structure

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/vault.rs` (stub)

**Step 1: Create commands module**

```rust
//! Tauri command handlers.

pub mod vault;
```

**Step 2: Create vault commands stub**

```rust
//! Vault management commands.

use crate::error::CommandError;
use crate::metadata::VaultMetadata;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

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

// Commands will be implemented in following tasks
```

**Step 3: Update lib.rs to declare modules**

```rust
// Add after use statements at top of file:
mod error;
mod metadata;
mod state;
mod commands;

// Keep existing code
```

**Step 4: Run build check**

```bash
cargo check -p spectral-app
```

Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src-tauri/src/commands/
git add src-tauri/src/lib.rs
git commit -m "feat(tauri): add commands module structure

- Create commands/mod.rs and commands/vault.rs
- Define VaultStatus and VaultInfo response types
- Declare modules in lib.rs"
```

---

## Task 5: vault_create Command

**Files:**
- Modify: `src-tauri/src/commands/vault.rs`

**Step 1: Implement vault_create command**

```rust
use spectral_vault::Vault;
use std::sync::Arc;
use tracing::{info, warn};

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

    // Create vault directory
    let vault_dir = state.vault_dir(&vault_id);
    std::fs::create_dir_all(&vault_dir)?;

    // Create encrypted vault database
    let db_path = state.vault_db_path(&vault_id);
    let vault = Vault::create(password, &db_path).await?;

    // Write metadata
    let metadata = VaultMetadata::new(vault_id.clone(), display_name);
    metadata.write_to_file(state.vault_metadata_path(&vault_id))?;

    // Insert into unlocked vaults
    state.insert_vault(vault_id.clone(), Arc::new(vault));

    info!("Vault created successfully: {}", vault_id);
    Ok(())
}
```

**Step 2: Build check**

```bash
cargo check -p spectral-app
```

Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src-tauri/src/commands/vault.rs
git commit -m "feat(tauri): implement vault_create command

- Create vault directory and encrypted database
- Store metadata.json with display name and timestamps
- Insert into unlocked vaults HashMap
- Validate vault doesn't already exist"
```

---

## Task 6: vault_unlock Command

**Files:**
- Modify: `src-tauri/src/commands/vault.rs`

**Step 1: Implement vault_unlock command**

```rust
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
    let vault = Vault::unlock(password, &db_path).await?;

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
```

**Step 2: Build check**

```bash
cargo check -p spectral-app
```

Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src-tauri/src/commands/vault.rs
git commit -m "feat(tauri): implement vault_unlock command

- Unlock vault with password verification
- Update last_accessed timestamp in metadata
- Idempotent: returns success if already unlocked
- Returns VAULT_NOT_FOUND if vault doesn't exist"
```

---

## Task 7: vault_lock Command

**Files:**
- Modify: `src-tauri/src/commands/vault.rs`

**Step 1: Implement vault_lock command**

```rust
/// Lock a vault.
///
/// Removes vault from unlocked state. Vault's Drop impl zeroizes keys.
/// Idempotent: returns success if already locked.
#[tauri::command]
pub async fn vault_lock(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<(), CommandError> {
    info!("Locking vault: {}", vault_id);

    // Remove from unlocked vaults (Drop impl zeroizes keys)
    state.remove_vault(&vault_id);

    info!("Vault locked: {}", vault_id);
    Ok(())
}
```

**Step 2: Build check**

```bash
cargo check -p spectral-app
```

Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src-tauri/src/commands/vault.rs
git commit -m "feat(tauri): implement vault_lock command

- Remove vault from unlocked HashMap
- Vault Drop impl automatically zeroizes encryption keys
- Idempotent: always returns success"
```

---

## Task 8: vault_status Command

**Files:**
- Modify: `src-tauri/src/commands/vault.rs`

**Step 1: Implement vault_status command**

```rust
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
```

**Step 2: Build check**

```bash
cargo check -p spectral-app
```

Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src-tauri/src/commands/vault.rs
git commit -m "feat(tauri): implement vault_status command

- Return exists, unlocked, and display_name
- Read metadata from disk if vault exists
- Returns VaultStatus struct for frontend"
```

---

## Task 9: list_vaults Command

**Files:**
- Modify: `src-tauri/src/commands/vault.rs`

**Step 1: Implement list_vaults command**

```rust
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
```

**Step 2: Build check**

```bash
cargo check -p spectral-app
```

Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src-tauri/src/commands/vault.rs
git commit -m "feat(tauri): implement list_vaults command

- Scan vaults directory for all vault subdirectories
- Read metadata.json for each vault
- Include unlocked status from AppState
- Return Vec<VaultInfo> with all vault details"
```

---

## Task 10: Register Commands in Tauri

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Initialize AppState and register commands**

```rust
// Replace the run() function:

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    info!("Starting Spectral v{}", env!("CARGO_PKG_VERSION"));

    // Initialize application state
    let app_state = AppState::new();

    tauri::Builder::default()
        .manage(app_state)
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                // Open devtools in debug builds
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health_check,
            get_version,
            commands::vault::vault_create,
            commands::vault::vault_unlock,
            commands::vault::vault_lock,
            commands::vault::vault_status,
            commands::vault::list_vaults,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 2: Build check**

```bash
cargo build -p spectral-app
```

Expected: Compiles successfully

**Step 3: Run existing tests**

```bash
cargo test -p spectral-app --lib
```

Expected: All tests pass

**Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(tauri): register vault commands and AppState

- Initialize AppState on startup
- Register all 5 vault commands with Tauri
- Manage AppState as shared state across commands"
```

---

## Task 11: Integration Test - Full Vault Lifecycle

**Files:**
- Create: `src-tauri/tests/vault_commands.rs`

**Step 1: Write full lifecycle integration test**

```rust
//! Integration tests for vault commands.

use spectral_app::state::AppState;
use spectral_app::commands::vault::*;
use tauri::State;
use tempfile::TempDir;
use uuid::Uuid;

/// Helper to create test AppState with temporary directory.
fn create_test_state() -> (AppState, TempDir) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let vaults_dir = temp_dir.path().join("vaults");
    std::fs::create_dir_all(&vaults_dir).expect("create vaults dir");

    let state = AppState {
        vaults_dir,
        unlocked_vaults: std::sync::RwLock::new(std::collections::HashMap::new()),
    };

    (state, temp_dir)
}

#[tokio::test]
async fn test_full_vault_lifecycle() {
    let (state, _temp_dir) = create_test_state();
    let vault_id = Uuid::new_v4().to_string();
    let password = "test-password-123"; // pragma: allowlist secret

    // 1. Create vault
    vault_create(
        State::from(&state),
        vault_id.clone(),
        "Test Vault".to_string(),
        password.to_string(),
    )
    .await
    .expect("create vault");

    // 2. Verify exists and unlocked
    let status = vault_status(State::from(&state), vault_id.clone())
        .await
        .expect("get status");
    assert!(status.exists);
    assert!(status.unlocked);
    assert_eq!(status.display_name, Some("Test Vault".to_string()));

    // 3. Lock vault
    vault_lock(State::from(&state), vault_id.clone())
        .await
        .expect("lock vault");

    // 4. Verify exists but locked
    let status = vault_status(State::from(&state), vault_id.clone())
        .await
        .expect("get status");
    assert!(status.exists);
    assert!(!status.unlocked);

    // 5. Unlock with correct password
    vault_unlock(State::from(&state), vault_id.clone(), password.to_string())
        .await
        .expect("unlock vault");

    let status = vault_status(State::from(&state), vault_id.clone())
        .await
        .expect("get status");
    assert!(status.unlocked);

    // 6. Verify wrong password fails
    let result = vault_unlock(
        State::from(&state),
        vault_id.clone(),
        "wrong-password".to_string(),
    )
    .await;
    assert!(result.is_err());

    // 7. Test list_vaults sees it
    let vaults = list_vaults(State::from(&state))
        .await
        .expect("list vaults");
    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0].vault_id, vault_id);
    assert_eq!(vaults[0].display_name, "Test Vault");
    assert!(vaults[0].unlocked);
}
```

**Step 2: Add uuid and tempfile to dev-dependencies**

```toml
# In [dev-dependencies] section of src-tauri/Cargo.toml:
uuid.workspace = true
tempfile.workspace = true
```

**Step 3: Make state module public**

In `src-tauri/src/lib.rs`, change:
```rust
mod state;
// to:
pub mod state;
```

And make commands public:
```rust
mod commands;
// to:
pub mod commands;
```

**Step 4: Run integration test**

```bash
cargo test -p spectral-app test_full_vault_lifecycle
```

Expected: Test passes

**Step 5: Commit**

```bash
git add src-tauri/tests/vault_commands.rs src-tauri/Cargo.toml src-tauri/src/lib.rs
git commit -m "test(tauri): add full vault lifecycle integration test

- Test create, unlock, lock, status, list flow
- Verify wrong password fails
- Verify vault appears in list with correct status
- Use tempfile for isolated test environment"
```

---

## Task 12: Integration Test - Multiple Vaults

**Files:**
- Modify: `src-tauri/tests/vault_commands.rs`

**Step 1: Add multiple vaults test**

```rust
#[tokio::test]
async fn test_multiple_vaults() {
    let (state, _temp_dir) = create_test_state();
    let vault1_id = Uuid::new_v4().to_string();
    let vault2_id = Uuid::new_v4().to_string();
    let password1 = "password-1"; // pragma: allowlist secret
    let password2 = "password-2"; // pragma: allowlist secret

    // 1. Create vault1 and vault2
    vault_create(
        State::from(&state),
        vault1_id.clone(),
        "Vault 1".to_string(),
        password1.to_string(),
    )
    .await
    .expect("create vault1");

    vault_create(
        State::from(&state),
        vault2_id.clone(),
        "Vault 2".to_string(),
        password2.to_string(),
    )
    .await
    .expect("create vault2");

    // 2. Verify both unlocked
    let vaults = list_vaults(State::from(&state))
        .await
        .expect("list vaults");
    assert_eq!(vaults.len(), 2);
    assert!(vaults.iter().all(|v| v.unlocked));

    // 3. Lock vault1
    vault_lock(State::from(&state), vault1_id.clone())
        .await
        .expect("lock vault1");

    // 4. Verify vault2 still unlocked
    let status1 = vault_status(State::from(&state), vault1_id.clone())
        .await
        .expect("get status1");
    let status2 = vault_status(State::from(&state), vault2_id.clone())
        .await
        .expect("get status2");

    assert!(!status1.unlocked);
    assert!(status2.unlocked);

    // 5. Verify list shows correct states
    let vaults = list_vaults(State::from(&state))
        .await
        .expect("list vaults");
    assert_eq!(vaults.len(), 2);

    let vault1_info = vaults.iter().find(|v| v.vault_id == vault1_id).unwrap();
    let vault2_info = vaults.iter().find(|v| v.vault_id == vault2_id).unwrap();

    assert!(!vault1_info.unlocked);
    assert!(vault2_info.unlocked);
}
```

**Step 2: Run integration test**

```bash
cargo test -p spectral-app test_multiple_vaults
```

Expected: Test passes

**Step 3: Commit**

```bash
git add src-tauri/tests/vault_commands.rs
git commit -m "test(tauri): add multiple vaults integration test

- Test creating and managing two vaults simultaneously
- Verify locking one vault doesn't affect the other
- Verify list_vaults correctly shows individual states"
```

---

## Task 13: Error Handling Tests

**Files:**
- Modify: `src-tauri/tests/vault_commands.rs`

**Step 1: Add error handling tests**

```rust
#[tokio::test]
async fn test_vault_already_exists() {
    let (state, _temp_dir) = create_test_state();
    let vault_id = Uuid::new_v4().to_string();

    // Create vault
    vault_create(
        State::from(&state),
        vault_id.clone(),
        "Test".to_string(),
        "password".to_string(),
    )
    .await
    .expect("create vault");

    // Try to create same vault again
    let result = vault_create(
        State::from(&state),
        vault_id.clone(),
        "Test".to_string(),
        "password".to_string(),
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, "VAULT_ALREADY_EXISTS");
}

#[tokio::test]
async fn test_vault_not_found() {
    let (state, _temp_dir) = create_test_state();
    let vault_id = Uuid::new_v4().to_string();

    // Try to unlock non-existent vault
    let result = vault_unlock(
        State::from(&state),
        vault_id.clone(),
        "password".to_string(),
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, "VAULT_NOT_FOUND");
}

#[tokio::test]
async fn test_vault_status_nonexistent() {
    let (state, _temp_dir) = create_test_state();
    let vault_id = Uuid::new_v4().to_string();

    let status = vault_status(State::from(&state), vault_id)
        .await
        .expect("get status");

    assert!(!status.exists);
    assert!(!status.unlocked);
    assert!(status.display_name.is_none());
}

#[tokio::test]
async fn test_vault_lock_idempotent() {
    let (state, _temp_dir) = create_test_state();
    let vault_id = Uuid::new_v4().to_string();

    // Lock non-existent/already-locked vault should succeed
    vault_lock(State::from(&state), vault_id)
        .await
        .expect("lock vault");
}

#[tokio::test]
async fn test_vault_unlock_idempotent() {
    let (state, _temp_dir) = create_test_state();
    let vault_id = Uuid::new_v4().to_string();
    let password = "password-123"; // pragma: allowlist secret

    // Create and verify unlocked
    vault_create(
        State::from(&state),
        vault_id.clone(),
        "Test".to_string(),
        password.to_string(),
    )
    .await
    .expect("create vault");

    // Unlock again (already unlocked) should succeed
    vault_unlock(State::from(&state), vault_id, password.to_string())
        .await
        .expect("unlock again");
}
```

**Step 2: Run error tests**

```bash
cargo test -p spectral-app
```

Expected: All tests pass (11 total)

**Step 3: Commit**

```bash
git add src-tauri/tests/vault_commands.rs
git commit -m "test(tauri): add error handling integration tests

- Test VAULT_ALREADY_EXISTS error
- Test VAULT_NOT_FOUND error
- Test vault_status for non-existent vault
- Test idempotent lock and unlock operations
- All 11 integration tests passing"
```

---

## Task 14: Run Clippy

**Files:**
- None (linting check)

**Step 1: Run clippy with strict settings**

```bash
cargo clippy -p spectral-app -- -D warnings
```

Expected: No warnings or errors

**Step 2: If warnings exist, fix them**

Common fixes:
- Add `#[allow(clippy::...)]` only if warning is false positive
- Add missing `#[must_use]` attributes
- Fix unnecessary clones or allocations
- Add documentation for public items

**Step 3: Commit any fixes**

```bash
git add -u
git commit -m "fix(tauri): address clippy warnings"
```

---

## Task 15: Final Test Run

**Files:**
- None (verification)

**Step 1: Clean build and test**

```bash
cargo clean -p spectral-app
cargo test -p spectral-app
```

Expected: All tests pass

**Step 2: Build release binary**

```bash
cargo build -p spectral-app --release
```

Expected: Builds successfully

**Step 3: Verify acceptance criteria checklist**

- ✅ Frontend can create new vault (vault_create command)
- ✅ Frontend can unlock vault with correct password (vault_unlock command)
- ✅ Frontend can lock vault (vault_lock command)
- ✅ Frontend can query vault status (vault_status command)
- ✅ Frontend can list all available vaults (list_vaults command)
- ✅ Errors properly serialized with codes and messages (CommandError)
- ✅ State persists across command calls (AppState with RwLock<HashMap>)
- ✅ Multiple vaults supported simultaneously (test_multiple_vaults)
- ✅ All tests pass (cargo test)
- ✅ Clippy passes (cargo clippy)

---

## Next Steps

After Task 1.4 completion, the following tasks become unblocked:

1. **Task 1.5: Unlock Screen UI** - Build SvelteKit components using the vault API (TypeScript wrappers and Svelte stores from design doc)

2. **Task 1.6: Database Integration** - Extend vault commands for profile operations (add_profile, get_profile, update_profile, delete_profile)

3. **Task 1.12: Onboarding UI** - First-run vault creation flow using vault_create command

The vault commands layer is now complete and ready for frontend integration.
