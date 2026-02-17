//! Removal attempts operations for tracking removal request submissions.
//!
//! This module provides CRUD operations for the `removal_attempts` table,
//! which stores removal request submissions for confirmed findings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use std::fmt;
use uuid::Uuid;

/// Status of a removal attempt.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum RemovalStatus {
    /// Request is pending submission
    Pending,
    /// Request has been submitted to the broker
    Submitted,
    /// Removal has been completed
    Completed,
    /// Removal request failed
    Failed,
}

impl fmt::Display for RemovalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Submitted => write!(f, "Submitted"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// A removal attempt represents a removal request submission to a data broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovalAttempt {
    /// Unique identifier
    pub id: String,
    /// ID of the finding being removed
    pub finding_id: String,
    /// ID of the broker
    pub broker_id: String,
    /// Status of the removal attempt
    pub status: RemovalStatus,
    /// When the attempt was created
    pub created_at: DateTime<Utc>,
    /// When the request was submitted (if submitted)
    pub submitted_at: Option<DateTime<Utc>>,
    /// When the removal was completed (if completed)
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Create a new removal attempt.
///
/// Creates a removal attempt with status "Pending" and links it to the finding.
///
/// # Errors
/// Returns `sqlx::Error` if the database insert fails.
pub async fn create_removal_attempt(
    pool: &Pool<Sqlite>,
    finding_id: String,
    broker_id: String,
) -> Result<RemovalAttempt, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now();

    // Insert removal attempt
    sqlx::query(
        "INSERT INTO removal_attempts (id, finding_id, broker_id, status, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&finding_id)
    .bind(&broker_id)
    .bind(RemovalStatus::Pending.to_string())
    .bind(created_at.to_rfc3339())
    .execute(pool)
    .await?;

    // Link removal attempt to finding
    sqlx::query("UPDATE findings SET removal_attempt_id = ? WHERE id = ?")
        .bind(&id)
        .bind(&finding_id)
        .execute(pool)
        .await?;

    Ok(RemovalAttempt {
        id,
        finding_id,
        broker_id,
        status: RemovalStatus::Pending,
        created_at,
        submitted_at: None,
        completed_at: None,
        error_message: None,
    })
}

/// Get all removal attempts for a specific finding.
///
/// Returns removal attempts ordered by newest first.
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn get_by_finding_id(
    pool: &Pool<Sqlite>,
    finding_id: &str,
) -> Result<Vec<RemovalAttempt>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, finding_id, broker_id, status, created_at, submitted_at, completed_at, error_message
         FROM removal_attempts WHERE finding_id = ? ORDER BY created_at DESC",
    )
    .bind(finding_id)
    .fetch_all(pool)
    .await?;

    parse_removal_attempts_from_rows(rows)
}

/// Update the status of a removal attempt.
///
/// Updates the status field and optionally updates timestamp fields.
///
/// # Errors
/// Returns `sqlx::Error` if the database update fails.
pub async fn update_status(
    pool: &Pool<Sqlite>,
    id: &str,
    new_status: RemovalStatus,
    submitted_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE removal_attempts
         SET status = ?, submitted_at = ?, completed_at = ?, error_message = ?
         WHERE id = ?",
    )
    .bind(new_status.to_string())
    .bind(submitted_at.map(|dt| dt.to_rfc3339()))
    .bind(completed_at.map(|dt| dt.to_rfc3339()))
    .bind(error_message)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get a removal attempt by its ID.
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn get_by_id(
    pool: &Pool<Sqlite>,
    id: &str,
) -> Result<Option<RemovalAttempt>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, finding_id, broker_id, status, created_at, submitted_at, completed_at, error_message
         FROM removal_attempts WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let rows = vec![row];
            let mut attempts = parse_removal_attempts_from_rows(rows)?;
            Ok(attempts.pop())
        }
        None => Ok(None),
    }
}

