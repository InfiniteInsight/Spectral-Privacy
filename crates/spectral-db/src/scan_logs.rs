//! Scan session and log management for PII discovery

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Configuration for PII scan types
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanConfig {
    /// Whether to scan for email addresses
    pub scan_emails: bool,
    /// Whether to scan for phone numbers
    pub scan_phones: bool,
    /// Whether to scan for SSNs
    pub scan_ssn: bool,
    /// Whether to scan for physical addresses
    pub scan_addresses: bool,
    /// Whether to scan for names
    pub scan_names: bool,
    /// Whether to scan for dates of birth
    pub scan_dob: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            scan_emails: true,
            scan_phones: true,
            scan_ssn: true,
            scan_addresses: true,
            scan_names: true,
            scan_dob: true,
        }
    }
}

/// A complete scan session with metadata and results
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanSession {
    /// Session ID
    pub id: String,
    /// Vault ID
    pub vault_id: String,
    /// ISO 8601 start timestamp
    pub started_at: String,
    /// ISO 8601 completion timestamp
    pub completed_at: Option<String>,
    /// Session status (running, completed, stopped, failed)
    pub status: String,
    /// Total number of files scanned
    pub total_files_scanned: i64,
    /// Total number of findings discovered
    pub total_findings: i64,
    /// Scan configuration used
    pub scan_config: ScanConfig,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Create a new scan session in the database
pub async fn create_scan_session(
    pool: &SqlitePool,
    vault_id: &str,
    config: &ScanConfig,
) -> Result<String, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    let config_json = serde_json::to_string(config).unwrap_or_default();

    sqlx::query(
        "INSERT INTO scan_sessions (id, vault_id, started_at, status, scan_config) VALUES (?, ?, ?, 'running', ?)",
    )
    .bind(&id)
    .bind(vault_id)
    .bind(&started_at)
    .bind(&config_json)
    .execute(pool)
    .await?;

    Ok(id)
}

/// Update scan session with progress and status
pub async fn update_scan_session(
    pool: &SqlitePool,
    session_id: &str,
    status: &str,
    files_scanned: i64,
    findings_count: i64,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    let completed_at = if status == "running" {
        None
    } else {
        Some(Utc::now().to_rfc3339())
    };

    sqlx::query(
        "UPDATE scan_sessions SET status = ?, total_files_scanned = ?, total_findings = ?, completed_at = ?, error_message = ? WHERE id = ?",
    )
    .bind(status)
    .bind(files_scanned)
    .bind(findings_count)
    .bind(completed_at)
    .bind(error_message)
    .bind(session_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Log a batch of scanned files to the database
pub async fn log_scanned_files_batch(
    pool: &SqlitePool,
    session_id: &str,
    files: &[(String, bool)],
) -> Result<(), sqlx::Error> {
    if files.is_empty() {
        return Ok(());
    }

    let scanned_at = Utc::now().to_rfc3339();

    for (path, had_findings) in files {
        sqlx::query(
            "INSERT INTO scan_logs (session_id, file_path, scanned_at, had_findings) VALUES (?, ?, ?, ?)",
        )
        .bind(session_id)
        .bind(path)
        .bind(&scanned_at)
        .bind(i32::from(*had_findings))
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Retrieve scan log entries for a session
pub async fn get_scan_log(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<Vec<(String, String, bool)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, i32)>(
        "SELECT file_path, scanned_at, had_findings FROM scan_logs WHERE session_id = ? ORDER BY id ASC",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(p, t, f)| (p, t, f != 0)).collect())
}

/// Get the most recent scan session for a vault
pub async fn get_latest_scan_session(
    pool: &SqlitePool,
    vault_id: &str,
) -> Result<Option<ScanSession>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, Option<String>, String, i64, i64, String, Option<String>)>(
        "SELECT id, vault_id, started_at, completed_at, status, total_files_scanned, total_findings, scan_config, error_message FROM scan_sessions WHERE vault_id = ? ORDER BY started_at DESC LIMIT 1",
    )
    .bind(vault_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(
        |(id, vault_id, started_at, completed_at, status, files, findings, config_json, error)| {
            ScanSession {
                id,
                vault_id,
                started_at,
                completed_at,
                status,
                total_files_scanned: files,
                total_findings: findings,
                scan_config: serde_json::from_str(&config_json).unwrap_or_default(),
                error_message: error,
            }
        },
    ))
}
