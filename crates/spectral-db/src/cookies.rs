//! Cookie tracking database module.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

/// Browser cookie record in database.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct BrowserCookie {
    pub id: String,
    pub vault_id: String,
    pub browser_type: String,
    pub profile_name: Option<String>,
    pub cookie_name: String,
    pub cookie_domain: String,
    pub cookie_value: Option<String>,
    pub cookie_path: String,
    pub creation_time: Option<i64>,
    pub expiry_time: Option<i64>,
    pub last_access_time: Option<i64>,
    pub is_secure: i32,
    pub is_httponly: i32,
    pub same_site: Option<String>,
    pub matched_broker_id: Option<String>,
    pub scan_timestamp: String,
    pub removal_status: String,
    pub removed_at: Option<String>,
    pub cookie_db_filename: String,
}

/// Cookie scan session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct CookieScan {
    pub id: String,
    pub vault_id: String,
    pub scan_timestamp: DateTime<Utc>,
    pub browsers_scanned: Vec<String>,
    pub total_cookies_found: i32,
    pub matched_cookies: i32,
    pub brokers_matched: Option<Vec<String>>,
    pub scan_status: String,
    pub error_message: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Cookie removal operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct CookieRemoval {
    pub id: String,
    pub vault_id: String,
    pub scan_id: Option<String>,
    pub browser_type: String,
    pub profile_name: Option<String>,
    pub cookies_to_remove: i32,
    pub cookies_removed: i32,
    pub cookies_failed: i32,
    pub removal_timestamp: DateTime<Utc>,
    pub completion_timestamp: Option<DateTime<Utc>>,
    pub status: String,
    pub error_message: Option<String>,
    pub backup_path: Option<String>,
}

/// Insert scanned cookies into database.
pub async fn insert_scanned_cookies(
    pool: &Pool<Sqlite>,
    vault_id: &str,
    cookies: Vec<BrowserCookie>,
) -> Result<(), sqlx::Error> {
    for cookie in cookies {
        sqlx::query(
            "INSERT INTO browser_cookies (id, vault_id, browser_type, profile_name,
                cookie_name, cookie_domain, cookie_value, cookie_path,
                creation_time, expiry_time, last_access_time,
                is_secure, is_httponly, same_site,
                matched_broker_id, scan_timestamp, removal_status)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&cookie.id)
        .bind(vault_id)
        .bind(&cookie.browser_type)
        .bind(&cookie.profile_name)
        .bind(&cookie.cookie_name)
        .bind(&cookie.cookie_domain)
        .bind(&cookie.cookie_value)
        .bind(&cookie.cookie_path)
        .bind(cookie.creation_time)
        .bind(cookie.expiry_time)
        .bind(cookie.last_access_time)
        .bind(cookie.is_secure)
        .bind(cookie.is_httponly)
        .bind(&cookie.same_site)
        .bind(&cookie.matched_broker_id)
        .bind(&cookie.scan_timestamp)
        .bind(&cookie.removal_status)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Create a new cookie scan session.
pub async fn create_cookie_scan(
    pool: &Pool<Sqlite>,
    vault_id: &str,
    scan_timestamp: &str,
    browsers_scanned: Vec<String>,
    total_cookies: i32,
    matched_cookies: i32,
    brokers_matched: Option<Vec<String>>,
) -> Result<String, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let browsers_json = serde_json::to_string(&browsers_scanned).unwrap_or_default();
    let brokers_json = brokers_matched
        .as_ref()
        .and_then(|b| serde_json::to_string(b).ok());

    sqlx::query(
        "INSERT INTO cookie_scans (id, vault_id, scan_timestamp, browsers_scanned,
            total_cookies_found, matched_cookies, brokers_matched, scan_status)
         VALUES (?, ?, ?, ?, ?, ?, ?, 'Completed')",
    )
    .bind(&id)
    .bind(vault_id)
    .bind(scan_timestamp)
    .bind(&browsers_json)
    .bind(total_cookies)
    .bind(matched_cookies)
    .bind(&brokers_json)
    .execute(pool)
    .await?;

    Ok(id)
}