/// Parse database rows into `RemovalAttempt` structs.
///
/// Helper function to avoid code duplication across query functions.
fn parse_removal_attempts_from_rows(
    rows: Vec<sqlx::sqlite::SqliteRow>,
) -> Result<Vec<RemovalAttempt>, sqlx::Error> {
    rows.into_iter()
        .map(|row| -> Result<RemovalAttempt, sqlx::Error> {
            let status_str: String = row.get("status"); // nosemgrep: use-zeroize-for-secrets
            let status = match status_str.as_str() {
                "Submitted" => RemovalStatus::Submitted,
                "Completed" => RemovalStatus::Completed,
                "Failed" => RemovalStatus::Failed,
                _ => RemovalStatus::Pending,
            };

            let created_at_str: String = row.get("created_at"); // nosemgrep: use-zeroize-for-secrets
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                .with_timezone(&Utc);

            let submitted_at = row
                .try_get::<Option<String>, _>("submitted_at")
                .ok()
                .flatten()
                .and_then(|s: String| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&Utc));

            let completed_at = row
                .try_get::<Option<String>, _>("completed_at")
                .ok()
                .flatten()
                .and_then(|s: String| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&Utc));

            Ok(RemovalAttempt {
                id: row.get("id"),
                finding_id: row.get("finding_id"),
                broker_id: row.get("broker_id"),
                status,
                created_at,
                submitted_at,
                completed_at,
                error_message: row.try_get("error_message").ok().flatten(),
            })
        })
        .collect()
}

/// Get all removal attempts in the CAPTCHA queue.
///
/// Returns removal attempts that are pending and require CAPTCHA resolution,
/// ordered by oldest first (`created_at` ASC) for FIFO processing.
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn get_captcha_queue(pool: &Pool<Sqlite>) -> Result<Vec<RemovalAttempt>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, finding_id, broker_id, status, created_at, submitted_at, completed_at, error_message
         FROM removal_attempts
         WHERE status = 'Pending' AND error_message LIKE 'CAPTCHA_REQUIRED%'
         ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    parse_removal_attempts_from_rows(rows)
}

/// Get all removal attempts in the failed queue.
///
/// Returns removal attempts that have failed and may need manual intervention,
/// ordered by newest first (`created_at` DESC) to prioritize recent failures.
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn get_failed_queue(pool: &Pool<Sqlite>) -> Result<Vec<RemovalAttempt>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, finding_id, broker_id, status, created_at, submitted_at, completed_at, error_message
         FROM removal_attempts
         WHERE status = 'Failed'
         ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    parse_removal_attempts_from_rows(rows)
}

/// Summary of removal attempts grouped by scan job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovalJobSummary {
    /// ID of the scan job these removal attempts belong to
    pub scan_job_id: String,
    /// When the earliest removal attempt was created (RFC3339)
    pub submitted_at: String,
    /// Total number of removal attempts for this scan job
    pub total: i64,
    /// Number of attempts with status "Submitted"
    pub submitted_count: i64,
    /// Number of attempts with status "Completed"
    pub completed_count: i64,
    /// Number of attempts with status "Failed"
    pub failed_count: i64,
    /// Number of attempts with status "Pending"
    pub pending_count: i64,
}

/// Get job history: removal attempts grouped by scan job, newest first.
///
/// Returns one summary row per scan job that has at least one removal attempt.
/// Joins through `findings` → `broker_scans` to resolve the `scan_job_id`.
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn get_job_history(pool: &Pool<Sqlite>) -> Result<Vec<RemovalJobSummary>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT
            bs.scan_job_id AS scan_job_id,
            MIN(ra.created_at) AS submitted_at,
            COUNT(*) AS total,
            SUM(CASE WHEN ra.status = 'Submitted' THEN 1 ELSE 0 END) AS submitted_count,
            SUM(CASE WHEN ra.status = 'Completed' THEN 1 ELSE 0 END) AS completed_count,
            SUM(CASE WHEN ra.status = 'Failed' THEN 1 ELSE 0 END) AS failed_count,
            SUM(CASE WHEN ra.status = 'Pending' THEN 1 ELSE 0 END) AS pending_count
        FROM removal_attempts ra
        JOIN findings f ON ra.finding_id = f.id
        JOIN broker_scans bs ON f.broker_scan_id = bs.id
        GROUP BY bs.scan_job_id
        ORDER BY MIN(ra.created_at) DESC",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| -> Result<RemovalJobSummary, sqlx::Error> {
            Ok(RemovalJobSummary {
                scan_job_id: row.get("scan_job_id"),
                submitted_at: row.get("submitted_at"),
                total: row.get("total"),
                submitted_count: row.get("submitted_count"),
                completed_count: row.get("completed_count"),
                failed_count: row.get("failed_count"),
                pending_count: row.get("pending_count"),
            })
        })
        .collect()
}

