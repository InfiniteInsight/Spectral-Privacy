//! Discovery commands for local PII scanning

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use spectral_db::scan_logs::{self, ScanConfig as DbScanConfig};
use spectral_discovery::{
    create_scanner_channels, AddressInfo, ScanCommand, ScanConfig, Scanner, UserPii,
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tauri::{Emitter, State};
use tokio::sync::Mutex;
use tracing::info;

#[allow(dead_code)]
struct ActiveScan {
    session_id: String,
    command_tx: crossbeam_channel::Sender<ScanCommand>,
}

static ACTIVE_SCAN: once_cell::sync::Lazy<Arc<Mutex<Option<ActiveScan>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryFinding {
    pub id: String,
    pub source: String,
    pub source_detail: String,
    pub finding_type: String,
    pub risk_level: String,
    pub description: String,
    pub recommended_action: Option<String>,
    pub pii_type: String,
    pub remediated: bool,
    pub ignored: bool,
    pub still_present_after_remediation: bool,
    pub found_at: String,
    pub matched_value: Option<String>,
    pub line_number: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ScanConfigInput {
    pub scan_emails: bool,
    pub scan_phones: bool,
    pub scan_ssn: bool,
    pub scan_addresses: bool,
    pub scan_names: bool,
    pub scan_dob: bool,
    pub custom_directories: Option<Vec<String>>,
}

#[tauri::command]
pub async fn start_discovery_scan<R: tauri::Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
    vault_id: String,
    config: ScanConfigInput,
) -> Result<String, String> {
    info!("start_discovery_scan: vault_id={}", vault_id);

    {
        let active = ACTIVE_SCAN.lock().await;
        if active.is_some() {
            return Err("A scan is already running".to_string());
        }
    }

    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {e}"))?;

    let profile_ids = vault
        .list_profiles()
        .await
        .map_err(|e| format!("Failed to list profiles: {e}"))?;

    if profile_ids.is_empty() {
        return Err("No user profiles found. Create a profile first.".to_string());
    }

    let profile = vault
        .load_profile(&profile_ids[0])
        .await
        .map_err(|e| format!("Failed to load profile: {e}"))?;

    let vault_key = vault
        .encryption_key()
        .map_err(|e| format!("Failed to get vault key: {e}"))?;

    let user_pii = extract_user_pii(&profile, vault_key);

    tracing::info!(
        "Extracted user PII: emails={}, phones={}, ssn={}, addresses={}, names={}, dob={}",
        user_pii.emails.len(),
        user_pii.phones.len(),
        user_pii.ssn.is_some(),
        user_pii.addresses.len(),
        user_pii.names.len(),
        user_pii.date_of_birth.is_some()
    );

    if is_pii_empty(&user_pii) {
        return Err("No PII configured in your profile".to_string());
    }

    let scan_config = ScanConfig {
        scan_emails: config.scan_emails,
        scan_phones: config.scan_phones,
        scan_ssn: config.scan_ssn,
        scan_addresses: config.scan_addresses,
        scan_names: config.scan_names,
        scan_dob: config.scan_dob,
        custom_directories: config
            .custom_directories
            .map(|d| d.into_iter().map(PathBuf::from).collect()),
    };

    let ignored_paths = get_ignored_paths(db.pool(), &vault_id).await;

    let db_config = DbScanConfig {
        scan_emails: scan_config.scan_emails,
        scan_phones: scan_config.scan_phones,
        scan_ssn: scan_config.scan_ssn,
        scan_addresses: scan_config.scan_addresses,
        scan_names: scan_config.scan_names,
        scan_dob: scan_config.scan_dob,
    };

    let session_id = scan_logs::create_scan_session(db.pool(), &vault_id, &db_config)
        .await
        .map_err(|e| format!("Failed to create scan session: {e}"))?;

    let (cmd_tx, cmd_rx, progress_tx, progress_rx) = create_scanner_channels();

    {
        let mut active = ACTIVE_SCAN.lock().await;
        *active = Some(ActiveScan {
            session_id: session_id.clone(),
            command_tx: cmd_tx,
        });
    }

    let scan_dirs = get_scan_directories(scan_config.custom_directories.clone())
        .ok_or_else(|| "Failed to get home directory".to_string())?;

    tracing::info!(
        "Starting scanner thread with {} directories to scan",
        scan_dirs.len()
    );

    let pool = db.pool().clone();
    let vault_id_clone = vault_id.clone();
    let session_id_clone = session_id.clone();
    let app_clone = app.clone();

    // Progress reporter on tokio
    let progress_session_id = session_id.clone();
    tokio::spawn(async move {
        while let Ok(progress) = progress_rx.recv() {
            let _ = app_clone.emit(
                "discovery:progress",
                serde_json::json!({
                    "session_id": progress_session_id,
                    "files_scanned": progress.files_scanned,
                    "files_with_findings": progress.files_with_findings,
                    "current_directory": progress.current_directory,
                    "is_complete": progress.is_complete,
                    "was_stopped": progress.was_stopped,
                }),
            );
            if progress.is_complete {
                break;
            }
        }
    });

    // Scanner on std thread (NOT tokio)
    thread::spawn(move || {
        let scanner = Scanner::new(user_pii, scan_config, ignored_paths, cmd_rx, progress_tx);
        let result = scanner.scan(scan_dirs);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");

        rt.block_on(persist_scan_results(
            &pool,
            &session_id_clone,
            &vault_id_clone,
            &app,
            result,
        ));
    });

    Ok(session_id)
}

#[tauri::command]
pub async fn stop_discovery_scan() -> Result<(), String> {
    let active = ACTIVE_SCAN.lock().await;
    if let Some(scan) = active.as_ref() {
        scan.command_tx
            .send(ScanCommand::Stop)
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No scan running".to_string())
    }
}

#[tauri::command]
pub async fn pause_discovery_scan() -> Result<(), String> {
    let active = ACTIVE_SCAN.lock().await;
    if let Some(scan) = active.as_ref() {
        scan.command_tx
            .send(ScanCommand::Pause)
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No scan running".to_string())
    }
}

#[tauri::command]
pub async fn resume_discovery_scan() -> Result<(), String> {
    let active = ACTIVE_SCAN.lock().await;
    if let Some(scan) = active.as_ref() {
        scan.command_tx
            .send(ScanCommand::Continue)
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No scan running".to_string())
    }
}

#[tauri::command]
pub async fn get_discovery_findings(
    state: State<'_, AppState>,
    vault_id: String,
    include_ignored: Option<bool>,
) -> Result<Vec<DiscoveryFinding>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    let db = vault.database().map_err(|e| format!("DB error: {e}"))?;

    let findings = spectral_db::discovery_findings::get_discovery_findings(
        db.pool(),
        &vault_id,
        include_ignored.unwrap_or(false),
    )
    .await
    .map_err(|e| format!("Query error: {e}"))?;

    Ok(findings
        .into_iter()
        .map(|f| DiscoveryFinding {
            id: f.id,
            source: f.source,
            source_detail: f.source_detail,
            finding_type: f.finding_type,
            risk_level: f.risk_level,
            description: f.description,
            recommended_action: f.recommended_action,
            pii_type: f.pii_type,
            remediated: f.remediated,
            ignored: f.ignored,
            still_present_after_remediation: f.still_present_after_remediation,
            found_at: f.found_at,
            matched_value: f.matched_value,
            line_number: f.line_number,
        })
        .collect())
}

