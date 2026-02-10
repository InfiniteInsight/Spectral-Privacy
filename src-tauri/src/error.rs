//! Error types for Tauri commands.

#![allow(dead_code)] // Used by vault commands (implemented in later tasks)

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
            VaultError::InvalidPassword => Self::new("INVALID_PASSWORD", "Invalid password"),
            VaultError::VaultNotFound(path) => Self::with_details(
                "VAULT_NOT_FOUND",
                "Vault does not exist",
                serde_json::json!({ "path": path }),
            ),
            VaultError::KeyDerivation(msg) => Self::new(
                "KEY_DERIVATION_FAILED",
                format!("Key derivation failed: {msg}"),
            ),
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

    #[test]
    fn test_vault_error_serialization() {
        let err: CommandError = VaultError::Locked.into();
        let json = serde_json::to_string(&err).expect("serialize error");
        assert!(json.contains("VAULT_LOCKED"));
        assert!(json.contains("locked"));
    }
}
