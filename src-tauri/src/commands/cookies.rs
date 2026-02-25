//! Cookie scanning and removal Tauri commands.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use spectral_cookies::{
    BrokerCookiePattern, Browser, Cookie, CookieMatcher, CookieRemover, CookieScanner,
};
use std::collections::HashMap;
use tauri::State;

/// Response for cookie scan operation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieScanResponse {
    pub scan_id: String,
    pub total_cookies: usize,
    pub matched_cookies: usize,
    pub cookies_by_browser: HashMap<String, usize>,
    pub cookies_by_broker: HashMap<String, usize>,
    pub browsers_scanned: Vec<String>,
    pub timestamp: String,
}

/// Response for individual scanned cookie.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedCookieResponse {
    pub cookie_name: String,
    pub cookie_domain: String,
    pub browser_type: String,
    pub profile_name: String,
    pub matched_broker_id: Option<String>,
    pub is_secure: bool,
    pub is_httponly: bool,
    pub creation_time: Option<i64>,
    pub expiry_time: Option<i64>,
}

/// Response for cookie removal operation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieRemovalResponse {
    pub browser_type: String,
    pub profile_name: String,
    pub cookies_removed: usize,
    pub cookies_failed: usize,
    pub backup_path: Option<String>,
    pub errors: Vec<String>,
}

