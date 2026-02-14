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

    let attempts = rows
        .into_iter()
        .map(|row| -> Result<RemovalAttempt, sqlx::Error> {
            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "Submitted" => RemovalStatus::Submitted,
                "Completed" => RemovalStatus::Completed,
                "Failed" => RemovalStatus::Failed,
                _ => RemovalStatus::Pending, // Default fallback for "Pending" or unknown
            };

            let created_at_str: String = row.get("created_at");
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
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

    Ok(attempts)
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

    let attempt = row
        .map(|row| -> Result<RemovalAttempt, sqlx::Error> {
            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "Submitted" => RemovalStatus::Submitted,
                "Completed" => RemovalStatus::Completed,
                "Failed" => RemovalStatus::Failed,
                _ => RemovalStatus::Pending, // Default fallback for "Pending" or unknown
            };

            let created_at_str: String = row.get("created_at");
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
        .transpose()?;

    Ok(attempt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use chrono::Utc;

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
}
