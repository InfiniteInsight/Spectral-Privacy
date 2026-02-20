//! Spectral Database Layer
//!
//! Provides `SQLite` database access with `SQLCipher` encryption for at-rest data protection.
//! Uses `SQLx` for compile-time checked queries and embedded migrations.
//!
//! # Architecture
//!
//! - **Encryption**: All data is encrypted at rest using `SQLCipher` with `AES-256`
//! - **Migrations**: SQL migrations are embedded and versioned using `SQLx`
//! - **Connection Pooling**: Configurable connection pool with automatic cleanup
//! - **Key Management**: Encryption keys are zeroized on drop to prevent memory leaks
//!
//! # Example
//!
//! ```ignore
//! use spectral_db::{Database, migrations};
//!
//! let key = vec![0u8; 32]; // In practice, derive from user password
//! let db = Database::new("spectral.db", key).await?;
//! db.run_migrations().await?;
//! ```
//!
//! # Design Principles
//!
//! - PII is encrypted at the application layer (spectral-vault), not database layer
//! - All queries use `sqlx::query!` macro for compile-time verification
//! - Migrations run automatically on first connection
//! - Connection pooling with configurable limits (default: 5 connections)

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod broker_scans;
pub mod connection;
pub mod discovery_findings;
pub mod error;
pub mod findings;
pub mod migrations;
pub mod removal_attempts;
/// Scan job management for tracking broker scan operations.
pub mod scan_jobs;
pub mod settings;

// Re-export commonly used types
pub use connection::EncryptedPool;
pub use error::{DatabaseError, Result};

use std::path::Path;

/// High-level database interface with encryption and migrations.
///
/// This provides a convenient wrapper around `EncryptedPool` that handles
/// initialization and migration automatically.
#[derive(Debug)]
pub struct Database {
    pool: EncryptedPool,
}

impl Database {
    /// Create a new database connection with encryption.
    ///
    /// This initializes an encrypted `SQLite` database at the specified path.
    /// The encryption key must be exactly 32 bytes.
    ///
    /// # Arguments
    /// * `path` - Path to the database file (or `:memory:` for in-memory)
    /// * `key` - 32-byte encryption key (will be zeroized on drop)
    ///
    /// # Errors
    /// Returns `DatabaseError` if the database cannot be opened or the key is invalid.
    pub async fn new(path: impl AsRef<Path>, key: Vec<u8>) -> Result<Self> {
        let pool = EncryptedPool::new(path, key).await?;
        Ok(Self { pool })
    }

    /// Create a database instance from an existing encrypted pool.
    ///
    /// This is useful when you already have an `EncryptedPool` and need to
    /// wrap it in the higher-level `Database` interface.
    ///
    /// # Arguments
    /// * `pool` - An existing encrypted connection pool
    #[must_use]
    pub fn from_encrypted_pool(pool: EncryptedPool) -> Self {
        Self { pool }
    }

    /// Run all pending database migrations.
    ///
    /// This should be called after creating a new database instance to ensure
    /// the schema is up to date.
    ///
    /// # Errors
    /// Returns `DatabaseError::Migration` if any migration fails.
    pub async fn run_migrations(&self) -> Result<()> {
        migrations::run_migrations(self.pool.pool()).await
    }

    /// Get the current schema version.
    ///
    /// Returns the number of applied migrations.
    ///
    /// # Errors
    /// Returns `DatabaseError` if the version cannot be queried.
    pub async fn get_schema_version(&self) -> Result<i64> {
        migrations::get_schema_version(self.pool.pool()).await
    }

    /// Get a reference to the underlying connection pool.
    ///
    /// This allows direct access to the `SQLx` pool for custom queries.
    #[must_use]
    pub fn pool(&self) -> &sqlx::Pool<sqlx::Sqlite> {
        self.pool.pool()
    }

    /// Get a reference to the encrypted pool.
    ///
    /// This is used by components that need access to the full `EncryptedPool`
    /// wrapper (e.g., for cloning into Arc for background tasks).
    #[must_use]
    pub fn encrypted_pool(&self) -> &EncryptedPool {
        &self.pool
    }

