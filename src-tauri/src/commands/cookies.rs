//! Cookie scanning and removal Tauri commands.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use spectral_cookies::{
    BrokerCookiePattern, Browser, BrowserProfile, Cookie, CookieMatcher, CookieRemover,
    CookieScanner,
};
use std::collections::HashMap;
use tauri::{Emitter, State};

// External dependencies for diagnostics
use dirs;

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
    pub id: String,
    pub cookie_name: String,
    pub cookie_domain: String,
    pub browser_type: String,
    pub profile_name: String,
    pub cookie_db_filename: String,
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

/// Log diagnostic information about browser installation paths.
fn log_browser_diagnostics() {
    if let Some(home) = dirs::home_dir() {
        tracing::info!("Home directory: {}", home.display());

        let chrome_path = home.join("AppData/Local/Google/Chrome/User Data");
        tracing::info!(
            "Chrome path: {} [exists: {}]",
            chrome_path.display(),
            chrome_path.exists()
        );

        let firefox_path = home.join("AppData/Roaming/Mozilla/Firefox/Profiles");
        tracing::info!(
            "Firefox path: {} [exists: {}]",
            firefox_path.display(),
            firefox_path.exists()
        );

        let edge_path = home.join("AppData/Local/Microsoft/Edge/User Data");
        tracing::info!(
            "Edge path: {} [exists: {}]",
            edge_path.display(),
            edge_path.exists()
        );
    } else {
        tracing::error!("Could not determine home directory!");
    }
}

/// Log detected browser profiles for diagnostic purposes.
fn log_detected_browsers() {
    tracing::info!("=== Starting Browser Detection ===");
    match Browser::detect_installed() {
        Ok(profiles) => {
            tracing::info!("✓ Browser detection found {} profiles", profiles.len());
            for profile in &profiles {
                let exists_marker = if profile.cookie_db_path.exists() {
                    "✓"
                } else {
                    "✗"
                };
                tracing::info!(
                    "  {} {} - {} at {}",
                    exists_marker,
                    profile.browser_type,
                    profile.profile_name,
                    profile.cookie_db_path.display()
                );
            }
        }
        Err(e) => {
            tracing::error!("✗ Browser detection failed: {}", e);
        }
    }
    tracing::info!("=== Browser Detection Complete ===");
}

/// Convert scanned cookie results to database records and insert them.
async fn save_scan_results_to_db(
    db_pool: &sqlx::Pool<sqlx::Sqlite>,
    vault_id: &str,
    scan_timestamp: &str,
    scanned_cookies: &[spectral_cookies::ScannedCookie],
) -> Result<(), String> {
    let browser_cookies: Vec<spectral_db::cookies::BrowserCookie> = scanned_cookies
        .iter()
        .map(|scanned_cookie| {
            let cookie_id = uuid::Uuid::new_v4().to_string();

            spectral_db::cookies::BrowserCookie {
                id: cookie_id,
                vault_id: vault_id.to_string(),
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
                scan_timestamp: scan_timestamp.to_string(),
                removal_status: "Pending".to_string(),
                removed_at: None,
                cookie_db_filename: scanned_cookie.cookie_db_filename.clone(),
            }
        })
        .collect();

    spectral_db::cookies::insert_scanned_cookies(db_pool, vault_id, browser_cookies)
        .await
        .map_err(|e| format!("Failed to save cookies: {}", e))
}

/// Create a scan session record in the database and audit log.
async fn create_scan_session_record(
    db_pool: &sqlx::Pool<sqlx::Sqlite>,
    vault_id: &str,
    scan_timestamp: &str,
    scan_result: &spectral_cookies::CookieScanResult,
) -> Result<String, String> {
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
        db_pool,
        vault_id,
        scan_timestamp,
        browsers_scanned,
        scan_result.total_cookies as i32,
        scan_result.matched_cookies as i32,
        brokers_matched,
    )
    .await
    .map_err(|e| format!("Failed to create scan: {}", e))?;

    // Log to audit log
    let _ = spectral_db::audit_log::insert_audit_entry(
        db_pool,
        vault_id.to_string(),
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

    Ok(scan_id)
}

