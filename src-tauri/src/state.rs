//! Application state management.

use spectral_vault::Vault;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Global application state shared across all Tauri commands.
#[allow(dead_code)] // Used by vault commands in later tasks
pub struct AppState {
    /// Base directory for all vaults (~/.local/share/spectral/vaults/)
    pub vaults_dir: PathBuf,

    /// Currently unlocked vaults: vault_id -> Vault
    /// RwLock allows concurrent reads (status checks)
    pub unlocked_vaults: RwLock<HashMap<String, Arc<Vault>>>,

    /// Shared browser engine for browser-form removal submissions.
    ///
    /// Lazily initialized on first use. Wrapped in `Option` so it can be
    /// initialized after construction without requiring Chromium at startup.
    /// Wrapped in `Arc` so it can be cloned into background worker tasks.
    pub browser_engine: Arc<tokio::sync::Mutex<Option<spectral_browser::BrowserEngine>>>,
}

#[allow(dead_code)] // Used by vault commands in later tasks
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
            browser_engine: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Get the directory path for a specific vault.
    pub fn vault_dir(&self, vault_id: &str) -> PathBuf {
        // Validate vault_id to prevent path traversal
        if vault_id.contains('/') || vault_id.contains('\\') || vault_id.contains("..") {
            panic!("Invalid vault_id: must not contain path separators or '..'");
        }
        if vault_id.is_empty() {
            panic!("Invalid vault_id: must not be empty");
        }

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
            .expect("RwLock poisoned: another thread panicked while holding the lock")
            .insert(vault_id, vault);
    }

    /// Remove a vault from the unlocked state (locks it).
    pub fn remove_vault(&self, vault_id: &str) -> Option<Arc<Vault>> {
        self.unlocked_vaults
            .write()
            .expect("RwLock poisoned: another thread panicked while holding the lock")
            .remove(vault_id)
    }

    /// Get a reference to an unlocked vault.
    pub fn get_vault(&self, vault_id: &str) -> Option<Arc<Vault>> {
        self.unlocked_vaults
            .read()
            .expect("RwLock poisoned: another thread panicked while holding the lock")
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

    #[test]
    #[should_panic(expected = "Invalid vault_id")]
    fn test_vault_id_path_traversal() {
        let state = AppState::new();
        state.vault_dir("../../../etc/passwd");
    }

    #[test]
    #[should_panic(expected = "Invalid vault_id")]
    fn test_vault_id_with_slash() {
        let state = AppState::new();
        state.vault_dir("vault/subdir");
    }

    #[test]
    #[should_panic(expected = "Invalid vault_id")]
    fn test_vault_id_empty() {
        let state = AppState::new();
        state.vault_dir("");
    }
}
