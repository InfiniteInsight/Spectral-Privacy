//! Error types for the vault module.

use thiserror::Error;

/// Errors that can occur during vault operations.
#[derive(Debug, Error)]
pub enum VaultError {
    /// Vault is locked and must be unlocked first.
    #[error("vault is locked")]
    Locked,

    /// Failed to derive key from password.
    #[error("key derivation failed: {0}")]
    KeyDerivation(String),

    /// Encryption operation failed.
    #[error("encryption failed: {0}")]
    Encryption(String),

    /// Decryption operation failed (wrong password or corrupted data).
    #[error("decryption failed: {0}")]
    Decryption(String),

    /// Requested PII field not found.
    #[error("field not found: {0}")]
    NotFound(String),

    /// Invalid vault format or corrupted data.
    #[error("invalid vault data: {0}")]
    InvalidData(String),

    /// Database operation failed.
    #[error("database error: {0}")]
    Database(#[from] spectral_db::DatabaseError),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Invalid password (authentication failure).
    #[error("invalid password")]
    InvalidPassword,

    /// Vault does not exist (must be created first).
    #[error("vault does not exist at {0}")]
    VaultNotFound(String),
}

/// Result type for vault operations.
pub type Result<T> = std::result::Result<T, VaultError>;
