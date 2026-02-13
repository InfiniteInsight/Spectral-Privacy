//! Findings operations for tracking discovered PII on broker sites.
//!
//! This module provides CRUD operations for the `findings` table,
//! which stores potential matches found during broker scans.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{Pool, Row, Sqlite};

/// A finding represents a potential match found on a data broker site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Unique identifier for this finding
    pub id: String,
    /// ID of the broker scan that discovered this finding
    pub broker_scan_id: String,
    /// ID of the broker where this was found
    pub broker_id: String,
    /// ID of the profile being searched
    pub profile_id: String,
    /// URL of the listing on the broker's site
    pub listing_url: String,
    /// Verification status
    pub verification_status: VerificationStatus,
    /// Extracted data from the listing (JSON)
    pub extracted_data: JsonValue,
    /// When this finding was discovered
    pub discovered_at: DateTime<Utc>,
    /// When this finding was verified (if verified)
    pub verified_at: Option<DateTime<Utc>>,
    /// Whether verified by user (true) or automatically (false)
    pub verified_by_user: Option<bool>,
    /// ID of removal attempt (if removal was attempted)
    pub removal_attempt_id: Option<String>,
}

/// Verification status for a finding.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerificationStatus {
    /// Awaiting user verification
    PendingVerification,
    /// User confirmed this is their information
    Confirmed,
    /// User rejected this as not their information
    Rejected,
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PendingVerification => write!(f, "PendingVerification"),
            Self::Confirmed => write!(f, "Confirmed"),
            Self::Rejected => write!(f, "Rejected"),
        }
    }
}

impl VerificationStatus {
    /// Parse from string representation.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "Confirmed" => Self::Confirmed,
            "Rejected" => Self::Rejected,
            _ => Self::PendingVerification,
        }
    }
}

/// Create a new finding record.
///
/// The finding is created with `PendingVerification` status.
///
/// # Errors
/// Returns `sqlx::Error` if the database insert fails.
pub async fn create_finding(
    pool: &Pool<Sqlite>,
    broker_scan_id: String,
    broker_id: String,
    profile_id: String,
    listing_url: String,
    extracted_data: JsonValue,
) -> Result<Finding, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let discovered_at = Utc::now();
    let status = VerificationStatus::PendingVerification;
    let extracted_json = serde_json::to_string(&extracted_data).unwrap_or_default();

    sqlx::query(
        "INSERT INTO findings (id, broker_scan_id, broker_id, profile_id, listing_url,
                               verification_status, extracted_data, discovered_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&broker_scan_id)
    .bind(&broker_id)
    .bind(&profile_id)
    .bind(&listing_url)
    .bind(status.to_string())
    .bind(&extracted_json)
    .bind(discovered_at.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(Finding {
        id,
        broker_scan_id,
        broker_id,
        profile_id,
        listing_url,
        verification_status: status,
        extracted_data,
        discovered_at,
        verified_at: None,
        verified_by_user: None,
        removal_attempt_id: None,
    })
}

/// Update the verification status of a finding.
///
/// # Errors
/// Returns `sqlx::Error` if the database update fails.
pub async fn update_verification_status(
    pool: &Pool<Sqlite>,
    finding_id: &str,
    status: VerificationStatus,
    verified_by_user: bool,
) -> Result<(), sqlx::Error> {
    let verified_at = Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE findings
         SET verification_status = ?, verified_at = ?, verified_by_user = ?
         WHERE id = ?",
    )
    .bind(status.to_string())
    .bind(&verified_at)
    .bind(verified_by_user)
    .bind(finding_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get all findings for a specific scan job.
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn get_by_scan_job(
    pool: &Pool<Sqlite>,
    scan_job_id: &str,
) -> Result<Vec<Finding>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT f.id, f.broker_scan_id, f.broker_id, f.profile_id, f.listing_url,
                f.verification_status, f.extracted_data, f.discovered_at,
                f.verified_at, f.verified_by_user, f.removal_attempt_id
         FROM findings f
         JOIN broker_scans bs ON f.broker_scan_id = bs.id
         WHERE bs.scan_job_id = ?
         ORDER BY f.discovered_at DESC",
    )
    .bind(scan_job_id)
    .fetch_all(pool)
    .await?;

    parse_findings_from_rows(rows)
}