/// Scan all browsers for tracking cookies.
#[tauri::command]
pub async fn scan_cookies(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<CookieScanResponse, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;

    log_browser_diagnostics();

    // Load broker cookie patterns and create matcher
    let broker_patterns = load_broker_cookie_patterns_from_registry(&state).await?;
    tracing::info!("Loaded {} broker cookie patterns", broker_patterns.len());

    let matcher = CookieMatcher::new(broker_patterns).map_err(|e| e.to_string())?;

    log_detected_browsers();

    // Detect browsers
    let profiles = Browser::detect_installed().map_err(|e| e.to_string())?;
    let total_browsers = profiles.len();

    tracing::info!(
        "Starting cookie scan for vault: {} ({} browsers)",
        vault_id,
        total_browsers
    );

    // Scan each browser with progress updates
    let scanner = CookieScanner::with_matcher(matcher);
    let mut all_cookies = Vec::new();
    let mut current = 0;

    for profile in &profiles {
        current += 1;

        // Emit progress event
        let _ = app.emit(
            "cookie-scan:progress",
            serde_json::json!({
                "browser": profile.browser_type.as_str(),
                "profile": &profile.profile_name,
                "current": current,
                "total": total_browsers,
                "message": format!("Scanning {} - {}", profile.browser_type.as_str(), profile.profile_name)
            }),
        );

        match scanner.scan_profile(profile) {
            Ok(cookies) => {
                let cookie_count = cookies.len();
                all_cookies.extend(cookies);

                // Emit completion event with cookie count
                let _ = app.emit(
                    "cookie-scan:progress",
                    serde_json::json!({
                        "browser": profile.browser_type.as_str(),
                        "profile": &profile.profile_name,
                        "current": current,
                        "total": total_browsers,
                        "cookieCount": cookie_count,
                        "message": format!("Scanned {} - {} ({} cookies found)",
                            profile.browser_type.as_str(),
                            profile.profile_name,
                            cookie_count)
                    }),
                );
            }
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

    let scan_result = scanner.build_scan_result(all_cookies);

    tracing::info!(
        "Cookie scan complete: {} total cookies, {} matched, {} browsers scanned",
        scan_result.total_cookies,
        scan_result.matched_cookies,
        scan_result.cookies_by_browser.len()
    );

    // Generate a single timestamp for this entire scan
    let scan_timestamp = chrono::Utc::now().to_rfc3339();

    // Save results to database (all cookies share the same timestamp)
    save_scan_results_to_db(
        db.pool(),
        &vault_id,
        &scan_timestamp,
        &scan_result.scanned_cookies,
    )
    .await?;

    // Create scan session record (using the same timestamp)
    let scan_id =
        create_scan_session_record(db.pool(), &vault_id, &scan_timestamp, &scan_result).await?;

    let browsers_scanned: Vec<String> = scan_result
        .cookies_by_browser
        .keys()
        .map(|k| k.to_string())
        .collect();

    Ok(CookieScanResponse {
        scan_id,
        total_cookies: scan_result.total_cookies,
        matched_cookies: scan_result.matched_cookies,
        cookies_by_browser: scan_result.cookies_by_browser,
        cookies_by_broker: scan_result.cookies_by_broker,
        browsers_scanned,
        timestamp: scan_timestamp,
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
            id: c.id,
            cookie_name: c.cookie_name,
            cookie_domain: c.cookie_domain,
            browser_type: c.browser_type,
            profile_name: c.profile_name.unwrap_or_default(),
            cookie_db_filename: c.cookie_db_filename,
            matched_broker_id: c.matched_broker_id,
            is_secure: c.is_secure != 0,
            is_httponly: c.is_httponly != 0,
            creation_time: c.creation_time,
            expiry_time: c.expiry_time,
        })
        .collect())
}

/// Helper to remove cookies for a single browser profile.
async fn remove_cookies_for_profile(
    db_pool: &sqlx::Pool<sqlx::Sqlite>,
    browser_type_str: &str,
    profile_name: &str,
    cookies_with_ids: Vec<(String, Cookie)>,
    profile: &BrowserProfile,
    remover: &CookieRemover,
) -> CookieRemovalResponse {
    // Check if browser is running
    if Browser::is_browser_running(profile.browser_type) {
        return CookieRemovalResponse {
            browser_type: browser_type_str.to_string(),
            profile_name: profile_name.to_string(),
            cookies_removed: 0,
            cookies_failed: cookies_with_ids.len(),
            backup_path: None,
            errors: vec![format!(
                "{} is running. Please close the browser and try again.",
                browser_type_str
            )],
        };
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
                let _ = spectral_db::cookies::mark_cookies_removed(db_pool, removed_ids).await;
            }

            CookieRemovalResponse {
                browser_type: browser_type_str.to_string(),
                profile_name: profile_name.to_string(),
                cookies_removed: result.cookies_removed,
                cookies_failed: result.cookies_failed,
                backup_path: result
                    .backup_path
                    .and_then(|p| p.to_str().map(|s| s.to_string())),
                errors: result.errors,
            }
        }
        Err(e) => CookieRemovalResponse {
            browser_type: browser_type_str.to_string(),
            profile_name: profile_name.to_string(),
            cookies_removed: 0,
            cookies_failed: cookies_with_ids.len(),
            backup_path: None,
            errors: vec![e.to_string()],
        },
    }
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
            let response = remove_cookies_for_profile(
                db.pool(),
                &browser_type_str,
                &profile_name,
                cookies_with_ids,
                profile,
                &remover,
            )
            .await;
            removal_responses.push(response);
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