/// Get all removal attempts for a scan job (via findings table).
pub async fn get_by_scan_job_id(
    pool: &Pool<Sqlite>,
    scan_job_id: &str,
) -> Result<Vec<RemovalAttempt>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT ra.id, ra.finding_id, ra.broker_id, ra.status, ra.created_at,
                ra.submitted_at, ra.completed_at, ra.error_message
         FROM removal_attempts ra
         INNER JOIN findings f ON ra.finding_id = f.id
         WHERE f.broker_scan_id IN (
           SELECT id FROM broker_scans WHERE scan_job_id = ?
         )
         ORDER BY ra.created_at ASC",
    )
    .bind(scan_job_id)
    .fetch_all(pool)
    .await?;

    parse_removal_attempts_from_rows(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use chrono::Utc;

    #[tokio::test]
    async fn test_get_job_history() {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create database");
        db.run_migrations().await.expect("run migrations");
        let pool = db.pool();

        // Insert profile (required by foreign key)
        let dummy_data = [0u8; 32];
        let dummy_nonce = [0u8; 12];
        sqlx::query(
            "INSERT INTO profiles (id, data, nonce, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind("prof-1")
        .bind(&dummy_data[..])
        .bind(&dummy_nonce[..])
        .bind("2026-01-01T00:00:00Z")
        .bind("2026-01-01T00:00:00Z")
        .execute(pool)
        .await
        .expect("insert profile");

        // Insert scan jobs
        sqlx::query(
            "INSERT INTO scan_jobs (id, profile_id, started_at, status, total_brokers, completed_brokers) VALUES (?, ?, ?, ?, ?, ?), (?, ?, ?, ?, ?, ?)",
        )
        .bind("job-a").bind("prof-1").bind("2026-01-01T00:00:00Z").bind("Completed").bind(2).bind(2)
        .bind("job-b").bind("prof-1").bind("2026-01-02T00:00:00Z").bind("Completed").bind(1).bind(1)
        .execute(pool)
        .await
        .expect("insert scan jobs");

        // Insert broker scans (findings link to these, not directly to scan_jobs)
        sqlx::query(
            "INSERT INTO broker_scans (id, scan_job_id, broker_id, status, started_at) VALUES (?, ?, ?, ?, ?), (?, ?, ?, ?, ?), (?, ?, ?, ?, ?)",
        )
        .bind("bscan-1").bind("job-a").bind("spokeo").bind("Success").bind("2026-01-01T00:00:00Z")
        .bind("bscan-2").bind("job-a").bind("whitepages").bind("Success").bind("2026-01-01T00:00:00Z")
        .bind("bscan-3").bind("job-b").bind("radaris").bind("Success").bind("2026-01-02T00:00:00Z")
        .execute(pool)
        .await
        .expect("insert broker scans");

        // Insert findings linked to broker scans
        sqlx::query(
            "INSERT INTO findings (id, broker_scan_id, broker_id, profile_id, listing_url, verification_status, extracted_data, discovered_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?), (?, ?, ?, ?, ?, ?, ?, ?), (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind("find-1").bind("bscan-1").bind("spokeo").bind("prof-1").bind("https://spokeo.com/1").bind("Confirmed").bind("{}").bind("2026-01-01T01:00:00Z")
        .bind("find-2").bind("bscan-2").bind("whitepages").bind("prof-1").bind("https://whitepages.com/2").bind("Confirmed").bind("{}").bind("2026-01-01T01:00:00Z")
        .bind("find-3").bind("bscan-3").bind("radaris").bind("prof-1").bind("https://radaris.com/3").bind("Confirmed").bind("{}").bind("2026-01-02T01:00:00Z")
        .execute(pool)
        .await
        .expect("insert findings");

        // Insert removal attempts linked to findings
        sqlx::query(
            "INSERT INTO removal_attempts (id, finding_id, broker_id, status, created_at) VALUES (?, ?, ?, ?, ?), (?, ?, ?, ?, ?), (?, ?, ?, ?, ?)",
        )
        .bind("att-1").bind("find-1").bind("spokeo").bind("Submitted").bind("2026-01-01T02:00:00Z")
        .bind("att-2").bind("find-2").bind("whitepages").bind("Failed").bind("2026-01-01T02:00:00Z")
        .bind("att-3").bind("find-3").bind("radaris").bind("Completed").bind("2026-01-02T02:00:00Z")
        .execute(pool)
        .await
        .expect("insert removal attempts");

        // nosemgrep: no-unwrap-in-production
        let history = get_job_history(pool).await.expect("get job history");
        assert_eq!(history.len(), 2);
        let job_a = history
            .iter()
            .find(|h| h.scan_job_id == "job-a")
            .expect("job-a not found");
        assert_eq!(job_a.total, 2);
        assert_eq!(job_a.submitted_count, 1);
        assert_eq!(job_a.failed_count, 1);
        // Newest first
        assert_eq!(history[0].scan_job_id, "job-b");
    }

    async fn setup_test_db() -> Database {
        let key = vec![0u8; 32];
        // nosemgrep: no-unwrap-in-production
        let db = Database::new(":memory:", key).await.unwrap();
        // nosemgrep: no-unwrap-in-production
        db.run_migrations().await.unwrap();

        // Create test profile (required by foreign key)
        let dummy_data = [0u8; 32];
        let dummy_nonce = [0u8; 12];
        // nosemgrep: no-unwrap-in-production
        sqlx::query(
            "INSERT INTO profiles (id, data, nonce, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind("profile-123")
        .bind(&dummy_data[..])
        .bind(&dummy_nonce[..])
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(db.pool())
        .await
        .expect("update status");

        // Create test scan job
        // nosemgrep: no-unwrap-in-production
        sqlx::query(
            "INSERT INTO scan_jobs (id, profile_id, started_at, status, total_brokers, completed_brokers) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind("job-456")
        .bind("profile-123")
        .bind(Utc::now().to_rfc3339())
        .bind("InProgress")
        .bind(5)
        .bind(0)
        .execute(db.pool())
        .await
        .expect("update status");

        // Create test broker scan
        // nosemgrep: no-unwrap-in-production
        sqlx::query(
            "INSERT INTO broker_scans (id, scan_job_id, broker_id, status, started_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind("scan-789")
        .bind("job-456")
        .bind("spokeo")
        .bind("Success")
        .bind(Utc::now().to_rfc3339())
        .execute(db.pool())
        .await
        .expect("update status");

        // Create test finding
        // nosemgrep: no-unwrap-in-production
        sqlx::query(
            "INSERT INTO findings (id, broker_scan_id, broker_id, profile_id, listing_url, verification_status, extracted_data, discovered_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind("finding-123")
        .bind("scan-789")
        .bind("spokeo")
        .bind("profile-123")
        .bind("https://example.com/123")
        .bind("Confirmed")
        .bind("{}")
        .bind(Utc::now().to_rfc3339())
        .execute(db.pool())
        .await
        .expect("update status");

        db
    }

    #[tokio::test]
    async fn test_create_removal_attempt() {
        let db = setup_test_db().await;

        let result =
            create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-1".to_string())
                .await;

        assert!(result.is_ok());
        let attempt = result.expect("create attempt");
        assert_eq!(attempt.finding_id, "finding-123");
        assert_eq!(attempt.broker_id, "broker-1");
        assert_eq!(attempt.status, RemovalStatus::Pending);
        assert!(attempt.submitted_at.is_none());
        assert!(attempt.completed_at.is_none());
        assert!(attempt.error_message.is_none());

        // Verify finding is linked to removal attempt
        let finding: Option<String> =
            sqlx::query_scalar("SELECT removal_attempt_id FROM findings WHERE id = ?")
                .bind("finding-123")
                .fetch_optional(db.pool())
                .await
                .expect("fetch finding");
        assert_eq!(finding, Some(attempt.id.clone()));
    }

    #[tokio::test]
    async fn test_get_by_finding_id() {
        let db = setup_test_db().await;

        // Create 2 removal attempts for same finding
        // nosemgrep: no-unwrap-in-production
        create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-1".to_string())
            .await
            .expect("update status");

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // nosemgrep: no-unwrap-in-production
        create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-1".to_string())
            .await
            .expect("update status");

        let attempts = get_by_finding_id(db.pool(), "finding-123")
            .await
            .expect("get by finding id");

        assert_eq!(attempts.len(), 2);
        // Verify ordered by created_at DESC (newest first)
        assert!(attempts[0].created_at >= attempts[1].created_at);
    }

    #[tokio::test]
    async fn test_get_by_finding_id_returns_empty() {
        let db = setup_test_db().await;

        // nosemgrep: no-unwrap-in-production
        let attempts = get_by_finding_id(db.pool(), "non-existent-finding")
            .await
            .expect("update status");

        assert_eq!(attempts.len(), 0);
    }

    #[tokio::test]
    async fn test_update_status_to_submitted() {
        let db = setup_test_db().await;

        let attempt =
            create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-1".to_string())
                .await
                .expect("create removal attempt");

        let submitted_timestamp = Utc::now();
        update_status(
            db.pool(),
            &attempt.id,
            RemovalStatus::Submitted,
            Some(submitted_timestamp),
            None,
            None,
        )
        .await
        .expect("update status");

        // Verify update
        let updated = get_by_id(db.pool(), &attempt.id)
            .await
            .expect("get by id")
            .expect("found attempt");
        assert_eq!(updated.status, RemovalStatus::Submitted);
        assert!(updated.submitted_at.is_some());
        // Verify timestamps are close (within 1 second)
        let diff = (updated.submitted_at.expect("submitted_at") - submitted_timestamp)
            .num_seconds()
            .abs();
        assert!(diff < 1);
        assert!(updated.completed_at.is_none());
        assert!(updated.error_message.is_none());
    }

    #[tokio::test]
    async fn test_update_status_to_completed() {
        let db = setup_test_db().await;

        let attempt =
            create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-1".to_string())
                .await
                .expect("create removal attempt");

        let completed_timestamp = Utc::now();
        update_status(
            db.pool(),
            &attempt.id,
            RemovalStatus::Completed,
            None,
            Some(completed_timestamp),
            None,
        )
        .await
        .expect("update status");

        // Verify update
        let updated = get_by_id(db.pool(), &attempt.id)
            .await
            .expect("get by id")
            .expect("found attempt");
        assert_eq!(updated.status, RemovalStatus::Completed);
        assert!(updated.completed_at.is_some());
        // Verify timestamps are close (within 1 second)
        let diff = (updated.completed_at.expect("completed_at") - completed_timestamp)
            .num_seconds()
            .abs();
        assert!(diff < 1);
    }

    #[tokio::test]
    async fn test_update_status_to_failed_with_error() {
        let db = setup_test_db().await;

        let attempt =
            create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-1".to_string())
                .await
                .expect("create removal attempt");

        update_status(
            db.pool(),
            &attempt.id,
            RemovalStatus::Failed,
            None,
            None,
            Some("Network timeout".to_string()),
        )
        .await
        .expect("update status");

        // Verify update
        let updated = get_by_id(db.pool(), &attempt.id)
            .await
            .expect("get by id")
            .expect("found attempt");
        assert_eq!(updated.status, RemovalStatus::Failed);
        assert_eq!(updated.error_message, Some("Network timeout".to_string()));
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let db = setup_test_db().await;

        // Create removal attempt
        let attempt =
            create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-1".to_string())
                .await
                .expect("create removal attempt");

        // Get by ID - should find it
        let found = get_by_id(db.pool(), &attempt.id).await.expect("get by id");
        assert!(found.is_some());
        let found_attempt = found.expect("found attempt");
        assert_eq!(found_attempt.id, attempt.id);
        assert_eq!(found_attempt.finding_id, "finding-123");

        // Get by non-existent ID - should return None
        let not_found = get_by_id(db.pool(), "non-existent-id")
            .await
            .expect("get by id");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_get_captcha_queue() {
        let db = setup_test_db().await;

        // Create 3 removal attempts
        let attempt1 =
            create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-1".to_string())
                .await
                .expect("create removal attempt 1");

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let attempt2 =
            create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-2".to_string())
                .await
                .expect("create removal attempt 2");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let attempt3 =
            create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-3".to_string())
                .await
                .expect("create removal attempt 3");

        // Mark attempt1 and attempt2 with CAPTCHA errors
        update_status(
            db.pool(),
            &attempt1.id,
            RemovalStatus::Pending,
            None,
            None,
            Some("CAPTCHA_REQUIRED: reCAPTCHA v2 detected".to_string()),
        )
        .await
        .expect("update status 1");

        update_status(
            db.pool(),
            &attempt2.id,
            RemovalStatus::Pending,
            None,
            None,
            Some("CAPTCHA_REQUIRED: hCaptcha detected".to_string()),
        )
        .await
        .expect("update status 2");

        // Mark attempt3 as submitted (no CAPTCHA)
        update_status(
            db.pool(),
            &attempt3.id,
            RemovalStatus::Submitted,
            Some(Utc::now()),
            None,
            None,
        )
        .await
        .expect("update status 3");

        // Query CAPTCHA queue - should return only attempt1 and attempt2
        let captcha_queue = get_captcha_queue(db.pool())
            .await
            .expect("get captcha queue");

        assert_eq!(captcha_queue.len(), 2);
        // Verify ordered by created_at ASC (oldest first)
        assert_eq!(captcha_queue[0].id, attempt1.id);
        assert_eq!(captcha_queue[1].id, attempt2.id);
        // Verify both have CAPTCHA error messages
        assert!(captcha_queue[0]
            .error_message
            .as_ref()
            .unwrap()
            .starts_with("CAPTCHA_REQUIRED"));
        assert!(captcha_queue[1]
            .error_message
            .as_ref()
            .unwrap()
            .starts_with("CAPTCHA_REQUIRED"));
    }

    #[tokio::test]
    async fn test_get_failed_queue() {
        let db = setup_test_db().await;

        // Create 3 removal attempts
        let attempt1 =
            create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-1".to_string())
                .await
                .expect("create removal attempt 1");

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let attempt2 =
            create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-2".to_string())
                .await
                .expect("create removal attempt 2");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let attempt3 =
            create_removal_attempt(db.pool(), "finding-123".to_string(), "broker-3".to_string())
                .await
                .expect("create removal attempt 3");

        // Mark attempt1 and attempt2 as failed
        update_status(
            db.pool(),
            &attempt1.id,
            RemovalStatus::Failed,
            None,
            None,
            Some("Network timeout".to_string()),
        )
        .await
        .expect("update status 1");

        update_status(
            db.pool(),
            &attempt2.id,
            RemovalStatus::Failed,
            None,
            None,
            Some("Invalid form data".to_string()),
        )
        .await
        .expect("update status 2");

        // Mark attempt3 as completed (not failed)
        update_status(
            db.pool(),
            &attempt3.id,
            RemovalStatus::Completed,
            Some(Utc::now()),
            Some(Utc::now()),
            None,
        )
        .await
        .expect("update status 3");

        // Query failed queue - should return only attempt1 and attempt2
        let failed_queue = get_failed_queue(db.pool()).await.expect("get failed queue");

        assert_eq!(failed_queue.len(), 2);
        // Verify ordered by created_at DESC (newest first)
        assert_eq!(failed_queue[0].id, attempt2.id);
        assert_eq!(failed_queue[1].id, attempt1.id);
        // Verify both have failed status
        assert_eq!(failed_queue[0].status, RemovalStatus::Failed);
        assert_eq!(failed_queue[1].status, RemovalStatus::Failed);
        // Verify both have error messages
        assert!(failed_queue[0].error_message.is_some());
        assert!(failed_queue[1].error_message.is_some());
    }
}
