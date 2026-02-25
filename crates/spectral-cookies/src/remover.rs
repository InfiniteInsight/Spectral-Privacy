//! Cookie removal functionality with backup/restore support.

use crate::{
    browser::{Browser, BrowserProfile, BrowserType},
    error::{CookieError, Result},
    Cookie,
};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Result of a cookie removal operation.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RemovalResult {
    pub browser_type: BrowserType,
    pub profile_name: String,
    pub cookies_removed: usize,
    pub cookies_failed: usize,
    pub backup_path: Option<PathBuf>,
    pub errors: Vec<String>,
}

/// Cookie remover that safely deletes cookies from browser databases.
pub struct CookieRemover {
    create_backup: bool,
}

impl CookieRemover {
    /// Create a new cookie remover.
    pub fn new() -> Self {
        Self {
            create_backup: true,
        }
    }

    /// Create a remover without backups (dangerous).
    pub fn without_backup() -> Self {
        Self {
            create_backup: false,
        }
    }

    /// Remove specific cookies from a browser profile.
    pub fn remove_cookies(
        &self,
        profile: &BrowserProfile,
        cookies_to_remove: &[Cookie],
    ) -> Result<RemovalResult> {
        // Check if browser is running
        if Browser::is_browser_running(profile.browser_type) {
            return Err(CookieError::BrowserRunning(
                profile.browser_type.to_string(),
            ));
        }

        // Create backup if enabled
        let backup_path = if self.create_backup {
            Some(self.create_backup(&profile.cookie_db_path)?)
        } else {
            None
        };

        // Perform removal based on browser type
        let result = match profile.browser_type {
            BrowserType::Chrome | BrowserType::Edge | BrowserType::Brave => {
                self.remove_chromium_cookies(profile, cookies_to_remove)
            }
            BrowserType::Firefox => self.remove_firefox_cookies(profile, cookies_to_remove),
            #[cfg(target_os = "macos")]
            BrowserType::Safari => Err(CookieError::ParseError(
                "Safari cookie removal not yet implemented".to_string(),
            )),
            #[cfg(not(target_os = "macos"))]
            BrowserType::Safari => Err(CookieError::BrowserNotFound(
                "Safari only supported on macOS".to_string(),
            )),
            BrowserType::Other => Err(CookieError::BrowserNotFound(
                "Unsupported browser type".to_string(),
            )),
        };

        // If removal failed and we have a backup, offer to restore
        match result {
            Ok(mut removal_result) => {
                removal_result.backup_path = backup_path;
                Ok(removal_result)
            }
            Err(e) => {
                if let Some(backup) = &backup_path {
                    tracing::warn!(
                        "Cookie removal failed, backup available at: {}",
                        backup.display()
                    );
                }
                Err(e)
            }
        }
    }

    /// Remove cookies from Chromium-based browsers.
    fn remove_chromium_cookies(
        &self,
        profile: &BrowserProfile,
        cookies_to_remove: &[Cookie],
    ) -> Result<RemovalResult> {
        let db_path = &profile.cookie_db_path;

        let conn = Connection::open(db_path)?;

        let mut cookies_removed = 0;
        let mut cookies_failed = 0;
        let mut errors = Vec::new();

        for cookie in cookies_to_remove {
            let result = conn.execute(
                "DELETE FROM cookies WHERE name = ?1 AND host_key = ?2 AND path = ?3",
                [&cookie.name, &cookie.domain, &cookie.path],
            );

            match result {
                Ok(rows_affected) => {
                    if rows_affected > 0 {
                        cookies_removed += rows_affected;
                    } else {
                        cookies_failed += 1;
                        errors.push(format!(
                            "Cookie not found: {} @ {}",
                            cookie.name, cookie.domain
                        ));
                    }
                }
                Err(e) => {
                    cookies_failed += 1;
                    errors.push(format!(
                        "Failed to remove {} @ {}: {}",
                        cookie.name, cookie.domain, e
                    ));
                }
            }
        }

        Ok(RemovalResult {
            browser_type: profile.browser_type,
            profile_name: profile.profile_name.clone(),
            cookies_removed,
            cookies_failed,
            backup_path: None,
            errors,
        })
    }