/// Get all scanned cookies for a vault.
pub async fn get_scanned_cookies(
    pool: &Pool<Sqlite>,
    vault_id: &str,
) -> Result<Vec<BrowserCookie>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, vault_id, browser_type, profile_name, cookie_name, cookie_domain,
                cookie_value, cookie_path, creation_time, expiry_time, last_access_time,
                is_secure, is_httponly, same_site, matched_broker_id,
                scan_timestamp, removal_status, removed_at, cookie_db_filename
         FROM browser_cookies
         WHERE vault_id = ?
         ORDER BY scan_timestamp DESC",
    )
    .bind(vault_id)
    .fetch_all(pool)
    .await?;

    let cookies: Vec<BrowserCookie> = rows
        .into_iter()
        .map(|row| BrowserCookie {
            id: row.get("id"),
            vault_id: row.get("vault_id"),
            browser_type: row.get("browser_type"),
            profile_name: row.get("profile_name"),
            cookie_name: row.get("cookie_name"),
            cookie_domain: row.get("cookie_domain"),
            cookie_value: row.get("cookie_value"),
            cookie_path: row.get("cookie_path"),
            creation_time: row.get("creation_time"),
            expiry_time: row.get("expiry_time"),
            last_access_time: row.get("last_access_time"),
            is_secure: row.get("is_secure"),
            is_httponly: row.get("is_httponly"),
            same_site: row.get("same_site"),
            matched_broker_id: row.get("matched_broker_id"),
            scan_timestamp: row.get("scan_timestamp"),
            removal_status: row.get("removal_status"),
            removed_at: row.get("removed_at"),
            cookie_db_filename: row.get("cookie_db_filename"),
        })
        .collect();

    Ok(cookies)
}

/// Get cookies matched to a specific broker.
pub async fn get_cookies_by_broker(
    pool: &Pool<Sqlite>,
    vault_id: &str,
    broker_id: &str,
) -> Result<Vec<BrowserCookie>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, vault_id, browser_type, profile_name, cookie_name, cookie_domain,
                cookie_value, cookie_path, creation_time, expiry_time, last_access_time,
                is_secure, is_httponly, same_site, matched_broker_id,
                scan_timestamp, removal_status, removed_at, cookie_db_filename
         FROM browser_cookies
         WHERE vault_id = ? AND matched_broker_id = ? AND removal_status = 'Pending'
         ORDER BY scan_timestamp DESC",
    )
    .bind(vault_id)
    .bind(broker_id)
    .fetch_all(pool)
    .await?;

    let cookies: Vec<BrowserCookie> = rows
        .into_iter()
        .map(|row| BrowserCookie {
            id: row.get("id"),
            vault_id: row.get("vault_id"),
            browser_type: row.get("browser_type"),
            profile_name: row.get("profile_name"),
            cookie_name: row.get("cookie_name"),
            cookie_domain: row.get("cookie_domain"),
            cookie_value: row.get("cookie_value"),
            cookie_path: row.get("cookie_path"),
            creation_time: row.get("creation_time"),
            expiry_time: row.get("expiry_time"),
            last_access_time: row.get("last_access_time"),
            is_secure: row.get("is_secure"),
            is_httponly: row.get("is_httponly"),
            same_site: row.get("same_site"),
            matched_broker_id: row.get("matched_broker_id"),
            scan_timestamp: row.get("scan_timestamp"),
            removal_status: row.get("removal_status"),
            removed_at: row.get("removed_at"),
            cookie_db_filename: row.get("cookie_db_filename"),
        })
        .collect();

    Ok(cookies)
}

/// Get cookies for a specific domain (typically unmatched cookies).
pub async fn get_cookies_by_domain(
    pool: &Pool<Sqlite>,
    vault_id: &str,
    domain: &str,
) -> Result<Vec<BrowserCookie>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, vault_id, browser_type, profile_name, cookie_name, cookie_domain,
                cookie_value, cookie_path, creation_time, expiry_time, last_access_time,
                is_secure, is_httponly, same_site, matched_broker_id,
                scan_timestamp, removal_status, removed_at, cookie_db_filename
         FROM browser_cookies
         WHERE vault_id = ? AND cookie_domain = ? AND removal_status = 'Pending'
         ORDER BY scan_timestamp DESC",
    )
    .bind(vault_id)
    .bind(domain)
    .fetch_all(pool)
    .await?;

    let cookies: Vec<BrowserCookie> = rows
        .into_iter()
        .map(|row| BrowserCookie {
            id: row.get("id"),
            vault_id: row.get("vault_id"),
            browser_type: row.get("browser_type"),
            profile_name: row.get("profile_name"),
            cookie_name: row.get("cookie_name"),
            cookie_domain: row.get("cookie_domain"),
            cookie_value: row.get("cookie_value"),
            cookie_path: row.get("cookie_path"),
            creation_time: row.get("creation_time"),
            expiry_time: row.get("expiry_time"),
            last_access_time: row.get("last_access_time"),
            is_secure: row.get("is_secure"),
            is_httponly: row.get("is_httponly"),
            same_site: row.get("same_site"),
            matched_broker_id: row.get("matched_broker_id"),
            scan_timestamp: row.get("scan_timestamp"),
            removal_status: row.get("removal_status"),
            removed_at: row.get("removed_at"),
            cookie_db_filename: row.get("cookie_db_filename"),
        })
        .collect();

    Ok(cookies)
}

