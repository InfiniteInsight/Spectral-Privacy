//! Audit log operations for privacy transparency.
//!
//! Records privacy-related actions like scans, findings, removals, and data access.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

/// An audit log entry recording a privacy-related action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique identifier for this entry
    pub id: String,
    /// Vault this entry belongs to
    pub vault_id: String,
    /// When this event occurred
    pub timestamp: DateTime<Utc>,
    /// Type of event (e.g., "`ScanStarted`", "`FindingCreated`")
    pub event_type: String,
    /// Human-readable description of the event
    pub subject: String,
    /// Names of PII fields involved (never values)
    pub pii_fields: Option<Vec<String>>,
    /// Where data went: `LocalOnly`, `ExternalSite:domain`, or `CloudLlm:provider`
    pub data_destination: String,
    /// Outcome: Allowed or Denied
    pub outcome: String,
}

/// Insert a new audit log entry.
pub async fn insert_audit_entry(
    pool: &Pool<Sqlite>,
    vault_id: String,
    event_type: String,
    subject: String,
    pii_fields: Option<Vec<String>>,
    data_destination: String,
    outcome: String,
) -> Result<AuditLogEntry, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let timestamp = Utc::now();
    let timestamp_str = timestamp.to_rfc3339();

    let pii_fields_json = pii_fields
        .as_ref()
        .map(|fields| serde_json::to_string(fields).unwrap_or_else(|_| "[]".to_string()));

    sqlx::query(
        "INSERT INTO audit_log (id, vault_id, timestamp, event_type, subject, pii_fields, data_destination, outcome)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&vault_id)
    .bind(&timestamp_str)
    .bind(&event_type)
    .bind(&subject)
    .bind(&pii_fields_json)
    .bind(&data_destination)
    .bind(&outcome)
    .execute(pool)
    .await?;

    Ok(AuditLogEntry {
        id,
        vault_id,
        timestamp,
        event_type,
        subject,
        pii_fields,
        data_destination,
        outcome,
    })
}

/// Get all audit log entries for a vault, ordered by timestamp descending.
pub async fn get_audit_entries(
    pool: &Pool<Sqlite>,
    vault_id: &str,
    limit: Option<i64>,
) -> Result<Vec<AuditLogEntry>, sqlx::Error> {
    let query = if let Some(lim) = limit {
        format!(
            "SELECT id, vault_id, timestamp, event_type, subject, pii_fields, data_destination, outcome
             FROM audit_log
             WHERE vault_id = ?
             ORDER BY timestamp DESC
             LIMIT {lim}"
        )
    } else {
        "SELECT id, vault_id, timestamp, event_type, subject, pii_fields, data_destination, outcome
         FROM audit_log
         WHERE vault_id = ?
         ORDER BY timestamp DESC"
            .to_string()
    };

    let rows = sqlx::query(&query).bind(vault_id).fetch_all(pool).await?;

    let mut entries = Vec::new();
    for row in rows {
        let pii_fields_json: Option<String> = row.try_get("pii_fields")?;
        let pii_fields =
            pii_fields_json.and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok());

        let timestamp_str: String = row.try_get("timestamp")?; // nosemgrep: use-zeroize-for-secrets
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

        entries.push(AuditLogEntry {
            id: row.try_get("id")?,
            vault_id: row.try_get("vault_id")?,
            timestamp,
            event_type: row.try_get("event_type")?,
            subject: row.try_get("subject")?,
            pii_fields,
            data_destination: row.try_get("data_destination")?,
            outcome: row.try_get("outcome")?,
        });
    }

    Ok(entries)
}