/// Scan all browsers for tracking cookies.
#[tauri::command]
pub async fn scan_cookies(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<CookieScanResponse, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;

    // Load broker cookie patterns from TOML files
    let broker_patterns = load_broker_cookie_patterns().await?;

    // Create matcher
    let matcher = CookieMatcher::new(broker_patterns).map_err(|e| e.to_string())?;

    // Scan cookies
    let scanner = CookieScanner::with_matcher(matcher);
    let scan_result = scanner
        .scan_all_browsers()
        .map_err(|e| format!("Cookie scan failed: {}", e))?;

    // Store scanned cookies in database
    let mut browser_cookies = Vec::new();
    for scanned_cookie in &scan_result.scanned_cookies {
        let cookie_id = uuid::Uuid::new_v4().to_string();
        let scan_timestamp = chrono::Utc::now().to_rfc3339();

        let browser_cookie = spectral_db::cookies::BrowserCookie {
            id: cookie_id,
            vault_id: vault_id.clone(),
            browser_type: scanned_cookie.browser_type.as_str().to_string(),
            profile_name: Some(scanned_cookie.profile_name.clone()),
            cookie_name: scanned_cookie.cookie.name.clone(),
            cookie_domain: scanned_cookie.cookie.domain.clone(),
            cookie_value: Some(scanned_cookie.cookie.value.clone()),
            cookie_path: scanned_cookie.cookie.path.clone(),
            creation_time: scanned_cookie.cookie.creation_time,
            expiry_time: scanned_cookie.cookie.expiry_time,
            last_access_time: scanned_cookie.cookie.last_access_time,
            is_secure: if scanned_cookie.cookie.is_secure {
                1
            } else {
                0
            },
            is_httponly: if scanned_cookie.cookie.is_httponly {
                1
            } else {
                0
            },
            same_site: scanned_cookie.cookie.same_site.map(|s| s.to_string()),
            matched_broker_id: scanned_cookie.matched_broker_id.clone(),
            scan_timestamp: scan_timestamp.clone(),
            removal_status: "Pending".to_string(),
            removed_at: None,
        };

        browser_cookies.push(browser_cookie);
    }

    // Insert into database
    spectral_db::cookies::insert_scanned_cookies(db.pool(), &vault_id, browser_cookies)
        .await
        .map_err(|e| format!("Failed to save cookies: {}", e))?;

    // Create scan session
    let browsers_scanned: Vec<String> = scan_result
        .cookies_by_browser
        .keys()
        .map(|k| k.to_string())
        .collect();

    let brokers_matched: Option<Vec<String>> = if !scan_result.cookies_by_broker.is_empty() {
        Some(
            scan_result
                .cookies_by_broker
                .keys()
                .map(|k| k.to_string())
                .collect(),
        )
    } else {
        None
    };

    #[allow(clippy::cast_possible_wrap)]
    let scan_id = spectral_db::cookies::create_cookie_scan(
        db.pool(),
        &vault_id,
        browsers_scanned.clone(),
        scan_result.total_cookies as i32,
        scan_result.matched_cookies as i32,
        brokers_matched,
    )
    .await
    .map_err(|e| format!("Failed to create scan: {}", e))?;

    // Log to audit log
    let _ = spectral_db::audit_log::insert_audit_entry(
        db.pool(),
        vault_id.clone(),
        "CookieScanCompleted".to_string(),
        format!(
            "Scanned {} cookies, matched {} to brokers",
            scan_result.total_cookies, scan_result.matched_cookies
        ),
        None,
        "LocalOnly".to_string(),
        "Allowed".to_string(),
    )
    .await;

    Ok(CookieScanResponse {
        scan_id,
        total_cookies: scan_result.total_cookies,
        matched_cookies: scan_result.matched_cookies,
        cookies_by_browser: scan_result.cookies_by_browser,
        cookies_by_broker: scan_result.cookies_by_broker,
        browsers_scanned,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get scanned cookies for a specific broker.
#[tauri::command]
pub async fn get_cookies_for_broker(
    state: State<'_, AppState>,
    vault_id: String,
    broker_id: String,
) -> Result<Vec<ScannedCookieResponse>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;

    let cookies = spectral_db::cookies::get_cookies_by_broker(db.pool(), &vault_id, &broker_id)
        .await
        .map_err(|e| format!("Failed to get cookies: {}", e))?;

    Ok(cookies
        .into_iter()
        .map(|c| ScannedCookieResponse {
            cookie_name: c.cookie_name,
            cookie_domain: c.cookie_domain,
            browser_type: c.browser_type,
            profile_name: c.profile_name.unwrap_or_default(),
            matched_broker_id: c.matched_broker_id,
            is_secure: c.is_secure != 0,
            is_httponly: c.is_httponly != 0,
            creation_time: c.creation_time,
            expiry_time: c.expiry_time,
        })
        .collect())
}

/// Remove cookies for a specific broker.
#[tauri::command]
pub async fn remove_cookies_for_broker(
    state: State<'_, AppState>,
    vault_id: String,
    broker_id: String,
) -> Result<Vec<CookieRemovalResponse>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;

    // Get cookies to remove
    let db_cookies = spectral_db::cookies::get_cookies_by_broker(db.pool(), &vault_id, &broker_id)
        .await
        .map_err(|e| format!("Failed to get cookies: {}", e))?;

    if db_cookies.is_empty() {
        return Ok(vec![]);
    }

    // Group cookies by browser and profile
    let mut cookies_by_browser: HashMap<(String, String), Vec<(String, Cookie)>> = HashMap::new();

    for db_cookie in db_cookies {
        let cookie = Cookie {
            name: db_cookie.cookie_name.clone(),
            value: db_cookie.cookie_value.clone().unwrap_or_default(),
            domain: db_cookie.cookie_domain.clone(),
            path: db_cookie.cookie_path.clone(),
            creation_time: db_cookie.creation_time,
            expiry_time: db_cookie.expiry_time,
            last_access_time: db_cookie.last_access_time,
            is_secure: db_cookie.is_secure != 0,
            is_httponly: db_cookie.is_httponly != 0,
            same_site: db_cookie.same_site.as_ref().and_then(|s| s.parse().ok()),
        };

        let key = (
            db_cookie.browser_type.clone(),
            db_cookie.profile_name.clone().unwrap_or_default(),
        );

        cookies_by_browser
            .entry(key)
            .or_default()
            .push((db_cookie.id.clone(), cookie));
    }

    // Detect browsers and remove cookies
    let browser_profiles = Browser::detect_installed().map_err(|e| e.to_string())?;
    let remover = CookieRemover::new();
    let mut removal_responses = Vec::new();

    for ((browser_type_str, profile_name), cookies_with_ids) in cookies_by_browser {
        // Find matching browser profile
        let profile = browser_profiles.iter().find(|p| {
            p.browser_type.as_str() == browser_type_str && p.profile_name == profile_name
        });

        if let Some(profile) = profile {
            // Check if browser is running
            if Browser::is_browser_running(profile.browser_type) {
                removal_responses.push(CookieRemovalResponse {
                    browser_type: browser_type_str.clone(),
                    profile_name: profile_name.clone(),
                    cookies_removed: 0,
                    cookies_failed: cookies_with_ids.len(),
                    backup_path: None,
                    errors: vec![format!(
                        "{} is running. Please close the browser and try again.",
                        browser_type_str
                    )],
                });
                continue;
            }

            let cookies: Vec<Cookie> = cookies_with_ids.iter().map(|(_, c)| c.clone()).collect();

            // Remove cookies
            match remover.remove_cookies(profile, &cookies) {
                Ok(result) => {
                    // Mark successfully removed cookies in database
                    let removed_ids: Vec<String> = cookies_with_ids
                        .iter()
                        .take(result.cookies_removed)
                        .map(|(id, _)| id.clone())
                        .collect();

                    if !removed_ids.is_empty() {
                        let _ = spectral_db::cookies::mark_cookies_removed(db.pool(), removed_ids)
                            .await;
                    }

                    removal_responses.push(CookieRemovalResponse {
                        browser_type: browser_type_str.clone(),
                        profile_name: profile_name.clone(),
                        cookies_removed: result.cookies_removed,
                        cookies_failed: result.cookies_failed,
                        backup_path: result
                            .backup_path
                            .and_then(|p| p.to_str().map(|s| s.to_string())),
                        errors: result.errors,
                    });
                }
                Err(e) => {
                    removal_responses.push(CookieRemovalResponse {
                        browser_type: browser_type_str.clone(),
                        profile_name: profile_name.clone(),
                        cookies_removed: 0,
                        cookies_failed: cookies_with_ids.len(),
                        backup_path: None,
                        errors: vec![e.to_string()],
                    });
                }
            }
        }
    }

    // Log to audit log
    let total_removed: usize = removal_responses.iter().map(|r| r.cookies_removed).sum();
    let _ = spectral_db::audit_log::insert_audit_entry(
        db.pool(),
        vault_id.clone(),
        "CookiesRemoved".to_string(),
        format!("Removed {} cookies for broker {}", total_removed, broker_id),
        None,
        "LocalOnly".to_string(),
        "Allowed".to_string(),
    )
    .await;

    Ok(removal_responses)
}

/// Get recent cookie scans.
#[tauri::command]
pub async fn get_recent_cookie_scans(
    state: State<'_, AppState>,
    vault_id: String,
    limit: i32,
) -> Result<Vec<CookieScanResponse>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;

    let scans = spectral_db::cookies::get_recent_cookie_scans(db.pool(), &vault_id, limit)
        .await
        .map_err(|e| format!("Failed to get scans: {}", e))?;

    Ok(scans
        .into_iter()
        .map(|scan| CookieScanResponse {
            scan_id: scan.id,
            total_cookies: scan.total_cookies_found as usize,
            matched_cookies: scan.matched_cookies as usize,
            cookies_by_browser: HashMap::new(), // Not stored in scan summary
            cookies_by_broker: HashMap::new(),  // Not stored in scan summary
            browsers_scanned: scan.browsers_scanned,
            timestamp: scan.scan_timestamp.to_rfc3339(),
        })
        .collect())
}