/// Get findings for a specific broker scan.
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn get_by_broker_scan(
    pool: &Pool<Sqlite>,
    broker_scan_id: &str,
) -> Result<Vec<Finding>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, broker_scan_id, broker_id, profile_id, listing_url,
                verification_status, extracted_data, discovered_at,
                verified_at, verified_by_user, removal_attempt_id
         FROM findings
         WHERE broker_scan_id = ?
         ORDER BY discovered_at DESC",
    )
    .bind(broker_scan_id)
    .fetch_all(pool)
    .await?;

    parse_findings_from_rows(rows)
}

/// Verify a finding (shorthand for updating status to Confirmed).
///
/// # Errors
/// Returns `sqlx::Error` if the database update fails.
pub async fn verify_finding(
    pool: &Pool<Sqlite>,
    finding_id: &str,
    is_confirmed: bool,
    verified_by_user: bool,
) -> Result<(), sqlx::Error> {
    let status = if is_confirmed {
        VerificationStatus::Confirmed
    } else {
        VerificationStatus::Rejected
    };

    update_verification_status(pool, finding_id, status, verified_by_user).await
}

/// Helper function to parse findings from database rows.
fn parse_findings_from_rows(
    rows: Vec<sqlx::sqlite::SqliteRow>,
) -> Result<Vec<Finding>, sqlx::Error> {
    let mut findings = Vec::new();

    for row in rows {
        // These are temporary strings for database deserialization.
        // Actual PII is encrypted at application layer via spectral-vault.
        let extracted_data_str: String = row.try_get("extracted_data")?; // nosemgrep: use-zeroize-for-secrets
        let extracted_data = serde_json::from_str(&extracted_data_str).unwrap_or(JsonValue::Null);

        let discovered_at_str: String = row.try_get("discovered_at")?; // nosemgrep: use-zeroize-for-secrets
        let discovered_at = DateTime::parse_from_rfc3339(&discovered_at_str)
            .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

        let verified_at: Option<String> = row.try_get("verified_at")?;
        let verified_at = verified_at.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        });

        let verified_by_user: Option<i64> = row.try_get("verified_by_user")?;
        let verified_by_user = verified_by_user.map(|v| v != 0);

        let verification_status_str: String = row.try_get("verification_status")?; // nosemgrep: use-zeroize-for-secrets
        let verification_status = VerificationStatus::parse(&verification_status_str);

        findings.push(Finding {
            id: row.try_get("id")?,
            broker_scan_id: row.try_get("broker_scan_id")?,
            broker_id: row.try_get("broker_id")?,
            profile_id: row.try_get("profile_id")?,
            listing_url: row.try_get("listing_url")?,
            verification_status,
            extracted_data,
            discovered_at,
            verified_at,
            verified_by_user,
            removal_attempt_id: row.try_get("removal_attempt_id")?,
        });
    }

    Ok(findings)
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
        .bind("profile-123")
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
        .bind("job-456")
        .bind("profile-123")
        .bind(Utc::now().to_rfc3339())
        .bind("InProgress")
        .bind(5)
        .bind(0)
        .execute(db.pool())
        .await
        .unwrap();

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
        .unwrap();

        db
    }

    #[tokio::test]
    async fn test_create_finding() {
        let db = setup_test_db().await;

        let extracted = serde_json::json!({
            "name": "John Doe",
            "age": 35,
            "location": "Los Angeles, CA"
        });

        let finding = create_finding(
            db.pool(),
            "scan-789".to_string(),
            "spokeo".to_string(),
            "profile-123".to_string(),
            "https://example.com/profile/123".to_string(),
            extracted,
        )
        .await
        .expect("create finding");

        assert_eq!(finding.broker_id, "spokeo");
        assert_eq!(
            finding.verification_status,
            VerificationStatus::PendingVerification
        );
        assert!(finding.verified_at.is_none());
        assert!(finding.verified_by_user.is_none());
    }

    #[tokio::test]
    async fn test_update_verification_status() {
        let db = setup_test_db().await;

        let extracted = serde_json::json!({"name": "Jane Smith"});

        let finding = create_finding(
            db.pool(),
            "scan-789".to_string(),
            "spokeo".to_string(),
            "profile-123".to_string(),
            "https://example.com/profile/456".to_string(),
            extracted,
        )
        .await
        .expect("create finding");

        update_verification_status(db.pool(), &finding.id, VerificationStatus::Confirmed, true)
            .await
            .expect("update status");

        // Verify update
        let findings = get_by_broker_scan(db.pool(), "scan-789")
            .await
            .expect("get findings");

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].verification_status,
            VerificationStatus::Confirmed
        );
        assert!(findings[0].verified_at.is_some());
        assert_eq!(findings[0].verified_by_user, Some(true));
    }

    #[tokio::test]
    async fn test_get_by_scan_job() {
        let db = setup_test_db().await;

        let extracted1 = serde_json::json!({"name": "Alice"});
        let extracted2 = serde_json::json!({"name": "Bob"});

        create_finding(
            db.pool(),
            "scan-789".to_string(),
            "spokeo".to_string(),
            "profile-123".to_string(),
            "https://example.com/1".to_string(),
            extracted1,
        )
        .await
        .expect("create finding 1");

        create_finding(
            db.pool(),
            "scan-789".to_string(),
            "spokeo".to_string(),
            "profile-123".to_string(),
            "https://example.com/2".to_string(),
            extracted2,
        )
        .await
        .expect("create finding 2");

        let findings = get_by_scan_job(db.pool(), "job-456")
            .await
            .expect("get by scan job");

        assert_eq!(findings.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_broker_scan() {
        let db = setup_test_db().await;

        let extracted = serde_json::json!({"name": "Charlie"});

        create_finding(
            db.pool(),
            "scan-789".to_string(),
            "spokeo".to_string(),
            "profile-123".to_string(),
            "https://example.com/3".to_string(),
            extracted,
        )
        .await
        .expect("create finding");

        let findings = get_by_broker_scan(db.pool(), "scan-789")
            .await
            .expect("get by broker scan");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].listing_url, "https://example.com/3");
    }

    #[tokio::test]
    async fn test_verify_finding() {
        let db = setup_test_db().await;

        let extracted = serde_json::json!({"name": "David"});

        let finding = create_finding(
            db.pool(),
            "scan-789".to_string(),
            "spokeo".to_string(),
            "profile-123".to_string(),
            "https://example.com/4".to_string(),
            extracted,
        )
        .await
        .expect("create finding");

        // Verify as confirmed
        verify_finding(db.pool(), &finding.id, true, true)
            .await
            .expect("verify finding");

        let findings = get_by_broker_scan(db.pool(), "scan-789")
            .await
            .expect("get findings");

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].verification_status,
            VerificationStatus::Confirmed
        );

        // Verify as rejected
        verify_finding(db.pool(), &finding.id, false, false)
            .await
            .expect("verify finding as rejected");

        let findings = get_by_broker_scan(db.pool(), "scan-789")
            .await
            .expect("get findings");

        assert_eq!(
            findings[0].verification_status,
            VerificationStatus::Rejected
        );
        assert_eq!(findings[0].verified_by_user, Some(false));
    }

    #[tokio::test]
    async fn test_verification_status_display() {
        assert_eq!(
            VerificationStatus::PendingVerification.to_string(),
            "PendingVerification"
        );
        assert_eq!(VerificationStatus::Confirmed.to_string(), "Confirmed");
        assert_eq!(VerificationStatus::Rejected.to_string(), "Rejected");
    }

    #[tokio::test]
    async fn test_verification_status_parse() {
        assert_eq!(
            VerificationStatus::parse("Confirmed"),
            VerificationStatus::Confirmed
        );
        assert_eq!(
            VerificationStatus::parse("Rejected"),
            VerificationStatus::Rejected
        );
        assert_eq!(
            VerificationStatus::parse("unknown"),
            VerificationStatus::PendingVerification
        );
    }
}