/// Remove all cookies for a specific domain (typically unmatched cookies).
#[tauri::command]
pub async fn remove_cookies_for_domain(
    state: State<'_, AppState>,
    vault_id: String,
    domain: String,
) -> Result<Vec<CookieRemovalResponse>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;

    // Get cookies to remove
    let db_cookies = spectral_db::cookies::get_cookies_by_domain(db.pool(), &vault_id, &domain)
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
            let response = remove_cookies_for_profile(
                db.pool(),
                &browser_type_str,
                &profile_name,
                cookies_with_ids,
                profile,
                &remover,
            )
            .await;
            removal_responses.push(response);
        }
    }

    // Log to audit log
    let total_removed: usize = removal_responses.iter().map(|r| r.cookies_removed).sum();
    let _ = spectral_db::audit_log::insert_audit_entry(
        db.pool(),
        vault_id.clone(),
        "CookiesRemoved".to_string(),
        format!("Removed {} cookies for domain {}", total_removed, domain),
        None,
        "LocalOnly".to_string(),
        "Allowed".to_string(),
    )
    .await;

    Ok(removal_responses)
}

/// Remove all scanned cookies (both matched and unmatched).
#[tauri::command]
pub async fn remove_all_cookies(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<CookieRemovalResponse>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;

    // Get all scanned cookies
    let db_cookies = spectral_db::cookies::get_scanned_cookies(db.pool(), &vault_id)
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
        let profile = browser_profiles.iter().find(|p| {
            p.browser_type.as_str() == browser_type_str && p.profile_name == profile_name
        });

        if let Some(profile) = profile {
            let response = remove_cookies_for_profile(
                db.pool(),
                &browser_type_str,
                &profile_name,
                cookies_with_ids,
                profile,
                &remover,
            )
            .await;
            removal_responses.push(response);
        }
    }

    // Log to audit log
    let total_removed: usize = removal_responses.iter().map(|r| r.cookies_removed).sum();
    let _ = spectral_db::audit_log::insert_audit_entry(
        db.pool(),
        vault_id.clone(),
        "AllCookiesRemoved".to_string(),
        format!("Removed all {} scanned cookies", total_removed),
        None,
        "LocalOnly".to_string(),
        "Allowed".to_string(),
    )
    .await;

    Ok(removal_responses)
}