/// Load broker cookie patterns from TOML files.
async fn load_broker_cookie_patterns() -> Result<Vec<BrokerCookiePattern>, String> {
    let broker_defs_path = std::path::PathBuf::from("broker-definitions");

    if !broker_defs_path.exists() {
        return Err("Broker definitions directory not found".to_string());
    }

    let mut patterns = Vec::new();

    // Recursively find all TOML files
    for entry in walkdir::WalkDir::new(&broker_defs_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml")
            && path.file_name().and_then(|s| s.to_str()) != Some("schema.toml")
        {
            // Read TOML file
            let content = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

            // Parse TOML
            let toml_value: toml::Value = toml::from_str(&content)
                .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

            // Extract broker ID
            let broker_id = toml_value
                .get("broker")
                .and_then(|b| b.get("id"))
                .and_then(|id| id.as_str())
                .ok_or_else(|| format!("Missing broker.id in {}", path.display()))?
                .to_string();

            // Check if this broker has cookie patterns
            if let Some(removal) = toml_value.get("removal") {
                if let Some(cookies) = removal.get("cookies") {
                    let cookie_patterns = cookies
                        .get("patterns")
                        .and_then(|p| p.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default();

                    let cookie_domains = cookies
                        .get("domains")
                        .and_then(|d| d.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default();

                    if !cookie_patterns.is_empty() && !cookie_domains.is_empty() {
                        patterns.push(BrokerCookiePattern {
                            broker_id,
                            patterns: cookie_patterns,
                            domains: cookie_domains,
                        });
                    }
                }
            }
        }
    }

    Ok(patterns)
}