    /// Remove cookies from Firefox.
    fn remove_firefox_cookies(
        &self,
        profile: &BrowserProfile,
        cookies_to_remove: &[Cookie],
    ) -> Result<RemovalResult> {
        let db_path = &profile.cookie_db_path;

        let conn = Connection::open(db_path)?;

        let mut cookies_removed = 0;
        let mut cookies_failed = 0;
        let mut errors = Vec::new();

        for cookie in cookies_to_remove {
            let result = conn.execute(
                "DELETE FROM moz_cookies WHERE name = ?1 AND host = ?2 AND path = ?3",
                [&cookie.name, &cookie.domain, &cookie.path],
            );

            match result {
                Ok(rows_affected) => {
                    if rows_affected > 0 {
                        cookies_removed += rows_affected;
                    } else {
                        cookies_failed += 1;
                        errors.push(format!(
                            "Cookie not found: {} @ {}",
                            cookie.name, cookie.domain
                        ));
                    }
                }
                Err(e) => {
                    cookies_failed += 1;
                    errors.push(format!(
                        "Failed to remove {} @ {}: {}",
                        cookie.name, cookie.domain, e
                    ));
                }
            }
        }

        Ok(RemovalResult {
            browser_type: profile.browser_type,
            profile_name: profile.profile_name.clone(),
            cookies_removed,
            cookies_failed,
            backup_path: None,
            errors,
        })
    }

    /// Create a backup of the cookie database.
    fn create_backup(&self, db_path: &Path) -> Result<PathBuf> {
        let backup_dir = dirs::data_local_dir()
            .ok_or_else(|| CookieError::BackupFailed("Cannot find data directory".to_string()))?
            .join("spectral")
            .join("cookie_backups");

        std::fs::create_dir_all(&backup_dir).map_err(|e| {
            CookieError::BackupFailed(format!("Failed to create backup directory: {}", e))
        })?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = db_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| CookieError::BackupFailed("Invalid database filename".to_string()))?;
        let backup_name = format!("{}_{}.backup", filename, timestamp);
        let backup_path = backup_dir.join(backup_name);

        std::fs::copy(db_path, &backup_path)
            .map_err(|e| CookieError::BackupFailed(format!("Failed to copy database: {}", e)))?;

        tracing::info!("Created cookie database backup: {}", backup_path.display());

        Ok(backup_path)
    }

    /// Restore a cookie database from backup.
    pub fn restore_from_backup(backup_path: &Path, target_path: &Path) -> Result<()> {
        if !backup_path.exists() {
            return Err(CookieError::RestoreFailed(
                "Backup file does not exist".to_string(),
            ));
        }

        std::fs::copy(backup_path, target_path)
            .map_err(|e| CookieError::RestoreFailed(format!("Failed to restore backup: {}", e)))?;

        tracing::info!("Restored cookie database from: {}", backup_path.display());

        Ok(())
    }

    /// Remove all cookies matching a specific domain.
    pub fn remove_by_domain(
        &self,
        profile: &BrowserProfile,
        domain: &str,
    ) -> Result<RemovalResult> {
        // Check if browser is running
        if Browser::is_browser_running(profile.browser_type) {
            return Err(CookieError::BrowserRunning(
                profile.browser_type.to_string(),
            ));
        }

        // Create backup
        let backup_path = if self.create_backup {
            Some(self.create_backup(&profile.cookie_db_path)?)
        } else {
            None
        };

        let conn = Connection::open(&profile.cookie_db_path)?;

        let (table_name, domain_column) = match profile.browser_type {
            BrowserType::Chrome | BrowserType::Edge | BrowserType::Brave => ("cookies", "host_key"),
            BrowserType::Firefox => ("moz_cookies", "host"),
            _ => {
                return Err(CookieError::BrowserNotFound(
                    "Unsupported browser type".to_string(),
                ))
            }
        };

        let query = format!("DELETE FROM {} WHERE {} LIKE ?1", table_name, domain_column);
        let rows_affected = conn.execute(&query, [format!("%{}", domain)])?;

        Ok(RemovalResult {
            browser_type: profile.browser_type,
            profile_name: profile.profile_name.clone(),
            cookies_removed: rows_affected,
            cookies_failed: 0,
            backup_path,
            errors: vec![],
        })
    }
}

impl Default for CookieRemover {
    fn default() -> Self {
        Self::new()
    }
}