    /// Verify that the database is accessible with the provided key.
    ///
    /// # Errors
    /// Returns `DatabaseError::InvalidKey` if the key is incorrect.
    pub async fn verify_key(&self) -> Result<()> {
        self.pool.verify_key().await
    }

    /// Close the database connection gracefully.
    ///
    /// This ensures all connections are properly closed and resources are cleaned up.
    pub async fn close(self) {
        self.pool.close().await;
    }

    /// Get all scheduled jobs
    pub async fn get_scheduled_jobs(&self) -> Result<Vec<spectral_scheduler::ScheduledJob>> {
        let rows = sqlx::query_as::<_, (String, String, i64, String, Option<String>, i64)>(
            r"SELECT id, job_type, interval_days, next_run_at, last_run_at, enabled
               FROM scheduled_jobs",
        )
        .fetch_all(self.pool.pool())
        .await?;

        let jobs: Result<Vec<_>> = rows
            .into_iter()
            .map(
                |(id, job_type_str, interval_days, next_run_at, last_run_at, enabled)| {
                    let job_type: spectral_scheduler::JobType =
                        serde_json::from_str(&format!("\"{job_type_str}\"")).map_err(|e| {
                            DatabaseError::Decode(format!(
                                "Invalid job_type '{job_type_str}' in scheduled_jobs table: {e}"
                            ))
                        })?;
                    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                    let interval_days = interval_days as u32;
                    Ok(spectral_scheduler::ScheduledJob {
                        id,
                        job_type,
                        interval_days,
                        next_run_at,
                        last_run_at,
                        enabled: enabled != 0,
                    })
                },
            )
            .collect();

        jobs
    }

