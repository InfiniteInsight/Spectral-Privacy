//! Removal attempts operations for tracking removal request submissions.
//!
//! This module provides CRUD operations for the `removal_attempts` table,
//! which stores removal request submissions for confirmed findings.

use sqlx::{Pool, Sqlite};
use uuid::Uuid;

/// A removal attempt represents a removal request submission to a data broker.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RemovalAttempt {
    /// Unique identifier
    pub id: String,
    /// ID of the finding being removed
    pub finding_id: String,
    /// ID of the broker
    pub broker_id: String,
    /// Status of the removal attempt
    pub status: String,
    /// When the attempt was created (ISO 8601 timestamp)
    pub created_at: String,
    /// When the request was submitted (if submitted)
    pub submitted_at: Option<String>,
    /// When the removal was completed (if completed)
    pub completed_at: Option<String>,
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
    let created_at = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO removal_attempts (id, finding_id, broker_id, status, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&finding_id)
    .bind(&broker_id)
    .bind("Pending")
    .bind(&created_at)
    .execute(pool)
    .await?;

    Ok(RemovalAttempt {
        id,
        finding_id,
        broker_id,
        status: "Pending".to_string(),
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
    let attempts = sqlx::query_as::<_, RemovalAttempt>(
        "SELECT * FROM removal_attempts WHERE finding_id = ? ORDER BY created_at DESC",
    )
    .bind(finding_id)
    .fetch_all(pool)
    .await?;

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
    new_status: &str,
    submitted_at: Option<String>,
    completed_at: Option<String>,
    error_message: Option<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE removal_attempts
         SET status = ?, submitted_at = ?, completed_at = ?, error_message = ?
         WHERE id = ?",
    )
    .bind(new_status)
    .bind(submitted_at)
    .bind(completed_at)
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
    let attempt =
        sqlx::query_as::<_, RemovalAttempt>("SELECT * FROM removal_attempts WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

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
        assert_eq!(attempt.status, "Pending");
        assert!(!attempt.created_at.is_empty());
        assert!(attempt.submitted_at.is_none());
        assert!(attempt.completed_at.is_none());
        assert!(attempt.error_message.is_none());
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

        let submitted_timestamp = chrono::Utc::now().to_rfc3339();
        update_status(
            db.pool(),
            &attempt.id,
            "Submitted",
            Some(submitted_timestamp.clone()),
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
        assert_eq!(updated.status, "Submitted");
        assert_eq!(updated.submitted_at, Some(submitted_timestamp));
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

        let completed_timestamp = chrono::Utc::now().to_rfc3339();
        update_status(
            db.pool(),
            &attempt.id,
            "Completed",
            None,
            Some(completed_timestamp.clone()),
            None,
        )
        .await
        .expect("update status");

        // Verify update
        let updated = get_by_id(db.pool(), &attempt.id)
            .await
            .expect("get by id")
            .expect("found attempt");
        assert_eq!(updated.status, "Completed");
        assert_eq!(updated.completed_at, Some(completed_timestamp));
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
            "Failed",
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
        assert_eq!(updated.status, "Failed");
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