#[tauri::command]
pub async fn mark_finding_remediated(
    state: State<'_, AppState>,
    vault_id: String,
    finding_id: String,
) -> Result<(), String> {
    let vault = state.get_vault(&vault_id).ok_or("Vault not unlocked")?;
    let db = vault.database().map_err(|e| format!("{e}"))?;
    spectral_db::discovery_findings::update_finding_remediated(db.pool(), &finding_id, true)
        .await
        .map_err(|e| format!("{e}"))
}

#[tauri::command]
pub async fn mark_finding_ignored(
    state: State<'_, AppState>,
    vault_id: String,
    finding_id: String,
    ignored: bool,
) -> Result<(), String> {
    let vault = state.get_vault(&vault_id).ok_or("Vault not unlocked")?;
    let db = vault.database().map_err(|e| format!("{e}"))?;
    spectral_db::discovery_findings::mark_finding_ignored(db.pool(), &finding_id, ignored)
        .await
        .map_err(|e| format!("{e}"))
}

#[tauri::command]
pub async fn delete_file(file_path: String) -> Result<(), String> {
    std::fs::remove_file(&file_path).map_err(|e| format!("Delete failed: {e}"))
}

#[tauri::command]
pub fn open_file_location(file_path: String) -> Result<(), String> {
    use std::process::Command;

    let path = std::path::Path::new(&file_path);
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Invalid path: {e}"))?;

    #[cfg(target_os = "windows")]
    Command::new("explorer")
        .args(["/select,", &canonical.to_string_lossy()])
        .spawn()
        .map_err(|e| format!("{e}"))?;

    #[cfg(target_os = "macos")]
    Command::new("open")
        .args(["-R", &canonical.to_string_lossy()])
        .spawn()
        .map_err(|e| format!("{e}"))?;

    #[cfg(target_os = "linux")]
    {
        let is_wsl = std::path::Path::new("/proc/version").exists()
            && std::fs::read_to_string("/proc/version")
                .map(|s| s.to_lowercase().contains("microsoft"))
                .unwrap_or(false);

        if is_wsl {
            let output = Command::new("wslpath")
                .arg("-w")
                .arg(&canonical)
                .output()
                .map_err(|e| format!("{e}"))?;
            let win_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Command::new("explorer.exe")
                .args(["/select,", &win_path])
                .spawn()
                .map_err(|e| format!("{e}"))?;
        } else {
            let parent = canonical.parent().ok_or("No parent")?;
            Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| format!("{e}"))?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn get_scan_log(
    state: State<'_, AppState>,
    vault_id: String,
    session_id: String,
) -> Result<String, String> {
    let vault = state.get_vault(&vault_id).ok_or("Vault not unlocked")?;
    let db = vault.database().map_err(|e| format!("{e}"))?;

    let logs = scan_logs::get_scan_log(db.pool(), &session_id)
        .await
        .map_err(|e| format!("{e}"))?;

    let mut output = format!(
        "PII Scan Log\nSession: {}\nFiles: {}\n\n",
        session_id,
        logs.len()
    );
    for (path, ts, had) in logs {
        let marker = if had { "[FINDING]" } else { "[OK]" };
        output.push_str(&format!("{} {} {}\n", marker, ts, path));
    }

    Ok(output)
}

// Helper functions

fn get_scan_directories(custom_directories: Option<Vec<PathBuf>>) -> Option<Vec<PathBuf>> {
    if let Some(custom) = custom_directories {
        tracing::info!("Using custom scan directories: {:?}", custom);
        Some(custom)
    } else {
        let dirs = directories::UserDirs::new()?;
        let home = dirs.home_dir().to_path_buf();
        tracing::info!("Using home directory for scan: {:?}", home);
        Some(vec![home])
    }
}

async fn persist_scan_results<R: tauri::Runtime>(
    pool: &sqlx::SqlitePool,
    session_id: &str,
    vault_id: &str,
    app: &tauri::AppHandle<R>,
    result: spectral_discovery::ScanResult,
) {
    let mut findings_count = 0;
    let mut files_logged = Vec::new();

    for file_result in &result.findings {
        let path = file_result.path.to_string_lossy().to_string();
        files_logged.push((path.clone(), true));

        for pii_match in &file_result.matches {
            if insert_pii_finding(pool, vault_id, &file_result.path, pii_match)
                .await
                .is_ok()
            {
                findings_count += 1;
            }
        }
    }

    if !files_logged.is_empty() {
        let _ = scan_logs::log_scanned_files_batch(pool, session_id, &files_logged).await;
    }

    let status = if result.was_stopped { "stopped" } else { "completed" };
    let _ = scan_logs::update_scan_session(
        pool,
        session_id,
        status,
        result.files_scanned as i64,
        findings_count,
        None,
    )
    .await;

    let _ = app.emit(
        "discovery:complete",
        serde_json::json!({
            "session_id": session_id,
            "vault_id": vault_id,
            "files_scanned": result.files_scanned,
            "findings_count": findings_count,
            "was_stopped": result.was_stopped,
        }),
    );

    let mut active = ACTIVE_SCAN.lock().await;
    *active = None;
}

fn decrypt_address_fields(
    profile: &spectral_vault::UserProfile,
    key_array: &[u8; 32],
) -> Option<AddressInfo> {
    let addr = AddressInfo {
        street: profile.address.as_ref().and_then(|a| a.decrypt(key_array).ok()),
        city: profile.city.as_ref().and_then(|c| c.decrypt(key_array).ok()),
        state: profile.state.as_ref().and_then(|s| s.decrypt(key_array).ok()),
        zip: profile.zip_code.as_ref().and_then(|z| z.decrypt(key_array).ok()),
    };
    if addr.street.is_some() || addr.zip.is_some() {
        Some(addr)
    } else {
        None
    }
}

fn decrypt_names(profile: &spectral_vault::UserProfile, key_array: &[u8; 32]) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(name) = &profile.full_name {
        if let Ok(n) = name.decrypt(key_array) {
            names.push(n);
        }
    }
    if let Some(first) = &profile.first_name {
        if let Ok(n) = first.decrypt(key_array) {
            names.push(n);
        }
    }
    if let Some(last) = &profile.last_name {
        if let Ok(n) = last.decrypt(key_array) {
            names.push(n);
        }
    }
    names
}

fn extract_user_pii(profile: &spectral_vault::UserProfile, key: &[u8]) -> UserPii {
    let mut pii = UserPii::default();

    // Convert slice to array reference
    let key_array: &[u8; 32] = key.try_into().expect("Key must be 32 bytes");

    #[allow(deprecated)]
    if let Some(email) = &profile.email {
        if let Ok(e) = email.decrypt(key_array) {
            pii.emails.push(e);
        }
    }
    for entry in &profile.email_addresses {
        if let Ok(e) = entry.email.decrypt(key_array) {
            pii.emails.push(e);
        }
    }

    #[allow(deprecated)]
    if let Some(phone) = &profile.phone {
        if let Ok(p) = phone.decrypt(key_array) {
            pii.phones.push(p);
        }
    }
    for entry in &profile.phone_numbers {
        if let Ok(p) = entry.number.decrypt(key_array) {
            pii.phones.push(p);
        }
    }

    if let Some(ssn) = &profile.ssn {
        if let Ok(s) = ssn.decrypt(key_array) {
            pii.ssn = Some(s);
        }
    }

    if let Some(addr) = decrypt_address_fields(profile, key_array) {
        pii.addresses.push(addr);
    }

    pii.names = decrypt_names(profile, key_array);

    if let Some(dob) = &profile.date_of_birth {
        if let Ok(d) = dob.decrypt(key_array) {
            pii.date_of_birth = Some(d);
        }
    }

    pii
}

fn is_pii_empty(pii: &UserPii) -> bool {
    pii.emails.is_empty()
        && pii.phones.is_empty()
        && pii.ssn.is_none()
        && pii.addresses.is_empty()
        && pii.names.is_empty()
        && pii.date_of_birth.is_none()
}

async fn get_ignored_paths(pool: &sqlx::SqlitePool, vault_id: &str) -> HashSet<String> {
    sqlx::query_as::<_, (String,)>(
        "SELECT source_detail FROM discovery_findings WHERE vault_id = ? AND ignored = 1",
    )
    .bind(vault_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(p,)| p)
    .collect()
}

async fn insert_pii_finding(
    pool: &sqlx::SqlitePool,
    vault_id: &str,
    path: &std::path::Path,
    pii_match: &spectral_discovery::PiiMatch,
) -> Result<(), sqlx::Error> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown");
    let description = format!(
        "{} found in {} (line {})",
        pii_match.pii_type.description(),
        file_name,
        pii_match.line_number
    );

    spectral_db::discovery_findings::insert_discovery_finding(
        pool,
        spectral_db::discovery_findings::CreateDiscoveryFinding {
            vault_id: vault_id.to_string(),
            source: "filesystem".to_string(),
            source_detail: path.to_string_lossy().to_string(),
            finding_type: "pii_exposure".to_string(),
            risk_level: pii_match.pii_type.risk_level().to_string(),
            description,
            recommended_action: Some("Review and remove if not needed".to_string()),
            pii_type: pii_match.pii_type.as_str().to_string(),
            matched_value: Some(pii_match.matched_value.clone()),
            line_number: Some(pii_match.line_number as i64),
        },
    )
    .await
    .map(|_| ())
}
