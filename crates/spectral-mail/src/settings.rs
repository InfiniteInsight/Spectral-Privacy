//! Email settings management.
//!
//! Persists SMTP and IMAP credentials plus the optional CC address in the
//! vault's encrypted `settings` KV table, following the same pattern used by
//! `spectral-privacy` for LLM API keys.

use crate::{ImapConfig, SmtpConfig};
use spectral_db::settings::{get_setting, set_setting};
use sqlx::SqlitePool;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// ── Setting keys ─────────────────────────────────────────────────────────────

const KEY_SMTP_ENABLED: &str = "email.smtp.enabled";
const KEY_SMTP_HOST: &str = "email.smtp.host";
const KEY_SMTP_PORT: &str = "email.smtp.port";
const KEY_SMTP_USERNAME: &str = "email.smtp.username";
const KEY_SMTP_PASSWORD: &str = "email.smtp.password";
const KEY_IMAP_ENABLED: &str = "email.imap.enabled";
const KEY_IMAP_HOST: &str = "email.imap.host";
const KEY_IMAP_PORT: &str = "email.imap.port";
const KEY_IMAP_USERNAME: &str = "email.imap.username";
const KEY_IMAP_PASSWORD: &str = "email.imap.password";
const KEY_CC_ADDRESS: &str = "email.cc_address";

// ── Defaults ─────────────────────────────────────────────────────────────────

const DEFAULT_SMTP_PORT: u16 = 587;
const DEFAULT_IMAP_PORT: u16 = 993;

// ── Public types ─────────────────────────────────────────────────────────────

