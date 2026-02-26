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

/// Unified scan history entry for all scan types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "scan_type")]
pub enum ScanHistoryEntry {
    /// Data broker scan entry
    #[serde(rename = "DataBroker")]
    DataBroker {
        /// Scan job ID
        id: String,
        /// Profile ID being scanned
        profile_id: String,
        /// When the scan started
        started_at: DateTime<Utc>,
        /// When the scan completed
        completed_at: Option<DateTime<Utc>>,
        /// Current status
        status: ScanJobStatus,
        /// Total brokers to scan
        total_brokers: u32,
        /// Brokers completed
        completed_brokers: u32,
        /// Total findings discovered
        total_findings: u32,
        /// Findings marked as confirmed
        confirmed_findings: u32,
        /// Findings marked as rejected
        rejected_findings: u32,
        /// Number of removal requests created
        removal_requests: u32,
        /// Error message if failed
        error_message: Option<String>,
    },
    /// Cookie scan entry
    #[serde(rename = "Cookie")]
    Cookie {
        /// Scan ID
        id: String,
        /// Vault ID
        vault_id: String,
        /// When the scan started
        started_at: DateTime<Utc>,
        /// When the scan completed
        completed_at: Option<DateTime<Utc>>,
        /// Scan status
        status: String,
        /// Total cookies found
        total_cookies_found: u32,
        /// Cookies matched to brokers
        matched_cookies: u32,
        /// Browsers scanned
        browsers_scanned: Vec<String>,
        /// Brokers matched
        brokers_matched: Vec<String>,
        /// Error message if failed
        error_message: Option<String>,
    },
    /// Local PII discovery entry
    #[serde(rename = "Discovery")]
    Discovery {
        /// Vault ID
        vault_id: String,
        /// When findings were first discovered
        started_at: DateTime<Utc>,
        /// Total findings
        total_findings: u32,
        /// Critical risk findings
        critical_findings: u32,
        /// Medium risk findings
        medium_findings: u32,
        /// Informational findings
        informational_findings: u32,
    },
}

/// Scan job history with statistics (deprecated - use `ScanHistoryEntry`).
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

/// Get unified scan history for a vault (all scan types).
///
/// # Errors
/// Returns an error if the database operation fails.
#[allow(clippy::too_many_lines)]
pub async fn get_unified_scan_history(
    pool: &SqlitePool,
    vault_id: &str,
    profile_id: Option<&str>,
) -> Result<Vec<ScanHistoryEntry>, sqlx::Error> {
    let mut entries = Vec::new();

    // Get broker scans
    if let Some(pid) = profile_id {
        let broker_rows = sqlx::query(
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
        .bind(pid)
        .fetch_all(pool)
        .await?;

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        for row in broker_rows {
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

            entries.push(ScanHistoryEntry::DataBroker {
                id: row.get("id"),
                profile_id: row.get("profile_id"),
                started_at,
                completed_at,
                status,
                total_brokers: total_brokers as u32,
                completed_brokers: completed_brokers as u32,
                total_findings: total_findings as u32,
                confirmed_findings: confirmed_findings as u32,
                rejected_findings: rejected_findings as u32,
                removal_requests: removal_requests as u32,
                error_message: row.get("error_message"),
            });
        }
    }

    // Get cookie scans
    let cookie_rows = sqlx::query(
        "SELECT id, vault_id, scan_timestamp, completed_at, scan_status,
                total_cookies_found, matched_cookies, browsers_scanned, brokers_matched, error_message
         FROM cookie_scans
         WHERE vault_id = ?
         ORDER BY scan_timestamp DESC"
    )
    .bind(vault_id)
    .fetch_all(pool)
    .await?;

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    for row in cookie_rows {
        // nosemgrep: use-zeroize-for-secrets
        let started_at_str: String = row.get("scan_timestamp");
        let started_at = DateTime::parse_from_rfc3339(&started_at_str)
            .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

        let completed_at_str: Option<String> = row.get("completed_at");
        let completed_at = completed_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        });

        // nosemgrep: use-zeroize-for-secrets
        let browsers_str: String = row.get("browsers_scanned");
        let browsers_scanned: Vec<String> = serde_json::from_str(&browsers_str).unwrap_or_default();

        let brokers_str: Option<String> = row.get("brokers_matched");
        let brokers_matched: Vec<String> = brokers_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let total_cookies: i64 = row.get("total_cookies_found");
        let matched: i64 = row.get("matched_cookies");

        entries.push(ScanHistoryEntry::Cookie {
            id: row.get("id"),
            vault_id: row.get("vault_id"),
            started_at,
            completed_at,
            status: row.get("scan_status"),
            total_cookies_found: total_cookies as u32,
            matched_cookies: matched as u32,
            browsers_scanned,
            brokers_matched,
            error_message: row.get("error_message"),
        });
    }

    // Get discovery findings (aggregate by day since there's no scan session tracking)
    let discovery_rows = sqlx::query(
        "SELECT
            vault_id,
            DATE(found_at) as scan_date,
            COUNT(*) as total_findings,
            COUNT(CASE WHEN risk_level = 'critical' THEN 1 END) as critical_findings,
            COUNT(CASE WHEN risk_level = 'medium' THEN 1 END) as medium_findings,
            COUNT(CASE WHEN risk_level = 'informational' THEN 1 END) as informational_findings,
            MIN(found_at) as first_found
         FROM discovery_findings
         WHERE vault_id = ? AND remediated = 0
         GROUP BY vault_id, DATE(found_at)
         ORDER BY first_found DESC",
    )
    .bind(vault_id)
    .fetch_all(pool)
    .await?;

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    for row in discovery_rows {
        // nosemgrep: use-zeroize-for-secrets
        let first_found_str: String = row.get("first_found");
        let started_at = DateTime::parse_from_rfc3339(&first_found_str)
            .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

        let total: i64 = row.get("total_findings");
        let critical: i64 = row.get("critical_findings");
        let medium: i64 = row.get("medium_findings");
        let informational: i64 = row.get("informational_findings");

        entries.push(ScanHistoryEntry::Discovery {
            vault_id: row.get("vault_id"),
            started_at,
            total_findings: total as u32,
            critical_findings: critical as u32,
            medium_findings: medium as u32,
            informational_findings: informational as u32,
        });
    }

    // Sort all entries by timestamp descending
    entries.sort_by(|a, b| {
        let time_a = match a {
            ScanHistoryEntry::DataBroker { started_at, .. }
            | ScanHistoryEntry::Cookie { started_at, .. }
            | ScanHistoryEntry::Discovery { started_at, .. } => started_at,
        };
        let time_b = match b {
            ScanHistoryEntry::DataBroker { started_at, .. }
            | ScanHistoryEntry::Cookie { started_at, .. }
            | ScanHistoryEntry::Discovery { started_at, .. } => started_at,
        };
        time_b.cmp(time_a)
    });

    Ok(entries)
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
