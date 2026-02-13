//! Database connection management with `SQLCipher` encryption.
//!
//! Provides an `EncryptedPool` wrapper around `SQLx` that handles `SQLCipher`
//! initialization and key management with automatic zeroization.

use crate::error::{DatabaseError, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;
use zeroize::Zeroizing;

/// Encrypted database connection pool using `SQLCipher`.
///
/// This wrapper manages a `SQLx` connection pool with `SQLCipher` encryption enabled.
/// The encryption key is zeroized on drop to prevent memory leaks.
#[derive(Debug)]
pub struct EncryptedPool {
    pool: Pool<Sqlite>,
    _key: Zeroizing<Vec<u8>>,
}

impl EncryptedPool {
    /// Create a new encrypted database connection pool.
    ///
    /// # Arguments
    /// * `path` - Path to the `SQLite` database file (or `:memory:` for in-memory)
    /// * `key` - 32-byte encryption key (will be zeroized on drop)
    ///
    /// # Errors
    /// Returns `DatabaseError` if:
    /// - The database file cannot be opened
    /// - `SQLCipher` pragmas fail to execute
    /// - The encryption key is invalid
    pub async fn new(path: impl AsRef<Path>, key: Vec<u8>) -> Result<Self> {
        if key.len() != 32 {
            return Err(DatabaseError::InvalidKey);
        }

        let key = Zeroizing::new(key);
        let path_str = path.as_ref().to_str().ok_or_else(|| {
            DatabaseError::Open("invalid database path: not valid UTF-8".to_string())
        })?;

        // Build connection options with SQLCipher pragmas
        // Note: SQLCipher requires hex keys to be prefixed with "x'" and suffixed with "'"
        let key_hex = format!("\"x'{}'\"", hex::encode(&*key));
        let connect_options = SqliteConnectOptions::from_str(path_str)
            .map_err(|e| DatabaseError::Open(format!("invalid connection string: {e}")))?
            .pragma("key", key_hex)
            .pragma("cipher_page_size", "4096")
            .pragma("kdf_iter", "256000")
            .pragma("cipher_hmac_algorithm", "HMAC_SHA512")
            .pragma("cipher_kdf_algorithm", "PBKDF2_HMAC_SHA512")
            .create_if_missing(true);

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connect_options)
            .await
            .map_err(|e| {
                DatabaseError::Encryption(format!("failed to initialize encrypted pool: {e}"))
            })?;

        tracing::info!("Encrypted database pool created at {}", path_str);

        Ok(Self { pool, _key: key })
    }

    /// Create an `EncryptedPool` from an existing pool and key.
    ///
    /// This is useful when you need to share the same connection pool
    /// across multiple components that each need their own `EncryptedPool` reference.
    ///
    /// # Arguments
    /// * `pool` - An existing `Pool<Sqlite>` (pools are Arc-based and can be cloned)
    /// * `key` - The encryption key (will be zeroized on drop)
    ///
    /// # Panics
    /// Panics if the key is not exactly 32 bytes.
    #[must_use]
    pub fn from_pool(pool: Pool<Sqlite>, key: Vec<u8>) -> Self {
        assert_eq!(key.len(), 32, "Encryption key must be exactly 32 bytes");
        Self {
            pool,
            _key: Zeroizing::new(key),
        }
    }

    /// Get a reference to the underlying `SQLx` pool.
    ///
    /// This allows consumers to execute queries directly using `SQLx`.
    #[must_use]
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Close the connection pool gracefully.
    ///
    /// This ensures all connections are properly closed before the pool is dropped.
    pub async fn close(self) {
        self.pool.close().await;
        tracing::info!("Encrypted database pool closed");
    }

    /// Verify that the database is accessible with the provided key.
    ///
    /// This performs a simple query to ensure the encryption key is correct.
    ///
    /// # Errors
    /// Returns `DatabaseError::InvalidKey` if the key is incorrect or the database is corrupted.
    pub async fn verify_key(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|_| DatabaseError::InvalidKey)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encrypted_pool_creation() {
        let key = vec![0u8; 32];
        let pool = EncryptedPool::new(":memory:", key)
            .await
            .expect("create encrypted pool");

        pool.verify_key().await.expect("verify encryption key");
    }

    #[tokio::test]
    async fn test_invalid_key_length() {
        let short_key = vec![0u8; 16];
        let result = EncryptedPool::new(":memory:", short_key).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DatabaseError::InvalidKey));
    }

    #[tokio::test]
    async fn test_pool_close() {
        let key = vec![0u8; 32];
        let pool = EncryptedPool::new(":memory:", key)
            .await
            .expect("create encrypted pool");

        pool.close().await; // Should not panic
    }
}
