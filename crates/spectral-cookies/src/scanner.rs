//! Cookie scanning functionality for various browsers.

use crate::{
    browser::{Browser, BrowserProfile, BrowserType},
    error::{CookieError, Result},
    matcher::CookieMatcher,
    Cookie, SameSite,
};
use rusqlite::Connection;
use std::path::Path;

/// A scanned cookie with metadata about which broker it matches.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScannedCookie {
    pub cookie: Cookie,
    pub browser_type: BrowserType,
    pub profile_name: String,
    pub cookie_db_filename: String,
    pub matched_broker_id: Option<String>,
}

/// Result of a cookie scan.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CookieScanResult {
    pub total_cookies: usize,
    pub matched_cookies: usize,
    pub cookies_by_browser: std::collections::HashMap<String, usize>,
    pub cookies_by_broker: std::collections::HashMap<String, usize>,
    pub scanned_cookies: Vec<ScannedCookie>,
}

/// Cookie scanner that reads cookies from browser databases.
pub struct CookieScanner {
    matcher: Option<CookieMatcher>,
}

impl CookieScanner {
    /// Create a new cookie scanner.
    pub fn new() -> Self {
        Self { matcher: None }
    }

    /// Create a scanner with a cookie matcher.
    pub fn with_matcher(matcher: CookieMatcher) -> Self {
        Self {
            matcher: Some(matcher),
        }
    }

    /// Scan all installed browsers for cookies.
    pub fn scan_all_browsers(&self) -> Result<CookieScanResult> {
        let profiles = Browser::detect_installed()?;
        let mut all_cookies = Vec::new();

        for profile in profiles {
            match self.scan_profile(&profile) {
                Ok(cookies) => all_cookies.extend(cookies),
                Err(e) => {
                    tracing::warn!(
                        "Failed to scan {}/{}: {}",
                        profile.browser_type,
                        profile.profile_name,
                        e
                    );
                }
            }
        }

        Ok(self.build_scan_result(all_cookies))
    }

    /// Scan a specific browser profile.
    pub fn scan_profile(&self, profile: &BrowserProfile) -> Result<Vec<ScannedCookie>> {
        // Check if browser is running
        if Browser::is_browser_running(profile.browser_type) {
            tracing::warn!(
                "{} is running, cookie database may be locked",
                profile.browser_type
            );
        }

        match profile.browser_type {
            BrowserType::Chrome | BrowserType::Edge | BrowserType::Brave => {
                self.scan_chromium_cookies(profile)
            }
            BrowserType::Firefox => self.scan_firefox_cookies(profile),
            #[cfg(target_os = "macos")]
            BrowserType::Safari => self.scan_safari_cookies(profile),
            #[cfg(not(target_os = "macos"))]
            BrowserType::Safari => Err(CookieError::BrowserNotFound(
                "Safari only supported on macOS".to_string(),
            )),
            BrowserType::Other => Err(CookieError::BrowserNotFound(
                "Unsupported browser type".to_string(),
            )),
        }
    }

    /// Scan Chromium-based browser cookies (Chrome, Edge, Brave).
    fn scan_chromium_cookies(&self, profile: &BrowserProfile) -> Result<Vec<ScannedCookie>> {
        let db_path = &profile.cookie_db_path;

        // Extract the database filename for display purposes
        let cookie_db_filename = db_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Cookies")
            .to_string();

        // Copy database to temporary location to avoid lock issues
        let temp_db = self.copy_to_temp(db_path)?;

        let conn = Connection::open(&temp_db).map_err(|e| {
            CookieError::DatabaseError(format!("Failed to open cookie database: {}", e))
        })?;

        let mut stmt = conn.prepare(
            "SELECT name, value, host_key, path, creation_utc, expires_utc, last_access_utc,
                    is_secure, is_httponly, samesite
             FROM cookies",
        )?;

        let cookies = stmt.query_map([], |row| {
            let same_site_val: i32 = row.get(9)?;
            let same_site = match same_site_val {
                0 => None,
                1 => Some(SameSite::None),
                2 => Some(SameSite::Lax),
                3 => Some(SameSite::Strict),
                _ => None,
            };

            Ok(Cookie {
                name: row.get(0)?,
                value: row.get(1)?,
                domain: row.get(2)?,
                path: row.get(3)?,
                creation_time: row.get(4).ok(),
                expiry_time: row.get(5).ok(),
                last_access_time: row.get(6).ok(),
                is_secure: row.get::<_, i32>(7)? != 0,
                is_httponly: row.get::<_, i32>(8)? != 0,
                same_site,
            })
        })?;

        let mut scanned_cookies = Vec::new();
        for cookie_result in cookies {
            let cookie = cookie_result?;
            let matched_broker_id = self.matcher.as_ref().and_then(|m| m.match_cookie(&cookie));

            scanned_cookies.push(ScannedCookie {
                cookie,
                browser_type: profile.browser_type,
                profile_name: profile.profile_name.clone(),
                cookie_db_filename: cookie_db_filename.clone(),
                matched_broker_id,
            });
        }

        // Clean up temp file
        let _ = std::fs::remove_file(temp_db);

        Ok(scanned_cookies)
    }