/// All email-related settings for a vault.
///
/// Passwords are stored as plaintext in memory; they are protected at rest by
/// the vault's encryption layer (the `SqlitePool` is always an
/// `EncryptedPool`).
#[derive(Debug, Clone, Default)]
pub struct EmailSettings {
    pub smtp_enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    /// Plaintext in memory; encrypted at rest via vault pool.
    pub smtp_password: String, // nosemgrep: use-zeroize-for-secrets
    pub imap_enabled: bool,
    pub imap_host: String,
    pub imap_port: u16,
    pub imap_username: String,
    /// Plaintext in memory; encrypted at rest via vault pool.
    pub imap_password: String, // nosemgrep: use-zeroize-for-secrets
    /// Email address to CC on every outbound removal email. Empty = no CC.
    pub cc_address: String,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn get_bool(pool: &SqlitePool, key: &str, default: bool) -> Result<bool> {
    match get_setting(pool, key).await? {
        Some(v) => Ok(serde_json::from_value(v)?),
        None => Ok(default),
    }
}

async fn get_string(pool: &SqlitePool, key: &str) -> Result<String> {
    match get_setting(pool, key).await? {
        Some(v) => Ok(serde_json::from_value(v)?),
        None => Ok(String::new()),
    }
}

async fn get_u16(pool: &SqlitePool, key: &str, default: u16) -> Result<u16> {
    match get_setting(pool, key).await? {
        Some(v) => Ok(serde_json::from_value(v)?),
        None => Ok(default),
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Load all email settings from the database, returning defaults for any
/// missing keys.
///
/// # Errors
/// Returns an error if the database is unavailable or a stored value is
/// malformed.
pub async fn get_email_settings(pool: &SqlitePool) -> Result<EmailSettings> {
    Ok(EmailSettings {
        smtp_enabled: get_bool(pool, KEY_SMTP_ENABLED, false).await?,
        smtp_host: get_string(pool, KEY_SMTP_HOST).await?,
        smtp_port: get_u16(pool, KEY_SMTP_PORT, DEFAULT_SMTP_PORT).await?,
        smtp_username: get_string(pool, KEY_SMTP_USERNAME).await?,
        smtp_password: get_string(pool, KEY_SMTP_PASSWORD).await?,
        imap_enabled: get_bool(pool, KEY_IMAP_ENABLED, false).await?,
        imap_host: get_string(pool, KEY_IMAP_HOST).await?,
        imap_port: get_u16(pool, KEY_IMAP_PORT, DEFAULT_IMAP_PORT).await?,
        imap_username: get_string(pool, KEY_IMAP_USERNAME).await?,
        imap_password: get_string(pool, KEY_IMAP_PASSWORD).await?,
        cc_address: get_string(pool, KEY_CC_ADDRESS).await?,
    })
}

/// Persist all email settings to the database.
///
/// # Errors
/// Returns an error if any database write fails.
pub async fn set_email_settings(pool: &SqlitePool, s: &EmailSettings) -> Result<()> {
    set_setting(
        pool,
        KEY_SMTP_ENABLED,
        &serde_json::to_value(s.smtp_enabled)?,
    )
    .await?;
    set_setting(pool, KEY_SMTP_HOST, &serde_json::to_value(&s.smtp_host)?).await?;
    set_setting(pool, KEY_SMTP_PORT, &serde_json::to_value(s.smtp_port)?).await?;
    set_setting(
        pool,
        KEY_SMTP_USERNAME,
        &serde_json::to_value(&s.smtp_username)?,
    )
    .await?;
    set_setting(
        pool,
        KEY_SMTP_PASSWORD,
        &serde_json::to_value(&s.smtp_password)?,
    )
    .await?;
    set_setting(
        pool,
        KEY_IMAP_ENABLED,
        &serde_json::to_value(s.imap_enabled)?,
    )
    .await?;
    set_setting(pool, KEY_IMAP_HOST, &serde_json::to_value(&s.imap_host)?).await?;
    set_setting(pool, KEY_IMAP_PORT, &serde_json::to_value(s.imap_port)?).await?;
    set_setting(
        pool,
        KEY_IMAP_USERNAME,
        &serde_json::to_value(&s.imap_username)?,
    )
    .await?;
    set_setting(
        pool,
        KEY_IMAP_PASSWORD,
        &serde_json::to_value(&s.imap_password)?,
    )
    .await?;
    set_setting(pool, KEY_CC_ADDRESS, &serde_json::to_value(&s.cc_address)?).await?;
    Ok(())
}

/// Returns a ready-to-use `SmtpConfig` when SMTP is enabled and configured,
/// or `None` otherwise.
///
/// # Errors
/// Returns an error if the database read fails.
pub async fn get_smtp_config(pool: &SqlitePool) -> Result<Option<SmtpConfig>> {
    let enabled = get_bool(pool, KEY_SMTP_ENABLED, false).await?;
    let host = get_string(pool, KEY_SMTP_HOST).await?;
    if !enabled || host.is_empty() {
        return Ok(None);
    }
    Ok(Some(SmtpConfig {
        host,
        port: get_u16(pool, KEY_SMTP_PORT, DEFAULT_SMTP_PORT).await?,
        username: get_string(pool, KEY_SMTP_USERNAME).await?,
        password: get_string(pool, KEY_SMTP_PASSWORD).await?, // nosemgrep: use-zeroize-for-secrets
    }))
}

/// Returns a ready-to-use `ImapConfig` when IMAP is enabled and configured,
/// or `None` otherwise.
///
/// # Errors
/// Returns an error if the database read fails.
pub async fn get_imap_config(pool: &SqlitePool) -> Result<Option<ImapConfig>> {
    let enabled = get_bool(pool, KEY_IMAP_ENABLED, false).await?;
    let host = get_string(pool, KEY_IMAP_HOST).await?;
    if !enabled || host.is_empty() {
        return Ok(None);
    }
    Ok(Some(ImapConfig {
        host,
        port: get_u16(pool, KEY_IMAP_PORT, DEFAULT_IMAP_PORT).await?,
        username: get_string(pool, KEY_IMAP_USERNAME).await?,
        password: get_string(pool, KEY_IMAP_PASSWORD).await?, // nosemgrep: use-zeroize-for-secrets
    }))
}

/// Returns the CC address if one is configured, or `None` if empty.
///
/// # Errors
/// Returns an error if the database read fails.
pub async fn get_cc_address(pool: &SqlitePool) -> Result<Option<String>> {
    let addr = get_string(pool, KEY_CC_ADDRESS).await?;
    if addr.is_empty() {
        Ok(None)
    } else {
        Ok(Some(addr))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use spectral_db::Database;

    async fn test_pool() -> SqlitePool {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create test database");
        db.run_migrations().await.expect("run migrations");
        db.pool().clone()
    }

    #[tokio::test]
    async fn test_defaults_when_empty() {
        let pool = test_pool().await;
        let s = get_email_settings(&pool).await.unwrap(); // nosemgrep: no-unwrap-in-production
        assert!(!s.smtp_enabled);
        assert!(s.smtp_host.is_empty());
        assert_eq!(s.smtp_port, DEFAULT_SMTP_PORT);
        assert!(!s.imap_enabled);
        assert_eq!(s.imap_port, DEFAULT_IMAP_PORT);
        assert!(s.cc_address.is_empty());
    }

    #[tokio::test]
    async fn test_round_trip() {
        let pool = test_pool().await;
        let settings = EmailSettings {
            smtp_enabled: true,
            smtp_host: "smtp.gmail.com".to_string(),
            smtp_port: 587,
            smtp_username: "user@example.com".to_string(),
            smtp_password: "secret".to_string(), // nosemgrep: use-zeroize-for-secrets  # pragma: allowlist secret
            imap_enabled: true,
            imap_host: "imap.gmail.com".to_string(),
            imap_port: 993,
            imap_username: "user@example.com".to_string(),
            imap_password: "imapsecret".to_string(), // nosemgrep: use-zeroize-for-secrets  # pragma: allowlist secret
            cc_address: "me@example.com".to_string(),
        };
        set_email_settings(&pool, &settings).await.unwrap(); // nosemgrep: no-unwrap-in-production

        let loaded = get_email_settings(&pool).await.unwrap(); // nosemgrep: no-unwrap-in-production
        assert!(loaded.smtp_enabled);
        assert_eq!(loaded.smtp_host, "smtp.gmail.com");
        assert_eq!(loaded.smtp_port, 587);
        assert_eq!(loaded.smtp_username, "user@example.com");
        assert_eq!(loaded.smtp_password, "secret");
        assert!(loaded.imap_enabled);
        assert_eq!(loaded.imap_host, "imap.gmail.com");
        assert_eq!(loaded.cc_address, "me@example.com");
    }

    #[tokio::test]
    async fn test_get_smtp_config_when_disabled() {
        let pool = test_pool().await;
        let s = EmailSettings {
            smtp_enabled: false,
            smtp_host: "smtp.gmail.com".to_string(),
            ..Default::default()
        };
        set_email_settings(&pool, &s).await.unwrap(); // nosemgrep: no-unwrap-in-production
        let config = get_smtp_config(&pool).await.unwrap(); // nosemgrep: no-unwrap-in-production
        assert!(config.is_none());
    }

    #[tokio::test]
    async fn test_get_smtp_config_when_enabled() {
        let pool = test_pool().await;
        let s = EmailSettings {
            smtp_enabled: true,
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 465,
            smtp_username: "u".to_string(),
            smtp_password: "p".to_string(), // nosemgrep: use-zeroize-for-secrets  # pragma: allowlist secret
            ..Default::default()
        };
        set_email_settings(&pool, &s).await.unwrap(); // nosemgrep: no-unwrap-in-production
        let config = get_smtp_config(&pool).await.unwrap(); // nosemgrep: no-unwrap-in-production
        assert!(config.is_some());
        let c = config.unwrap(); // nosemgrep: no-unwrap-in-production
        assert_eq!(c.host, "smtp.example.com");
        assert_eq!(c.port, 465);
    }

    #[tokio::test]
    async fn test_get_cc_address_empty() {
        let pool = test_pool().await;
        let cc = get_cc_address(&pool).await.unwrap(); // nosemgrep: no-unwrap-in-production
        assert!(cc.is_none());
    }

    #[tokio::test]
    async fn test_get_cc_address_set() {
        let pool = test_pool().await;
        let s = EmailSettings {
            cc_address: "cc@example.com".to_string(),
            ..Default::default()
        };
        set_email_settings(&pool, &s).await.unwrap(); // nosemgrep: no-unwrap-in-production
        let cc = get_cc_address(&pool).await.unwrap(); // nosemgrep: no-unwrap-in-production
        assert_eq!(cc, Some("cc@example.com".to_string()));
    }
}
