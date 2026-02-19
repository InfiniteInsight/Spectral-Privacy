//! Database error types.
//!
//! Provides comprehensive error handling for database operations using `thiserror`.

use thiserror::Error;

/// Database-specific errors.
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// Failed to open or create database connection.
    #[error("failed to open database: {0}")]
    Open(String),

    /// Failed to configure database encryption.
    #[error("encryption configuration failed: {0}")]
    Encryption(String),

    /// Migration execution failed.
    #[error("migration failed: {0}")]
    Migration(String),

    /// Query execution failed.
    #[error("query failed: {0}")]
    Query(String),

    /// Connection pool exhausted or unavailable.
    #[error("connection pool exhausted")]
    PoolExhausted,

    /// Requested record was not found.
    #[error("record not found")]
    NotFound,

    /// Database record with provided identifier not found.
    #[error("{0}")]
    NotFoundWithMessage(String),

    /// Failed to decode database value.
    #[error("decode error: {0}")]
    Decode(String),

    /// Invalid encryption key provided.
    #[error("invalid encryption key")]
    InvalidKey,

    /// Serialization/deserialization failed.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// Underlying `SQLx` error.
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// I/O error during database operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type alias for database operations.
pub type Result<T> = std::result::Result<T, DatabaseError>;