/// Mark cookies as removed.
pub async fn mark_cookies_removed(
    pool: &Pool<Sqlite>,
    cookie_ids: Vec<String>,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();

    for id in cookie_ids {
        sqlx::query(
            "UPDATE browser_cookies SET removal_status = 'Removed', removed_at = ?
             WHERE id = ?",
        )
        .bind(&now)
        .bind(&id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Get recent cookie scans.
pub async fn get_recent_cookie_scans(
    pool: &Pool<Sqlite>,
    vault_id: &str,
    limit: i32,
) -> Result<Vec<CookieScan>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, vault_id, scan_timestamp, browsers_scanned,
                total_cookies_found, matched_cookies, brokers_matched,
                scan_status, error_message, completed_at
         FROM cookie_scans
         WHERE vault_id = ?
         ORDER BY scan_timestamp DESC
         LIMIT ?",
    )
    .bind(vault_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let scans: Vec<CookieScan> = rows
        .into_iter()
        .filter_map(|row| {
            // nosemgrep: use-zeroize-for-secrets
            let browsers_json: String = row.get("browsers_scanned");
            let browsers_scanned: Vec<String> =
                serde_json::from_str(&browsers_json).unwrap_or_default();

            let brokers_json: Option<String> = row.get("brokers_matched");
            let brokers_matched: Option<Vec<String>> = brokers_json
                .as_ref()
                .and_then(|json| serde_json::from_str(json).ok());

            // nosemgrep: use-zeroize-for-secrets
            let scan_timestamp_str: String = row.get("scan_timestamp");
            let scan_timestamp = DateTime::parse_from_rfc3339(&scan_timestamp_str)
                .ok()?
                .with_timezone(&Utc);

            let completed_at_str: Option<String> = row.get("completed_at");
            let completed_at = completed_at_str.and_then(|ts| {
                DateTime::parse_from_rfc3339(&ts)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });

            Some(CookieScan {
                id: row.get("id"),
                vault_id: row.get("vault_id"),
                scan_timestamp,
                browsers_scanned,
                total_cookies_found: row.get("total_cookies_found"),
                matched_cookies: row.get("matched_cookies"),
                brokers_matched,
                scan_status: row.get("scan_status"),
                error_message: row.get("error_message"),
                completed_at,
            })
        })
        .collect();

    Ok(scans)
}

/// Get unmatched cookies (no broker match) from the most recent scan.
pub async fn get_unmatched_cookies(
    pool: &Pool<Sqlite>,
    vault_id: &str,
) -> Result<Vec<BrowserCookie>, sqlx::Error> {
    // Get the most recent scan timestamp
    let recent_scan = sqlx::query(
        "SELECT scan_timestamp FROM cookie_scans
         WHERE vault_id = ?
         ORDER BY scan_timestamp DESC
         LIMIT 1",
    )
    .bind(vault_id)
    .fetch_optional(pool)
    .await?;

    if let Some(scan_row) = recent_scan {
        // nosemgrep: use-zeroize-for-secrets
        let scan_timestamp: String = scan_row.get("scan_timestamp");

        // Get unmatched cookies from this scan
        let rows = sqlx::query(
            "SELECT id, vault_id, browser_type, profile_name, cookie_name, cookie_domain,
                    cookie_value, cookie_path, creation_time, expiry_time, last_access_time,
                    is_secure, is_httponly, same_site, matched_broker_id,
                    scan_timestamp, removal_status, removed_at, cookie_db_filename
             FROM browser_cookies
             WHERE vault_id = ? AND matched_broker_id IS NULL AND scan_timestamp >= ?
             ORDER BY cookie_domain, cookie_name",
        )
        .bind(vault_id)
        .bind(&scan_timestamp)
        .fetch_all(pool)
        .await?;

        let cookies: Vec<BrowserCookie> = rows
            .into_iter()
            .map(|row| BrowserCookie {
                id: row.get("id"),
                vault_id: row.get("vault_id"),
                browser_type: row.get("browser_type"),
                profile_name: row.get("profile_name"),
                cookie_name: row.get("cookie_name"),
                cookie_domain: row.get("cookie_domain"),
                cookie_value: row.get("cookie_value"),
                cookie_path: row.get("cookie_path"),
                creation_time: row.get("creation_time"),
                expiry_time: row.get("expiry_time"),
                last_access_time: row.get("last_access_time"),
                is_secure: row.get("is_secure"),
                is_httponly: row.get("is_httponly"),
                same_site: row.get("same_site"),
                matched_broker_id: row.get("matched_broker_id"),
                scan_timestamp: row.get("scan_timestamp"),
                removal_status: row.get("removal_status"),
                removed_at: row.get("removed_at"),
                cookie_db_filename: row.get("cookie_db_filename"),
            })
            .collect();

        Ok(cookies)
    } else {
        Ok(vec![])
    }
}

/// Clear all cookie scan data for a vault.
pub async fn clear_cookie_data(pool: &Pool<Sqlite>, vault_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM browser_cookies WHERE vault_id = ?")
        .bind(vault_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM cookie_scans WHERE vault_id = ?")
        .bind(vault_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM cookie_removals WHERE vault_id = ?")
        .bind(vault_id)
        .execute(pool)
        .await?;

    Ok(())
}