/// Remove all tracking cookies (only matched cookies).
#[tauri::command]
pub async fn remove_all_tracking_cookies(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<CookieRemovalResponse>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;

    // Get all matched cookies (tracking cookies)
    let db_cookies = spectral_db::cookies::get_scanned_cookies(db.pool(), &vault_id)
        .await
        .map_err(|e| format!("Failed to get cookies: {}", e))?
        .into_iter()
        .filter(|c| c.matched_broker_id.is_some())
        .collect::<Vec<_>>();

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
        let profile = browser_profiles.iter().find(|p| {
            p.browser_type.as_str() == browser_type_str && p.profile_name == profile_name
        });

        if let Some(profile) = profile {
            let response = remove_cookies_for_profile(
                db.pool(),
                &browser_type_str,
                &profile_name,
                cookies_with_ids,
                profile,
                &remover,
            )
            .await;
            removal_responses.push(response);
        }
    }

    // Log to audit log
    let total_removed: usize = removal_responses.iter().map(|r| r.cookies_removed).sum();
    let _ = spectral_db::audit_log::insert_audit_entry(
        db.pool(),
        vault_id.clone(),
        "TrackingCookiesRemoved".to_string(),
        format!("Removed all {} tracking cookies", total_removed),
        None,
        "LocalOnly".to_string(),
        "Allowed".to_string(),
    )
    .await;

    Ok(removal_responses)
}

/// Remove a single cookie by its database ID.
#[tauri::command]
pub async fn remove_single_cookie(
    state: State<'_, AppState>,
    vault_id: String,
    cookie_id: String,
) -> Result<CookieRemovalResponse, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;

    // Get the specific cookie
    let db_cookies = spectral_db::cookies::get_scanned_cookies(db.pool(), &vault_id)
        .await
        .map_err(|e| format!("Failed to get cookies: {}", e))?;

    let db_cookie = db_cookies
        .into_iter()
        .find(|c| c.id == cookie_id)
        .ok_or_else(|| format!("Cookie not found: {}", cookie_id))?;

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

    let browser_type_str = db_cookie.browser_type.clone();
    let profile_name = db_cookie.profile_name.clone().unwrap_or_default();

    // Find matching browser profile
    let browser_profiles = Browser::detect_installed().map_err(|e| e.to_string())?;
    let profile = browser_profiles
        .iter()
        .find(|p| p.browser_type.as_str() == browser_type_str && p.profile_name == profile_name)
        .ok_or_else(|| format!("Browser profile not found: {}", browser_type_str))?;

    let remover = CookieRemover::new();
    let response = remove_cookies_for_profile(
        db.pool(),
        &browser_type_str,
        &profile_name,
        vec![(db_cookie.id.clone(), cookie)],
        profile,
        &remover,
    )
    .await;

    // Log to audit log
    if response.cookies_removed > 0 {
        let _ = spectral_db::audit_log::insert_audit_entry(
            db.pool(),
            vault_id,
            "CookieRemoved".to_string(),
            format!(
                "Removed cookie {} from {}",
                db_cookie.cookie_name, browser_type_str
            ),
            None,
            "LocalOnly".to_string(),
            "Allowed".to_string(),
        )
        .await;
    }

    Ok(response)
}