    /// Scan Firefox cookies.
    fn scan_firefox_cookies(&self, profile: &BrowserProfile) -> Result<Vec<ScannedCookie>> {
        let db_path = &profile.cookie_db_path;

        // Extract the database filename for display purposes
        let cookie_db_filename = db_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("cookies.sqlite")
            .to_string();

        // Copy database to temporary location
        let temp_db = self.copy_to_temp(db_path)?;

        let conn = Connection::open(&temp_db).map_err(|e| {
            CookieError::DatabaseError(format!("Failed to open Firefox cookie database: {}", e))
        })?;

        let mut stmt = conn.prepare(
            "SELECT name, value, host, path, creationTime, expiry, lastAccessed,
                    isSecure, isHttpOnly, sameSite
             FROM moz_cookies",
        )?;

        let cookies = stmt.query_map([], |row| {
            let same_site_val: i32 = row.get(9)?;
            let same_site = match same_site_val {
                0 => None,
                1 => Some(SameSite::None),
                2 => Some(SameSite::Lax),
                3 => Some(SameSite::Strict),
                _ => None,
            };

            Ok(Cookie {
                name: row.get(0)?,
                value: row.get(1)?,
                domain: row.get(2)?,
                path: row.get(3)?,
                creation_time: row.get(4).ok(),
                expiry_time: row.get(5).ok(),
                last_access_time: row.get(6).ok(),
                is_secure: row.get::<_, i32>(7)? != 0,
                is_httponly: row.get::<_, i32>(8)? != 0,
                same_site,
            })
        })?;

        let mut scanned_cookies = Vec::new();
        for cookie_result in cookies {
            let cookie = cookie_result?;
            let matched_broker_id = self.matcher.as_ref().and_then(|m| m.match_cookie(&cookie));

            scanned_cookies.push(ScannedCookie {
                cookie,
                browser_type: profile.browser_type,
                profile_name: profile.profile_name.clone(),
                cookie_db_filename: cookie_db_filename.clone(),
                matched_broker_id,
            });
        }

        // Clean up temp file
        let _ = std::fs::remove_file(temp_db);

        Ok(scanned_cookies)
    }

    /// Scan Safari cookies (macOS only).
    #[cfg(target_os = "macos")]
    fn scan_safari_cookies(&self, profile: &BrowserProfile) -> Result<Vec<ScannedCookie>> {
        // Safari uses binary plist format (.binarycookies)
        // This is more complex and would require binarycookies parser
        // For now, return an informative error
        Err(CookieError::ParseError(
            "Safari cookie parsing not yet implemented. Safari uses .binarycookies format which requires specialized parsing.".to_string()
        ))
    }

    /// Copy database to temporary location to avoid lock issues.
    fn copy_to_temp(&self, db_path: &Path) -> Result<std::path::PathBuf> {
        let temp_dir = std::env::temp_dir();
        let temp_name = format!("spectral_cookies_{}.db", uuid::Uuid::new_v4());
        let temp_path = temp_dir.join(temp_name);

        std::fs::copy(db_path, &temp_path).map_err(|e| {
            CookieError::DatabaseError(format!("Failed to copy cookie database: {}", e))
        })?;

        Ok(temp_path)
    }

    /// Build scan result from scanned cookies.
    fn build_scan_result(&self, scanned_cookies: Vec<ScannedCookie>) -> CookieScanResult {
        let total_cookies = scanned_cookies.len();
        let matched_cookies = scanned_cookies
            .iter()
            .filter(|c| c.matched_broker_id.is_some())
            .count();

        let mut cookies_by_browser = std::collections::HashMap::new();
        let mut cookies_by_broker = std::collections::HashMap::new();

        for scanned in &scanned_cookies {
            *cookies_by_browser
                .entry(scanned.browser_type.as_str().to_string())
                .or_insert(0) += 1;

            if let Some(broker_id) = &scanned.matched_broker_id {
                *cookies_by_broker.entry(broker_id.clone()).or_insert(0) += 1;
            }
        }

        CookieScanResult {
            total_cookies,
            matched_cookies,
            cookies_by_browser,
            cookies_by_broker,
            scanned_cookies,
        }
    }
}

impl Default for CookieScanner {
    fn default() -> Self {
        Self::new()
    }
}
