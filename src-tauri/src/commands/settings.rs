use crate::error::CommandError;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

/// Helper function to get database pool for a vault.
fn get_vault_pool(state: &AppState, vault_id: &str) -> Result<SqlitePool, CommandError> {
    let vault = state
        .get_vault(vault_id)
        .ok_or_else(|| CommandError::new("VAULT_LOCKED", "Vault is locked"))?;

    let pool = vault
        .database()
        .map_err(|e| {
            CommandError::new(
                "VAULT_ERROR",
                format!("Failed to access vault database: {e}"),
            )
        })?
        .pool()
        .clone();

    Ok(pool)
}

/// Serializable payload for email settings.
///
/// Passwords are never returned to the frontend — use `has_smtp_password` and
/// `has_imap_password` flags to indicate whether a password is currently saved.
/// To update a password, send a non-empty `smtp_password` / `imap_password`
/// field in `save_email_settings`. Empty strings are treated as "no change".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSettingsPayload {
    pub smtp_enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    /// Always empty in responses from `get_email_settings`.
    /// Set to a non-empty string in `save_email_settings` to update the password.
    pub smtp_password: String,
    /// True when an SMTP password is currently saved in the vault.
    pub has_smtp_password: bool,
    pub imap_enabled: bool,
    pub imap_host: String,
    pub imap_port: u16,
    pub imap_username: String,
    /// Always empty in responses from `get_email_settings`.
    /// Set to a non-empty string in `save_email_settings` to update the password.
    pub imap_password: String,
    /// True when an IMAP password is currently saved in the vault.
    pub has_imap_password: bool,
    /// Email address to CC on all outbound removal emails. Empty = no CC.
    pub cc_address: String,
}

/// Load email settings for a vault.
///
/// Passwords are never returned; use `has_smtp_password` / `has_imap_password`
/// to determine whether credentials are saved.
#[tauri::command]
pub async fn get_email_settings(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<EmailSettingsPayload, CommandError> {
    let pool = get_vault_pool(&state, &vault_id)?;

    let s = spectral_mail::settings::get_email_settings(&pool)
        .await
        .map_err(|e| CommandError::new("EMAIL_SETTINGS_ERROR", format!("{e}")))?;

    Ok(EmailSettingsPayload {
        smtp_enabled: s.smtp_enabled,
        smtp_host: s.smtp_host,
        smtp_port: s.smtp_port,
        smtp_username: s.smtp_username,
        smtp_password: String::new(), // never expose stored password
        has_smtp_password: !s.smtp_password.is_empty(),
        imap_enabled: s.imap_enabled,
        imap_host: s.imap_host,
        imap_port: s.imap_port,
        imap_username: s.imap_username,
        imap_password: String::new(), // never expose stored password
        has_imap_password: !s.imap_password.is_empty(),
        cc_address: s.cc_address,
    })
}

/// Persist email settings for a vault.
///
/// If `smtp_password` / `imap_password` is an empty string, the existing
/// stored password is preserved unchanged.
#[tauri::command]
pub async fn save_email_settings(
    state: State<'_, AppState>,
    vault_id: String,
    payload: EmailSettingsPayload,
) -> Result<(), CommandError> {
    let pool = get_vault_pool(&state, &vault_id)?;

    // Load existing to preserve passwords when the payload sends empty strings
    let existing = spectral_mail::settings::get_email_settings(&pool)
        .await
        .map_err(|e| CommandError::new("EMAIL_SETTINGS_ERROR", format!("{e}")))?;

    let smtp_password = if payload.smtp_password.is_empty() {
        existing.smtp_password
    } else {
        payload.smtp_password
    };

    let imap_password = if payload.imap_password.is_empty() {
        existing.imap_password
    } else {
        payload.imap_password
    };

    let settings = spectral_mail::settings::EmailSettings {
        smtp_enabled: payload.smtp_enabled,
        smtp_host: payload.smtp_host,
        smtp_port: payload.smtp_port,
        smtp_username: payload.smtp_username,
        smtp_password,
        imap_enabled: payload.imap_enabled,
        imap_host: payload.imap_host,
        imap_port: payload.imap_port,
        imap_username: payload.imap_username,
        imap_password,
        cc_address: payload.cc_address,
    };

    spectral_mail::settings::set_email_settings(&pool, &settings)
        .await
        .map_err(|e| CommandError::new("EMAIL_SETTINGS_ERROR", format!("{e}")))?;

    Ok(())
}

/// Test SMTP connectivity using the provided credentials.
#[tauri::command]
pub async fn test_smtp_connection(
    host: String,
    port: u16,
    username: String,
    password: String,
) -> Result<(), CommandError> {
    let config = spectral_mail::SmtpConfig {
        host,
        port,
        username,
        password,
    };

    spectral_mail::sender::test_smtp(&config)
        .await
        .map_err(|e| CommandError::new("SMTP_CONNECTION_ERROR", e))
}

/// Test IMAP connectivity using the provided credentials.
#[tauri::command]
pub async fn test_imap_connection(
    host: String,
    port: u16,
    username: String,
    password: String,
) -> Result<(), CommandError> {
    use spectral_mail::imap::{poll_for_verifications, ImapConfig};
    use std::collections::HashMap;

    let config = ImapConfig {
        host,
        port,
        username,
        password,
    };

    // Run synchronous IMAP polling in blocking task
    let result =
        tokio::task::spawn_blocking(move || poll_for_verifications(&config, &HashMap::new()))
            .await
            .map_err(|e| CommandError::new("TASK_JOIN_ERROR", format!("Task join error: {e}")))?;

    if let Some(err) = result.errors.first() {
        return Err(CommandError::new("IMAP_CONNECTION_ERROR", err.clone()));
    }

    Ok(())
}
