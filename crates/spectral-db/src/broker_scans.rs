//! Broker scan operations for tracking individual broker scan progress.
//!
//! This module provides CRUD operations for the `broker_scans` table,
//! which tracks the status and results of scanning individual brokers
//! as part of a larger scan job.

use chrono::Utc;
use sqlx::{Pool, Row, Sqlite};

/// A record representing an individual broker scan within a scan job.
#[derive(Debug, Clone)]
pub struct BrokerScan {
    /// Unique identifier for this broker scan
    pub id: String,
    /// ID of the parent scan job
    pub scan_job_id: String,
    /// ID of the broker being scanned
    pub broker_id: String,
    /// Current status (Pending, Success, Failed, Skipped)
    pub status: String,
    /// When the scan started (RFC3339 timestamp)
    pub started_at: Option<String>,
    /// When the scan completed (RFC3339 timestamp)
    pub completed_at: Option<String>,
    /// Error message if scan failed
    pub error_message: Option<String>,
    /// Number of findings discovered in this scan
    pub findings_count: i64,
}

/// Create a new broker scan record.
///
/// # Errors
/// Returns `sqlx::Error` if the database insert fails.
pub async fn create_broker_scan(
    pool: &Pool<Sqlite>,
    scan_job_id: String,
    broker_id: String,
) -> Result<BrokerScan, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO broker_scans (id, scan_job_id, broker_id, status, started_at) VALUES (?, ?, ?, 'Pending', ?)"
    )
    .bind(&id)
    .bind(&scan_job_id)
    .bind(&broker_id)
    .bind(&started_at)
    .execute(pool)
    .await?;

    Ok(BrokerScan {
        id: id.clone(),
        scan_job_id,
        broker_id,
        status: "Pending".to_string(),
        started_at: Some(started_at),
        completed_at: None,
        error_message: None,
        findings_count: 0,
    })
}

/// Update the status of a broker scan.
///
/// Sets the status, completion time, and optional error message.
///
/// # Errors
/// Returns `sqlx::Error` if the database update fails.
pub async fn update_status(
    pool: &Pool<Sqlite>,
    id: &str,
    status: &str,
    error_message: Option<String>,
) -> Result<(), sqlx::Error> {
    let completed_at = Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE broker_scans SET status = ?, completed_at = ?, error_message = ? WHERE id = ?",
    )
    .bind(status)
    .bind(&completed_at)
    .bind(error_message)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get all broker scans for a specific scan job.
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn get_by_scan_job(
    pool: &Pool<Sqlite>,
    scan_job_id: &str,
) -> Result<Vec<BrokerScan>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, scan_job_id, broker_id, status, started_at, completed_at, error_message, findings_count FROM broker_scans WHERE scan_job_id = ?"
    )
    .bind(scan_job_id)
    .fetch_all(pool)
    .await?;

    let mut scans = Vec::new();
    for row in rows {
        scans.push(BrokerScan {
            id: row.try_get("id")?,
            scan_job_id: row.try_get("scan_job_id")?,
            broker_id: row.try_get("broker_id")?,
            status: row.try_get("status")?,
            started_at: row.try_get("started_at")?,
            completed_at: row.try_get("completed_at")?,
            error_message: row.try_get("error_message")?,
            findings_count: row.try_get("findings_count")?,
        });
    }

    Ok(scans)
}

/// Get a specific broker scan by ID.
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn get_by_id(pool: &Pool<Sqlite>, id: &str) -> Result<Option<BrokerScan>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, scan_job_id, broker_id, status, started_at, completed_at, error_message, findings_count FROM broker_scans WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(BrokerScan {
            id: r.try_get("id")?,
            scan_job_id: r.try_get("scan_job_id")?,
            broker_id: r.try_get("broker_id")?,
            status: r.try_get("status")?,
            started_at: r.try_get("started_at")?,
            completed_at: r.try_get("completed_at")?,
            error_message: r.try_get("error_message")?,
            findings_count: r.try_get("findings_count")?,
        })),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

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
        .bind("profile-456")
        .bind(&dummy_data[..])
        .bind(&dummy_nonce[..])
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(db.pool())
        .await
        .unwrap();

        // Create test scan job
        // nosemgrep: no-unwrap-in-production
        sqlx::query(
            "INSERT INTO scan_jobs (id, profile_id, started_at, status, total_brokers, completed_brokers) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind("job-123")
        .bind("profile-456")
        .bind("2025-01-01T00:00:00Z")
        .bind("InProgress")
        .bind(5)
        .bind(0)
        .execute(db.pool())
        .await
        .unwrap();

        db
    }

    #[tokio::test]
    async fn test_create_broker_scan() {
        let db = setup_test_db().await;

        let scan = create_broker_scan(db.pool(), "job-123".to_string(), "test-broker".to_string())
            .await
            .expect("create broker scan");

        assert_eq!(scan.scan_job_id, "job-123");
        assert_eq!(scan.broker_id, "test-broker");
        assert_eq!(scan.status, "Pending");
        assert!(scan.started_at.is_some());
    }

    #[tokio::test]
    async fn test_update_status() {
        let db = setup_test_db().await;

        let scan = create_broker_scan(db.pool(), "job-123".to_string(), "test-broker".to_string())
            .await
            .expect("create broker scan");

        update_status(db.pool(), &scan.id, "Success", None)
            .await
            .expect("update status");

        let updated = get_by_id(db.pool(), &scan.id)
            .await
            .expect("get by id")
            .expect("scan exists");

        assert_eq!(updated.status, "Success");
        assert!(updated.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_get_by_scan_job() {
        let db = setup_test_db().await;

        create_broker_scan(db.pool(), "job-123".to_string(), "broker-1".to_string())
            .await
            .expect("create scan 1");

        create_broker_scan(db.pool(), "job-123".to_string(), "broker-2".to_string())
            .await
            .expect("create scan 2");

        let scans = get_by_scan_job(db.pool(), "job-123")
            .await
            .expect("get by scan job");

        assert_eq!(scans.len(), 2);
    }
}
