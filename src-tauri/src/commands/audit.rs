//! Audit log commands for privacy transparency.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Audit log entry response.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLogEntryResponse {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub subject: String,
    pub pii_fields: Option<Vec<String>>,
    pub data_destination: String,
    pub outcome: String,
}

/// Get audit log entries for a vault.
#[tauri::command]
pub async fn get_audit_log(
    state: State<'_, AppState>,
    vault_id: String,
    limit: Option<i64>,
) -> Result<Vec<AuditLogEntryResponse>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {e}"))?;

    let entries = spectral_db::audit_log::get_audit_entries(db.pool(), &vault_id, limit)
        .await
        .map_err(|e| format!("Failed to get audit entries: {e}"))?;

    Ok(entries
        .into_iter()
        .map(|entry| AuditLogEntryResponse {
            id: entry.id,
            timestamp: entry.timestamp.to_rfc3339(),
            event_type: entry.event_type,
            subject: entry.subject,
            pii_fields: entry.pii_fields,
            data_destination: entry.data_destination,
            outcome: entry.outcome,
        })
        .collect())
}

/// Get audit log entries filtered by event type.
#[tauri::command]
pub async fn get_audit_log_by_type(
    state: State<'_, AppState>,
    vault_id: String,
    event_type: String,
    limit: Option<i64>,
) -> Result<Vec<AuditLogEntryResponse>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {e}"))?;

    let entries =
        spectral_db::audit_log::get_audit_entries_by_type(db.pool(), &vault_id, &event_type, limit)
            .await
            .map_err(|e| format!("Failed to get audit entries: {e}"))?;

    Ok(entries
        .into_iter()
        .map(|entry| AuditLogEntryResponse {
            id: entry.id,
            timestamp: entry.timestamp.to_rfc3339(),
            event_type: entry.event_type,
            subject: entry.subject,
            pii_fields: entry.pii_fields,
            data_destination: entry.data_destination,
            outcome: entry.outcome,
        })
        .collect())
}

/// Create a new audit log entry.
#[tauri::command]
pub async fn create_audit_entry(
    state: State<'_, AppState>,
    vault_id: String,
    event_type: String,
    subject: String,
    pii_fields: Option<Vec<String>>,
    data_destination: String,
    outcome: String,
) -> Result<AuditLogEntryResponse, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {e}"))?;

    let entry = spectral_db::audit_log::insert_audit_entry(
        db.pool(),
        vault_id,
        event_type,
        subject,
        pii_fields,
        data_destination,
        outcome,
    )
    .await
    .map_err(|e| format!("Failed to create audit entry: {e}"))?;

    Ok(AuditLogEntryResponse {
        id: entry.id,
        timestamp: entry.timestamp.to_rfc3339(),
        event_type: entry.event_type,
        subject: entry.subject,
        pii_fields: entry.pii_fields,
        data_destination: entry.data_destination,
        outcome: entry.outcome,
    })
}

/// Clear all audit log entries for a vault.
#[tauri::command]
pub async fn clear_audit_log(state: State<'_, AppState>, vault_id: String) -> Result<u64, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {e}"))?;

    spectral_db::audit_log::clear_audit_log(db.pool(), &vault_id)
        .await
        .map_err(|e| format!("Failed to clear audit log: {e}"))
}
