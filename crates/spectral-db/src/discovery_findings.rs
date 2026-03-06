//! Discovery findings operations for tracking local PII exposures.
//!
//! This module provides CRUD operations for the `discovery_findings` table,
//! which stores PII found during local filesystem, browser, and email scans.

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};

/// A discovery finding represents PII found in local data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryFinding {
    /// Unique identifier
    pub id: String,
    /// Vault ID this finding belongs to
    pub vault_id: String,
    /// Source of the finding (filesystem, browser, email)
    pub source: String,
    /// Detailed source information (file path, browser name, etc.)
    pub source_detail: String,
    /// Type of finding (`pii_exposure`, `broker_contact`, `broker_account`)
    pub finding_type: String,
    /// Risk level (critical, medium, informational)
    pub risk_level: String,
    /// Human-readable description
    pub description: String,
    /// Recommended action to take
    pub recommended_action: Option<String>,
    /// PII type (email, phone, ssn, address, etc.)
    pub pii_type: String,
    /// Whether this finding has been remediated (user claims to have fixed it)
    pub remediated: bool,
    /// Whether this finding is ignored (user accepts it as false positive or acceptable)
    pub ignored: bool,
    /// Whether PII is still present despite being marked as remediated
    pub still_present_after_remediation: bool,
    /// ISO 8601 timestamp when found
    pub found_at: String,
    /// The actual matched value (e.g., the phone number or email address)
    pub matched_value: Option<String>,
    /// Line number where the PII was found
    pub line_number: Option<i64>,
}

/// Parameters for creating a discovery finding
#[derive(Debug)]
pub struct CreateDiscoveryFinding {
    /// Vault ID
    pub vault_id: String,
    /// Source type
    pub source: String,
    /// Source detail
    pub source_detail: String,
    /// Finding type
    pub finding_type: String,
    /// Risk level
    pub risk_level: String,
    /// Description
    pub description: String,
    /// Recommended action
    pub recommended_action: Option<String>,
    /// PII type
    pub pii_type: String,
    /// The actual matched value
    pub matched_value: Option<String>,
    /// Line number where found
    pub line_number: Option<usize>,
}

/// Check if a finding already exists (to prevent duplicates)
///
/// Returns the existing finding if one matches the `vault_id`, `source_detail`, and `pii_type`.
async fn find_existing_finding(
    pool: &Pool<Sqlite>,
    vault_id: &str,
    source_detail: &str,
    pii_type: &str,
) -> Result<Option<DiscoveryFinding>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, vault_id, source, source_detail, finding_type, risk_level, description, recommended_action, pii_type, remediated, ignored, still_present_after_remediation, found_at, matched_value, line_number
         FROM discovery_findings
         WHERE vault_id = ? AND source_detail = ? AND pii_type = ?
         LIMIT 1",
    )
    .bind(vault_id)
    .bind(source_detail)
    .bind(pii_type)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| DiscoveryFinding {
        id: row.get("id"),
        vault_id: row.get("vault_id"),
        source: row.get("source"),
        source_detail: row.get("source_detail"),
        finding_type: row.get("finding_type"),
        risk_level: row.get("risk_level"),
        description: row.get("description"),
        recommended_action: row.get("recommended_action"),
        pii_type: row.get("pii_type"),
        remediated: row.get::<i64, _>("remediated") != 0,
        ignored: row.get::<i64, _>("ignored") != 0,
        still_present_after_remediation: row.get::<i64, _>("still_present_after_remediation") != 0,
        found_at: row.get("found_at"),
        matched_value: row.get("matched_value"),
        line_number: row.get("line_number"),
    }))
}