    /// Update job's `next_run_at` and `last_run_at` timestamps
    pub async fn update_job_next_run(
        &self,
        job_id: &str,
        next_run_at: &str,
        last_run_at: &str,
    ) -> Result<()> {
        let result =
            sqlx::query("UPDATE scheduled_jobs SET next_run_at = ?, last_run_at = ? WHERE id = ?")
                .bind(next_run_at)
                .bind(last_run_at)
                .bind(job_id)
                .execute(self.pool.pool())
                .await?;

        if result.rows_affected() == 0 {
            return Err(DatabaseError::NotFoundWithMessage(format!(
                "Scheduled job '{job_id}' not found"
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_creation() {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create database");

        db.verify_key().await.expect("verify encryption key");
    }

    #[tokio::test]
    async fn test_database_migrations() {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create database");

        let version_before = db.get_schema_version().await.expect("get version");
        assert_eq!(version_before, 0);

        db.run_migrations().await.expect("run migrations");

        let version_after = db.get_schema_version().await.expect("get version");
        assert_eq!(version_after, 9);
    }

    #[tokio::test]
    async fn test_database_schema() {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create database");

        db.run_migrations().await.expect("run migrations");

        // Verify all tables exist
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations' ORDER BY name"
        )
        .fetch_all(db.pool())
        .await
        .expect("query tables");

        assert_eq!(
            tables,
            vec![
                "audit_log",
                "broker_results",
                "broker_scans",
                "discovery_findings",
                "email_removals",
                "findings",
                "profiles",
                "removal_attempts",
                "removal_evidence",
                "scan_jobs",
                "scheduled_jobs"
            ]
        );

        // Verify profiles table schema
        let profile_columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('profiles') ORDER BY cid")
                .fetch_all(db.pool())
                .await
                .expect("query columns");

        assert_eq!(
            profile_columns,
            vec!["id", "data", "nonce", "created_at", "updated_at"]
        );
    }

    #[tokio::test]
    async fn test_database_close() {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create database");

        db.close().await; // Should not panic
    }
}

#[cfg(test)]
mod migration_tests {
    use super::*;

    #[tokio::test]
    async fn test_005_audit_log_migration() {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");
        let pool = db.pool();
        sqlx::query("INSERT INTO audit_log (id, vault_id, timestamp, event_type, subject, data_destination, outcome) VALUES ('test-id', 'vault-1', '2026-01-01T00:00:00Z', 'VaultUnlocked', 'core', 'LocalOnly', 'Allowed')")
            .execute(pool)
            .await
            .expect("audit_log table should exist after migration 005");
    }

    #[tokio::test]
    async fn test_006_removal_evidence_migration() {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");
        // Acquire a single connection to ensure PRAGMA and INSERT run on the same connection
        let mut conn = db.pool().acquire().await.expect("acquire connection");
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(conn.as_mut())
            .await
            .expect("disable foreign keys");
        sqlx::query("INSERT INTO removal_evidence (id, attempt_id, screenshot_bytes, captured_at) VALUES ('ev-1', 'att-1', X'00', '2026-01-01T00:00:00Z')")
            .execute(conn.as_mut())
            .await
            .expect("removal_evidence table must exist");
    }

    #[tokio::test]
    async fn test_008_scheduled_jobs_migration() {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");

        // Verify table exists and has default jobs
        let jobs = db.get_scheduled_jobs().await.expect("get scheduled jobs");
        assert_eq!(jobs.len(), 2);

        // Verify default jobs
        let scan_all = jobs
            .iter()
            .find(|j| j.id == "default-scan-all")
            .expect("scan-all job");
        assert_eq!(scan_all.job_type, spectral_scheduler::JobType::ScanAll);
        assert_eq!(scan_all.interval_days, 7);
        assert!(scan_all.enabled);

        let verify_removals = jobs
            .iter()
            .find(|j| j.id == "default-verify-removals")
            .expect("verify-removals job");
        assert_eq!(
            verify_removals.job_type,
            spectral_scheduler::JobType::VerifyRemovals
        );
        assert_eq!(verify_removals.interval_days, 3);
        assert!(verify_removals.enabled);
    }

    #[tokio::test]
    async fn test_get_scheduled_jobs_handles_invalid_job_type() {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");

        // Insert a job with an invalid job_type
        sqlx::query(
            "INSERT INTO scheduled_jobs (id, job_type, interval_days, next_run_at, enabled)
             VALUES ('invalid-job', 'InvalidJobType', 1, datetime('now'), 1)",
        )
        .execute(db.pool())
        .await
        .expect("insert invalid job");

        // Should return an error instead of panicking
        let result = db.get_scheduled_jobs().await;
        assert!(result.is_err());
        match result {
            Err(DatabaseError::Decode(msg)) => {
                assert!(msg.contains("Invalid job_type 'InvalidJobType'"));
            }
            _ => panic!("Expected Decode error"),
        }
    }

    #[tokio::test]
    async fn test_update_job_next_run_handles_missing_job() {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");

        // Try to update a non-existent job
        let result = db
            .update_job_next_run(
                "non-existent-job",
                "2026-03-01T00:00:00Z",
                "2026-02-01T00:00:00Z",
            )
            .await;

        assert!(result.is_err());
        match result {
            Err(DatabaseError::NotFoundWithMessage(msg)) => {
                assert!(msg.contains("Scheduled job 'non-existent-job' not found"));
            }
            _ => panic!("Expected NotFoundWithMessage error"),
        }
    }

    #[tokio::test]
    async fn test_update_job_next_run_succeeds_for_existing_job() {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");

        // Update an existing job
        let result = db
            .update_job_next_run(
                "default-scan-all",
                "2026-03-01T00:00:00Z",
                "2026-02-01T00:00:00Z",
            )
            .await;

        assert!(result.is_ok());

        // Verify the update
        let jobs = db.get_scheduled_jobs().await.expect("get jobs");
        let updated_job = jobs
            .iter()
            .find(|j| j.id == "default-scan-all")
            .expect("find updated job");
        assert_eq!(updated_job.next_run_at, "2026-03-01T00:00:00Z");
        assert_eq!(
            updated_job.last_run_at,
            Some("2026-02-01T00:00:00Z".to_string())
        );
    }
}

#[cfg(test)]
mod scan_tests {
    use super::*;

    async fn create_test_database() -> Result<Database> {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key).await?;
        db.run_migrations().await?;
        Ok(db)
    }

    #[tokio::test]
    async fn test_scan_jobs_table_exists() {
        let db = create_test_database().await.expect("create test database");

        // Try to query scan_jobs table
        let result = sqlx::query("SELECT id FROM scan_jobs LIMIT 1")
            .fetch_optional(db.pool())
            .await;

        assert!(result.is_ok());
    }
}
