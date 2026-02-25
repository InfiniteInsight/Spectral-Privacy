//! Error types for cookie operations.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, CookieError>;

#[derive(Debug, Error)]
pub enum CookieError {
    #[error("Browser not found: {0}")]
    BrowserNotFound(String),

    #[error("Browser database locked: {0}")]
    BrowserLocked(String),

    #[error("Failed to read cookie database: {0}")]
    DatabaseError(String),

    #[error("Failed to parse cookie: {0}")]
    ParseError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),

    #[error("SQLx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid cookie format: {0}")]
    InvalidFormat(String),

    #[error("Backup failed: {0}")]
    BackupFailed(String),

    #[error("Restore failed: {0}")]
    RestoreFailed(String),

    #[error("Browser running: Cannot modify cookies while {0} is running")]
    BrowserRunning(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
