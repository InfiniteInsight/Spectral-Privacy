//! Scan job management for tracking broker scan operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Represents a scan job that tracks the overall progress of scanning brokers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanJob {
    /// Unique identifier for the scan job
    pub id: String,
    /// Profile ID being scanned
    pub profile_id: String,
    /// When the scan started
    pub started_at: DateTime<Utc>,
    /// When the scan completed (if finished)
    pub completed_at: Option<DateTime<Utc>>,
    /// Current status of the scan
    pub status: ScanJobStatus,
    /// Total number of brokers to scan
    pub total_brokers: u32,
    /// Number of brokers completed so far
    pub completed_brokers: u32,
    /// Error message if the scan failed
    pub error_message: Option<String>,
}

/// Status of a scan job.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScanJobStatus {
    /// Scan is currently in progress
    InProgress,
    /// Scan completed successfully
    Completed,
    /// Scan failed with an error
    Failed,
    /// Scan was cancelled by the user
    Cancelled,
}

impl std::fmt::Display for ScanJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InProgress => write!(f, "InProgress"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Create a new scan job in the database.
///
/// # Errors
/// Returns an error if the database operation fails or if the `profile_id` doesn't exist.
pub async fn create_scan_job(
    pool: &SqlitePool,
    profile_id: String,
    total_brokers: u32,
) -> Result<ScanJob, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let started_at = Utc::now();
    let status = ScanJobStatus::InProgress;

    sqlx::query(
        "INSERT INTO scan_jobs (id, profile_id, started_at, status, total_brokers, completed_brokers)
         VALUES (?, ?, ?, ?, ?, 0)"
    )
    .bind(&id)
    .bind(&profile_id)
    .bind(started_at.to_rfc3339())
    .bind(status.to_string())
    .bind(i64::from(total_brokers))
    .execute(pool)
    .await?;

    Ok(ScanJob {
        id,
        profile_id,
        started_at,
        completed_at: None,
        status,
        total_brokers,
        completed_brokers: 0,
        error_message: None,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::Database;

    async fn setup_test_db() -> Database {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create test database");
        db.run_migrations().await.expect("run migrations");
        db
    }

    #[tokio::test]
    async fn test_create_scan_job() {
        let db = setup_test_db().await;

        // Create a test profile first to satisfy foreign key constraint
        sqlx::query(
            "INSERT INTO profiles (id, data, nonce, created_at, updated_at)
             VALUES (?, ?, ?, datetime('now'), datetime('now'))",
        )
        .bind("profile-123")
        .bind("encrypted_data")
        .bind("nonce")
        .execute(db.pool())
        .await
        .expect("create test profile");

        let job = create_scan_job(db.pool(), "profile-123".to_string(), 5)
            .await
            .expect("create scan job");

        assert_eq!(job.profile_id, "profile-123");
        assert_eq!(job.total_brokers, 5);
        assert_eq!(job.completed_brokers, 0);
        assert_eq!(job.status, ScanJobStatus::InProgress);
    }
}
