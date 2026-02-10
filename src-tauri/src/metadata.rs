//! Vault metadata schema.

#![allow(dead_code)] // Will be used by vault commands

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
