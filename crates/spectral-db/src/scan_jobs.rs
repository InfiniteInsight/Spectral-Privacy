//! Scan job management for tracking broker scan operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

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

/// Scan job history with statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanJobHistory {
    /// Scan job details
    pub scan_job: ScanJob,
    /// Total findings discovered
    pub total_findings: u32,
    /// Number of findings marked as confirmed
    pub confirmed_findings: u32,
    /// Number of findings marked as rejected
    pub rejected_findings: u32,
    /// Number of removal requests created
    pub removal_requests: u32,
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

/// Get all scan jobs for a profile with statistics.
///
/// # Errors
/// Returns an error if the database operation fails.
pub async fn get_scan_job_history(
    pool: &SqlitePool,
    profile_id: &str,
) -> Result<Vec<ScanJobHistory>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT
            sj.id, sj.profile_id, sj.started_at, sj.completed_at, sj.status,
            sj.total_brokers, sj.completed_brokers, sj.error_message,
            COUNT(DISTINCT f.id) as total_findings,
            COUNT(DISTINCT CASE WHEN f.verification_status = 'Confirmed' THEN f.id END) as confirmed_findings,
            COUNT(DISTINCT CASE WHEN f.verification_status = 'Rejected' THEN f.id END) as rejected_findings,
            COUNT(DISTINCT ra.id) as removal_requests
         FROM scan_jobs sj
         LEFT JOIN broker_scans bs ON bs.scan_job_id = sj.id
         LEFT JOIN findings f ON f.broker_scan_id = bs.id
         LEFT JOIN removal_attempts ra ON ra.finding_id = f.id
         WHERE sj.profile_id = ?
         GROUP BY sj.id
         ORDER BY sj.started_at DESC"
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await?;

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let history = rows
        .into_iter()
        .map(|row| {
            // nosemgrep: use-zeroize-for-secrets
            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "Completed" => ScanJobStatus::Completed,
                "Failed" => ScanJobStatus::Failed,
                "Cancelled" => ScanJobStatus::Cancelled,
                _ => ScanJobStatus::InProgress,
            };

            // nosemgrep: use-zeroize-for-secrets
            let started_at_str: String = row.get("started_at");
            let started_at = DateTime::parse_from_rfc3339(&started_at_str)
                .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

            let completed_at_str: Option<String> = row.get("completed_at");
            let completed_at = completed_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });

            let total_brokers: i64 = row.get("total_brokers");
            let completed_brokers: i64 = row.get("completed_brokers");
            let total_findings: i64 = row.get("total_findings");
            let confirmed_findings: i64 = row.get("confirmed_findings");
            let rejected_findings: i64 = row.get("rejected_findings");
            let removal_requests: i64 = row.get("removal_requests");

            ScanJobHistory {
                scan_job: ScanJob {
                    id: row.get("id"),
                    profile_id: row.get("profile_id"),
                    started_at,
                    completed_at,
                    status,
                    total_brokers: total_brokers as u32,
                    completed_brokers: completed_brokers as u32,
                    error_message: row.get("error_message"),
                },
                total_findings: total_findings as u32,
                confirmed_findings: confirmed_findings as u32,
                rejected_findings: rejected_findings as u32,
                removal_requests: removal_requests as u32,
            }
        })
        .collect();

    Ok(history)
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
