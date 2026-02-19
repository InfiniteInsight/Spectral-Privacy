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
    /// Whether this finding has been remediated
    pub remediated: bool,
    /// ISO 8601 timestamp when found
    pub found_at: String,
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
}

/// Insert a new discovery finding
///
/// # Errors
/// Returns `sqlx::Error` if the database insert fails.
pub async fn insert_discovery_finding(
    pool: &Pool<Sqlite>,
    params: CreateDiscoveryFinding,
) -> Result<DiscoveryFinding, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let found_at = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO discovery_findings (id, vault_id, source, source_detail, finding_type, risk_level, description, recommended_action, found_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&params.vault_id)
    .bind(&params.source)
    .bind(&params.source_detail)
    .bind(&params.finding_type)
    .bind(&params.risk_level)
    .bind(&params.description)
    .bind(&params.recommended_action)
    .bind(&found_at)
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
        remediated: false,
        found_at,
    })
}

/// Get all discovery findings for a vault
///
/// # Errors
/// Returns `sqlx::Error` if the database query fails.
pub async fn get_discovery_findings(
    pool: &Pool<Sqlite>,
    vault_id: &str,
) -> Result<Vec<DiscoveryFinding>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, vault_id, source, source_detail, finding_type, risk_level, description, recommended_action, remediated, found_at
         FROM discovery_findings
         WHERE vault_id = ?
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
            remediated: row.get::<i64, _>("remediated") != 0,
            found_at: row.get("found_at"),
        })
        .collect();

    Ok(findings)
}

/// Update the remediated status of a finding
///
/// # Errors
/// Returns `sqlx::Error` if the database update fails.
pub async fn update_finding_remediated(
    pool: &Pool<Sqlite>,
    finding_id: &str,
    remediated: bool,
) -> Result<(), sqlx::Error> {
    let remediated_int = i32::from(remediated);

    sqlx::query("UPDATE discovery_findings SET remediated = ? WHERE id = ?")
        .bind(remediated_int)
        .bind(finding_id)
        .execute(pool)
        .await?;

    Ok(())
}