/// Diagnostic: Get browser detection paths and status.
#[tauri::command]
pub async fn diagnose_browser_detection() -> Result<Vec<(String, String, bool)>, String> {
    use spectral_cookies::Browser;

    let mut diagnostics = Vec::new();

    // Check home directory
    if let Some(home) = dirs::home_dir() {
        diagnostics.push((
            "Home Directory".to_string(),
            home.display().to_string(),
            true,
        ));

        // Chrome
        let chrome_path = home.join("AppData/Local/Google/Chrome/User Data");
        diagnostics.push((
            "Chrome Base Path".to_string(),
            chrome_path.display().to_string(),
            chrome_path.exists(),
        ));

        // Firefox
        let firefox_path = home.join("AppData/Roaming/Mozilla/Firefox/Profiles");
        diagnostics.push((
            "Firefox Base Path".to_string(),
            firefox_path.display().to_string(),
            firefox_path.exists(),
        ));

        // Edge
        let edge_path = home.join("AppData/Local/Microsoft/Edge/User Data");
        diagnostics.push((
            "Edge Base Path".to_string(),
            edge_path.display().to_string(),
            edge_path.exists(),
        ));

        // Brave
        let brave_path = home.join("AppData/Local/BraveSoftware/Brave-Browser/User Data");
        diagnostics.push((
            "Brave Base Path".to_string(),
            brave_path.display().to_string(),
            brave_path.exists(),
        ));

        // Firefox-based browsers
        let zen_path = home.join("AppData/Roaming/zen");
        diagnostics.push((
            "Zen Browser Path".to_string(),
            zen_path.display().to_string(),
            zen_path.exists(),
        ));

        let floorp_path = home.join("AppData/Roaming/floorp");
        diagnostics.push((
            "Floorp Browser Path".to_string(),
            floorp_path.display().to_string(),
            floorp_path.exists(),
        ));

        let librewolf_path = home.join("AppData/Roaming/librewolf");
        diagnostics.push((
            "LibreWolf Browser Path".to_string(),
            librewolf_path.display().to_string(),
            librewolf_path.exists(),
        ));
    } else {
        diagnostics.push((
            "Error".to_string(),
            "Could not determine home directory".to_string(),
            false,
        ));
    }

    // Try actual detection
    tracing::info!("Running browser detection diagnostics...");
    match Browser::detect_installed() {
        Ok(profiles) => {
            diagnostics.push((
                "✓ Detected Profiles".to_string(),
                format!("{} browser profiles found", profiles.len()),
                !profiles.is_empty(),
            ));
            for profile in profiles {
                let cookie_exists = profile.cookie_db_path.exists();
                let status = if cookie_exists { "✓" } else { "✗" };
                diagnostics.push((
                    format!(
                        "{} {} - {}",
                        status, profile.browser_type, profile.profile_name
                    ),
                    profile.cookie_db_path.display().to_string(),
                    cookie_exists,
                ));
            }
        }
        Err(e) => {
            diagnostics.push(("✗ Detection Error".to_string(), e.to_string(), false));
        }
    }

    Ok(diagnostics)
}

/// Reconstruct cookie groupings by browser and broker for a specific scan.
async fn reconstruct_cookie_groupings(
    db_pool: &sqlx::Pool<sqlx::Sqlite>,
    vault_id: &str,
    scan_timestamp: &str,
) -> Result<(HashMap<String, usize>, HashMap<String, usize>), String> {
    // Get all cookies from this scan based on scan_timestamp
    let cookies = spectral_db::cookies::get_scanned_cookies(db_pool, vault_id)
        .await
        .map_err(|e| format!("Failed to get cookies: {}", e))?;

    tracing::debug!(
        "[RECONSTRUCT] Total cookies in DB for vault {}: {}",
        vault_id,
        cookies.len()
    );
    tracing::debug!(
        "[RECONSTRUCT] Looking for scan_timestamp: {}",
        scan_timestamp
    );

    // Show sample of actual timestamps in DB
    if !cookies.is_empty() {
        tracing::debug!("[RECONSTRUCT] Sample cookie timestamps from DB:");
        for (i, cookie) in cookies.iter().take(3).enumerate() {
            tracing::debug!("  [{}] {}", i, cookie.scan_timestamp);
        }
    }

    // Filter cookies by scan timestamp (only get cookies from this specific scan)
    let scan_cookies: Vec<_> = cookies
        .into_iter()
        .filter(|c| c.scan_timestamp == scan_timestamp)
        .collect();

    tracing::debug!(
        "[RECONSTRUCT] Cookies matching timestamp: {}",
        scan_cookies.len()
    );

    // Group by browser
    let mut cookies_by_browser: HashMap<String, usize> = HashMap::new();
    for cookie in &scan_cookies {
        *cookies_by_browser
            .entry(cookie.browser_type.clone())
            .or_insert(0) += 1;
    }

    // Group by broker (only matched cookies)
    let mut cookies_by_broker: HashMap<String, usize> = HashMap::new();
    for cookie in &scan_cookies {
        if let Some(broker_id) = &cookie.matched_broker_id {
            *cookies_by_broker.entry(broker_id.clone()).or_insert(0) += 1;
        }
    }

    tracing::info!(
        "[RECONSTRUCT] Results - Browsers: {:?}, Brokers: {:?}",
        cookies_by_browser,
        cookies_by_broker
    );

    Ok((cookies_by_browser, cookies_by_broker))
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

    let mut scan_responses = Vec::new();

    for scan in scans {
        // Reconstruct cookie groupings from the actual cookie data
        let (cookies_by_browser, cookies_by_broker) =
            reconstruct_cookie_groupings(db.pool(), &vault_id, &scan.scan_timestamp.to_rfc3339())
                .await?;

        scan_responses.push(CookieScanResponse {
            scan_id: scan.id,
            total_cookies: scan.total_cookies_found as usize,
            matched_cookies: scan.matched_cookies as usize,
            cookies_by_browser,
            cookies_by_broker,
            browsers_scanned: scan.browsers_scanned,
            timestamp: scan.scan_timestamp.to_rfc3339(),
        });
    }

    Ok(scan_responses)
}