/// Insert a new discovery finding, or return existing if already present
///
/// This prevents duplicate findings from being created when the same PII
/// is found in the same location across multiple scans.
///
/// Special handling:
/// - If existing finding is ignored: returns it (won't be shown in UI)
/// - If existing finding is remediated: marks it as `still_present_after_remediation`
/// - Otherwise: returns existing finding as-is
///
/// # Errors
/// Returns `sqlx::Error` if the database operation fails.
pub async fn insert_discovery_finding(
    pool: &Pool<Sqlite>,
    params: CreateDiscoveryFinding,
) -> Result<DiscoveryFinding, sqlx::Error> {
    // Check if a finding already exists for this location and PII type
    if let Some(mut existing) = find_existing_finding(
        pool,
        &params.vault_id,
        &params.source_detail,
        &params.pii_type,
    )
    .await?
    {
        // If the existing finding is remediated but still found, mark it
        if existing.remediated && !existing.still_present_after_remediation {
            sqlx::query(
                "UPDATE discovery_findings SET still_present_after_remediation = 1 WHERE id = ?",
            )
            .bind(&existing.id)
            .execute(pool)
            .await?;
            existing.still_present_after_remediation = true;
        }

        // Return the existing finding (whether ignored, remediated, or neither)
        return Ok(existing);
    }

    // Create a new finding if none exists
    let id = uuid::Uuid::new_v4().to_string();
    let found_at = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO discovery_findings (id, vault_id, source, source_detail, finding_type, risk_level, description, recommended_action, pii_type, found_at, matched_value, line_number)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&params.vault_id)
    .bind(&params.source)
    .bind(&params.source_detail)
    .bind(&params.finding_type)
    .bind(&params.risk_level)
    .bind(&params.description)
    .bind(&params.recommended_action)
    .bind(&params.pii_type)
    .bind(&found_at)
    .bind(&params.matched_value)
    .bind(params.line_number.map(|n| i64::try_from(n).unwrap_or(0)))
    .execute(pool)
    .await?;

    Ok(DiscoveryFinding {
        id,
        vault_id: params.vault_id,
        source: params.source,
        source_detail: params.source_detail,
        finding_type: params.finding_type,
        risk_level: params.risk_level,
        description: params.description,
        recommended_action: params.recommended_action,
        pii_type: params.pii_type,
        remediated: false,
        ignored: false,
        still_present_after_remediation: false,
        found_at,
        matched_value: params.matched_value,
        line_number: params.line_number.map(|n| i64::try_from(n).unwrap_or(0)),
    })
}

/// Get all discovery findings for a vault (excluding ignored findings)
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn get_discovery_findings(
    pool: &Pool<Sqlite>,
    vault_id: &str,
) -> Result<Vec<DiscoveryFinding>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, vault_id, source, source_detail, finding_type, risk_level, description, recommended_action, pii_type, remediated, ignored, still_present_after_remediation, found_at, matched_value, line_number
         FROM discovery_findings
         WHERE vault_id = ? AND ignored = 0
         ORDER BY found_at DESC",
    )
    .bind(vault_id)
    .fetch_all(pool)
    .await?;

    let findings = rows
        .into_iter()
        .map(|row| DiscoveryFinding {
            id: row.get("id"),
            vault_id: row.get("vault_id"),
            source: row.get("source"),
            source_detail: row.get("source_detail"),
            finding_type: row.get("finding_type"),
            risk_level: row.get("risk_level"),
            description: row.get("description"),
            recommended_action: row.get("recommended_action"),
            pii_type: row.get("pii_type"),
            remediated: row.get::<i64, _>("remediated") != 0,
            ignored: row.get::<i64, _>("ignored") != 0,
            still_present_after_remediation: row.get::<i64, _>("still_present_after_remediation")
                != 0,
            found_at: row.get("found_at"),
            matched_value: row.get("matched_value"),
            line_number: row.get("line_number"),
        })
        .collect();

    Ok(findings)
}

/// Update the remediated status of a finding
///
/// When marking as remediated, also resets the `still_present_after_remediation` flag
/// since the user is claiming to have fixed the issue.
///
/// # Errors
/// Returns `sqlx::Error` if the database update fails.
pub async fn update_finding_remediated(
    pool: &Pool<Sqlite>,
    finding_id: &str,
    remediated: bool,
) -> Result<(), sqlx::Error> {
    let remediated_int = i32::from(remediated);

    sqlx::query(
        "UPDATE discovery_findings
         SET remediated = ?, still_present_after_remediation = 0
         WHERE id = ?",
    )
    .bind(remediated_int)
    .bind(finding_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a finding as ignored (false positive or acceptable risk)
///
/// Ignored findings are not shown in the UI and will not trigger warnings
/// on future scans.
///
/// # Errors
/// Returns `sqlx::Error` if the database update fails.
pub async fn mark_finding_ignored(
    pool: &Pool<Sqlite>,
    finding_id: &str,
    ignored: bool,
) -> Result<(), sqlx::Error> {
    let ignored_int = i32::from(ignored);

    sqlx::query("UPDATE discovery_findings SET ignored = ? WHERE id = ?")
        .bind(ignored_int)
        .bind(finding_id)
        .execute(pool)
        .await?;

    Ok(())
}