/// Get audit entries filtered by event type.
pub async fn get_audit_entries_by_type(
    pool: &Pool<Sqlite>,
    vault_id: &str,
    event_type: &str,
    limit: Option<i64>,
) -> Result<Vec<AuditLogEntry>, sqlx::Error> {
    let query = if let Some(lim) = limit {
        format!(
            "SELECT id, vault_id, timestamp, event_type, subject, pii_fields, data_destination, outcome
             FROM audit_log
             WHERE vault_id = ? AND event_type = ?
             ORDER BY timestamp DESC
             LIMIT {lim}"
        )
    } else {
        "SELECT id, vault_id, timestamp, event_type, subject, pii_fields, data_destination, outcome
         FROM audit_log
         WHERE vault_id = ? AND event_type = ?
         ORDER BY timestamp DESC"
            .to_string()
    };

    let rows = sqlx::query(&query)
        .bind(vault_id)
        .bind(event_type)
        .fetch_all(pool)
        .await?;

    let mut entries = Vec::new();
    for row in rows {
        let pii_fields_json: Option<String> = row.try_get("pii_fields")?;
        let pii_fields =
            pii_fields_json.and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok());

        let timestamp_str: String = row.try_get("timestamp")?; // nosemgrep: use-zeroize-for-secrets
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

        entries.push(AuditLogEntry {
            id: row.try_get("id")?,
            vault_id: row.try_get("vault_id")?,
            timestamp,
            event_type: row.try_get("event_type")?,
            subject: row.try_get("subject")?,
            pii_fields,
            data_destination: row.try_get("data_destination")?,
            outcome: row.try_get("outcome")?,
        });
    }

    Ok(entries)
}

/// Delete all audit log entries for a vault.
pub async fn clear_audit_log(pool: &Pool<Sqlite>, vault_id: &str) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM audit_log WHERE vault_id = ?")
        .bind(vault_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    // nosemgrep: no-unwrap-in-production
    async fn setup_test_db() -> Pool<Sqlite> {
        let pool = Pool::<Sqlite>::connect(":memory:").await.unwrap();

        sqlx::query(
            "CREATE TABLE audit_log (
                id TEXT PRIMARY KEY NOT NULL,
                vault_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                event_type TEXT NOT NULL,
                subject TEXT NOT NULL,
                pii_fields TEXT,
                data_destination TEXT NOT NULL,
                outcome TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    // nosemgrep: no-unwrap-in-production
    async fn test_insert_and_get_audit_entries() {
        let pool = setup_test_db().await;

        let entry = insert_audit_entry(
            &pool,
            "vault-1".to_string(),
            "ScanStarted".to_string(),
            "Scan job started for profile John Doe".to_string(),
            Some(vec!["name".to_string(), "city".to_string()]),
            "LocalOnly".to_string(),
            "Allowed".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(entry.vault_id, "vault-1");
        assert_eq!(entry.event_type, "ScanStarted");

        let entries = get_audit_entries(&pool, "vault-1", None).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].subject, "Scan job started for profile John Doe");
    }

    #[tokio::test]
    // nosemgrep: no-unwrap-in-production
    async fn test_get_audit_entries_with_limit() {
        let pool = setup_test_db().await;

        for i in 0..10 {
            insert_audit_entry(
                &pool,
                "vault-1".to_string(),
                "ScanStarted".to_string(),
                format!("Scan {i}"),
                None,
                "LocalOnly".to_string(),
                "Allowed".to_string(),
            )
            .await
            .unwrap();
        }

        let entries = get_audit_entries(&pool, "vault-1", Some(5)).await.unwrap();
        assert_eq!(entries.len(), 5);
    }

    #[tokio::test]
    // nosemgrep: no-unwrap-in-production
    async fn test_get_audit_entries_by_type() {
        let pool = setup_test_db().await;

        insert_audit_entry(
            &pool,
            "vault-1".to_string(),
            "ScanStarted".to_string(),
            "Scan 1".to_string(),
            None,
            "LocalOnly".to_string(),
            "Allowed".to_string(),
        )
        .await
        .unwrap();

        insert_audit_entry(
            &pool,
            "vault-1".to_string(),
            "FindingCreated".to_string(),
            "Finding 1".to_string(),
            None,
            "LocalOnly".to_string(),
            "Allowed".to_string(),
        )
        .await
        .unwrap();

        let entries = get_audit_entries_by_type(&pool, "vault-1", "ScanStarted", None)
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event_type, "ScanStarted");
    }

    #[tokio::test]
    // nosemgrep: no-unwrap-in-production
    async fn test_clear_audit_log() {
        let pool = setup_test_db().await;

        insert_audit_entry(
            &pool,
            "vault-1".to_string(),
            "ScanStarted".to_string(),
            "Test".to_string(),
            None,
            "LocalOnly".to_string(),
            "Allowed".to_string(),
        )
        .await
        .unwrap();

        let deleted = clear_audit_log(&pool, "vault-1").await.unwrap();
        assert_eq!(deleted, 1);

        let entries = get_audit_entries(&pool, "vault-1", None).await.unwrap();
        assert_eq!(entries.len(), 0);
    }
}