/// Get unmatched cookies from the most recent scan.
#[tauri::command]
pub async fn get_unmatched_cookies(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<ScannedCookieResponse>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not unlocked".to_string())?;

    let db = vault.database().map_err(|e| e.to_string())?;

    let cookies = spectral_db::cookies::get_unmatched_cookies(db.pool(), &vault_id)
        .await
        .map_err(|e| format!("Failed to get unmatched cookies: {}", e))?;

    Ok(cookies
        .into_iter()
        .map(|c| ScannedCookieResponse {
            id: c.id,
            cookie_name: c.cookie_name,
            cookie_domain: c.cookie_domain,
            browser_type: c.browser_type,
            profile_name: c.profile_name.unwrap_or_default(),
            cookie_db_filename: c.cookie_db_filename,
            matched_broker_id: c.matched_broker_id,
            is_secure: c.is_secure != 0,
            is_httponly: c.is_httponly != 0,
            creation_time: c.creation_time,
            expiry_time: c.expiry_time,
        })
        .collect())
}

/// Check if a directory is a Cargo workspace root with broker-definitions.
fn check_for_broker_definitions(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let cargo_toml = dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return None;
    }

    let contents = std::fs::read_to_string(&cargo_toml).ok()?;
    if !contents.contains("[workspace]") {
        return None;
    }

    let definitions_dir = dir.join("broker-definitions");
    if definitions_dir.exists() {
        Some(definitions_dir)
    } else {
        None
    }
}

/// Find the broker-definitions directory using the same logic as BrokerLoader::with_default_dir().
fn find_broker_definitions_dir() -> Result<std::path::PathBuf, String> {
    use std::path::PathBuf;

    // Find workspace root by traversing parent directories
    let mut current_dir = std::env::current_dir().map_err(|e| e.to_string())?;

    loop {
        if let Some(definitions_dir) = check_for_broker_definitions(&current_dir) {
            return Ok(definitions_dir);
        }

        if !current_dir.pop() {
            break;
        }
    }

    // Fallback: try relative path from current directory
    let definitions_dir = PathBuf::from("broker-definitions");
    if definitions_dir.exists() {
        return Ok(definitions_dir);
    }

    Err("Broker definitions directory not found".to_string())
}

/// Load broker cookie patterns by parsing TOML files using the same loader as broker registry.
async fn load_broker_cookie_patterns_from_registry(
    _state: &AppState,
) -> Result<Vec<BrokerCookiePattern>, String> {
    // Find broker definitions directory using the same logic as BrokerLoader::with_default_dir()
    let broker_defs_path = find_broker_definitions_dir()
        .map_err(|e| format!("Failed to find broker definitions: {}", e))?;

    let mut patterns = Vec::new();

    // Recursively find all TOML files
    for entry in walkdir::WalkDir::new(broker_defs_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml")
            && path.file_name().and_then(|s| s.to_str()) != Some("schema.toml")
        {
            if let Some(pattern) = parse_broker_cookie_patterns(path).await? {
                patterns.push(pattern);
            }
        }
    }

    Ok(patterns)
}

/// Parse a single broker TOML file and extract cookie patterns.
async fn parse_broker_cookie_patterns(
    path: &std::path::Path,
) -> Result<Option<BrokerCookiePattern>, String> {
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

    // Check if this broker has cookie patterns in [removal.cookies] section
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
                return Ok(Some(BrokerCookiePattern {
                    broker_id,
                    patterns: cookie_patterns,
                    domains: cookie_domains,
                }));
            }
        }
    }

    Ok(None)
}

/// Helper: Check if running in WSL
#[cfg(target_os = "linux")]
fn is_wsl() -> bool {
    std::path::Path::new("/proc/version").exists()
        && std::fs::read_to_string("/proc/version")
            .map(|s| s.to_lowercase().contains("microsoft"))
            .unwrap_or(false)
}

/// Helper: Try to open directory with native Linux file manager
#[cfg(target_os = "linux")]
fn try_linux_file_manager(db_dir: &std::path::Path) -> Result<(), String> {
    // Try xdg-open first (most common)
    if std::process::Command::new("xdg-open")
        .arg(db_dir)
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    // Fall back to common file managers
    let file_managers = ["nautilus", "dolphin", "thunar", "nemo", "caja"];
    for fm in &file_managers {
        if std::process::Command::new(fm).arg(db_dir).spawn().is_ok() {
            return Ok(());
        }
    }

    Err(format!(
        "No file manager found. Cookie database location: {}",
        db_dir.display()
    ))
}

/// Open the browser cookie database location in file explorer.
#[tauri::command]
pub async fn open_cookie_location(
    browser_type: String,
    profile_name: String,
) -> Result<(), String> {
    // Detect all browser profiles
    let profiles = spectral_cookies::Browser::detect_installed()
        .map_err(|e| format!("Failed to detect browsers: {}", e))?;

    // Find matching profile
    let profile = profiles
        .iter()
        .find(|p| {
            p.browser_type.as_str().eq_ignore_ascii_case(&browser_type)
                && p.profile_name == profile_name
        })
        .ok_or_else(|| {
            format!(
                "Browser profile not found: {} - {}",
                browser_type, profile_name
            )
        })?;

    // Get the parent directory of the cookie database file
    tracing::info!(
        "Cookie database path for {} - {}: {}",
        browser_type,
        profile_name,
        profile.cookie_db_path.display()
    );

    // Canonicalize the path to resolve symlinks and relative paths
    let canonical_db_path = profile
        .cookie_db_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize cookie database path: {}", e))?;

    tracing::info!(
        "Canonical cookie database path: {}",
        canonical_db_path.display()
    );

    let db_dir = canonical_db_path
        .parent()
        .ok_or_else(|| "Could not determine cookie database directory".to_string())?;

    tracing::info!("Opening directory: {}", db_dir.display());

    // Open in file explorer
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(db_dir)
            .spawn()
            .map_err(|e| format!("Failed to open file explorer: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(db_dir)
            .spawn()
            .map_err(|e| format!("Failed to open Finder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        if is_wsl() {
            // In WSL, convert Linux path to Windows path using wslpath
            let output = std::process::Command::new("wslpath")
                .arg("-w")
                .arg(db_dir)
                .output()
                .map_err(|e| format!("Failed to convert WSL path: {}", e))?;

            if !output.status.success() {
                return Err(format!(
                    "wslpath command failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }

            let windows_path = String::from_utf8_lossy(&output.stdout).trim().to_string();

            tracing::info!("Converted WSL path to Windows path: {}", windows_path);

            // Use Windows explorer.exe with the Windows path
            std::process::Command::new("explorer.exe")
                .arg(&windows_path)
                .spawn()
                .map_err(|e| format!("Failed to open Windows Explorer from WSL: {}", e))?;
        } else {
            // Native Linux: Try various file managers
            try_linux_file_manager(db_dir)?;
        }
    }

    tracing::info!(
        "Opened cookie location for {} - {} at: {}",
        browser_type,
        profile_name,
        db_dir.display()
    );

    Ok(())
}
