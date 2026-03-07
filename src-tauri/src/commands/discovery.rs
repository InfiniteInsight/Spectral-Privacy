//! Discovery commands for local PII scanning

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use spectral_discovery::{FileScanResult, PiiMatch, PiiPatterns};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, State};
use tracing::{error, info};

/// Scan control state for pause/resume/stop
#[derive(Debug, Clone)]
enum ScanControl {
    Running,
    Paused,
    Stopped,
}

/// Global scan control state
static SCAN_CONTROL: once_cell::sync::Lazy<Arc<Mutex<ScanControl>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(ScanControl::Running)));

/// Discovery finding response
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
}

/// Wait for scan to be running (handle pause/stop state)
/// Returns true if scan should continue, false if stopped
async fn wait_for_scan_running() -> bool {
    loop {
        let control = SCAN_CONTROL
            .lock()
            .expect("Failed to acquire scan control lock")
            .clone();
        match control {
            ScanControl::Stopped => {
                // Scan was stopped, exit
                return false;
            }
            ScanControl::Paused => {
                // Scan is paused, wait and check again
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }
            ScanControl::Running => {
                // Continue scanning
                return true;
            }
        }
    }
}

/// Scanned file information for batch progress updates
#[derive(Clone, serde::Serialize)]
struct ScannedFileInfo {
    name: String,
    path: String,
}

/// Process a single file during scan
async fn process_scanned_file<R: tauri::Runtime>(
    path: &Path,
    patterns: &PiiPatterns,
    app: &tauri::AppHandle<R>,
    files_scanned: &mut usize,
    results: &mut Vec<FileScanResult>,
    file_batch: &mut Vec<ScannedFileInfo>,
) {
    *files_scanned += 1;

    // Add file to batch
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();
    file_batch.push(ScannedFileInfo {
        name: file_name,
        path: path.to_string_lossy().to_string(),
    });

    // Emit batch progress every 50 files
    if file_batch.len() >= 50 {
        let _ = app.emit(
            "discovery:progress",
            serde_json::json!({
                "files_scanned": *files_scanned,
                "batch": file_batch.clone()
            }),
        );
        file_batch.clear();
    }

    if let Some(result) = spectral_discovery::scan_file(path, patterns).await {
        results.push(result);
    }
}

/// Check if a directory should be excluded from scanning
fn should_exclude_directory(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let name_lower = name.to_lowercase();

        // Exclude common system, cache, and development directories
        matches!(
            name_lower.as_str(),
            // System directories
            "appdata" | "application data" | ".cache" | "cache" | "caches" | ".local" |
            "library" | "application support" | ".config" | "snap" | ".snapshots" |
            // Browser caches and data
            "google" | "mozilla" | "microsoft edge" | "brave-browser" | "firefox" | "chrome" |
            // Cloud sync temp
            "onedrive" | "dropbox" | "google drive" | ".icloud" | "box" | "sync" |
            // Development
            "node_modules" | ".git" | ".svn" | ".hg" | "target" | "build" | "dist" | ".next" |
            "out" | "output" | ".output" | ".nuxt" | ".svelte-kit" | "coverage" |
            // Windows system
            "windows" | "program files" | "program files (x86)" | "programdata" | "$recycle.bin" |
            "system volume information" | "recovery" | "perflogs" |
            // Package managers and tooling
            ".npm" | ".cargo" | ".rustup" | ".gradle" | ".maven" | ".pnpm-store" | ".yarn" |
            ".composer" | ".bundler" | "vendor" |
            // Temp directories
            "temp" | "tmp" | ".tmp" | "temps" |
            // Virtual environments
            "venv" | ".venv" | "env" | ".env" | "virtualenv" | ".virtualenv" | "venvs" |
            // IDE and editors
            ".vscode" | ".idea" | ".vs" | ".eclipse" | ".settings" | ".metadata" |
            // Media and large files
            "steam" | "steamapps" | "games" | "videos" | "movies" | ".steam" |
            // Container and VM
            "docker" | ".docker" | "virtualbox" | ".vagrant" | "vmware" |
            // macOS specific
            ".trash" | ".spotlight-v100" | ".fseventsd" | ".documentrevisions-v100" |
            // Linux specific
            ".thumbnails" | ".gvfs" | ".dbus" | ".mozilla-thunderbird"
        )
    } else {
        false
    }
}

/// Scan directory with progress events
async fn scan_directory_with_progress<R: tauri::Runtime>(
    dir: &Path,
    patterns: &PiiPatterns,
    app: &tauri::AppHandle<R>,
    files_scanned: &mut usize,
) -> Vec<FileScanResult> {
    let max_depth = 10;
    let mut results = Vec::new();
    let mut file_batch = Vec::new();
    scan_recursive(
        dir,
        patterns,
        app,
        files_scanned,
        &mut results,
        &mut file_batch,
        max_depth,
    )
    .await;

    // Emit any remaining files in the batch
    if !file_batch.is_empty() {
        let _ = app.emit(
            "discovery:progress",
            serde_json::json!({
                "files_scanned": *files_scanned,
                "batch": file_batch
            }),
        );
    }

    results
}

/// Recursive scan with progress updates
fn scan_recursive<'a, R: tauri::Runtime + 'static>(
    dir: &'a Path,
    patterns: &'a PiiPatterns,
    app: &'a tauri::AppHandle<R>,
    files_scanned: &'a mut usize,
    results: &'a mut Vec<FileScanResult>,
    file_batch: &'a mut Vec<ScannedFileInfo>,
    max_depth: usize,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    use tokio::fs;

    Box::pin(async move {
        if max_depth == 0 {
            return;
        }

        let mut entries = match fs::read_dir(dir).await {
            Ok(entries) => entries,
            Err(_) => return,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            // Check if scan should continue (handles pause/stop)
            if !wait_for_scan_running().await {
                return;
            }

            let path = entry.path();

            if path.is_dir() {
                // Skip excluded directories
                if should_exclude_directory(&path) {
                    tracing::debug!("Skipping excluded directory: {:?}", path);
                } else {
                    scan_recursive(
                        &path,
                        patterns,
                        app,
                        files_scanned,
                        results,
                        file_batch,
                        max_depth - 1,
                    )
                    .await;
                }
            } else if path.is_file() {
                process_scanned_file(&path, patterns, app, files_scanned, results, file_batch)
                    .await;
            }
        }
    })
}

/// Process scan results and insert findings into the database
async fn process_scan_results(
    results: Vec<FileScanResult>,
    pool: &sqlx::SqlitePool,
    vault_id: &str,
) -> usize {
    let mut findings_count = 0;

    for result in results {
        for pii_match in &result.matches {
            if insert_pii_finding(&result.path, pii_match, pool, vault_id)
                .await
                .is_ok()
            {
                findings_count += 1;
            }
        }
    }

    findings_count
}

/// Insert a PII finding into the database
async fn insert_pii_finding(
    file_path: &Path,
    pii_match: &PiiMatch,
    pool: &sqlx::SqlitePool,
    vault_id: &str,
) -> Result<(), sqlx::Error> {
    let file_name = match file_path.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => {
            tracing::warn!("Could not extract filename from path: {:?}", file_path);
            file_path.to_string_lossy().to_string()
        }
    };

    let description = format!(
        "{} found in file: {} (line {})",
        pii_match.description(),
        file_name,
        pii_match.line_number
    );

    spectral_db::discovery_findings::insert_discovery_finding(
        pool,
        spectral_db::discovery_findings::CreateDiscoveryFinding {
            vault_id: vault_id.to_string(),
            source: "filesystem".to_string(),
            source_detail: file_path.to_string_lossy().to_string(),
            finding_type: "pii_exposure".to_string(),
            risk_level: pii_match.risk_level().to_string(),
            description,
            recommended_action: Some(
                "Review file and remove sensitive information if no longer needed".to_string(),
            ),
            pii_type: pii_match.pii_type_str().to_string(),
            matched_value: Some(pii_match.matched_value.clone()),
            line_number: Some(pii_match.line_number),
        },
    )
    .await
    .map(|_| ())
}

/// Start a discovery scan of local files
///
/// Scans the entire user profile directory for PII by default, or custom
/// directories if specified. Runs in background and emits `discovery:complete`
/// event when done.
#[tauri::command]
pub async fn start_discovery_scan<R: tauri::Runtime>(
    state: State<'_, AppState>,
    app: tauri::AppHandle<R>,
    vault_id: String,
    custom_directories: Option<Vec<String>>,
) -> Result<String, String> {
    info!("start_discovery_scan: vault_id={}", vault_id);

    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {e}"))?;

    // Clone the pool for background task
    let pool = db.pool().clone();
    let vault_id_clone = vault_id.clone();

    // Spawn background scan task
    tokio::spawn(async move {
        info!("Starting filesystem scan for vault {}", vault_id_clone);

        // Reset scan control to Running
        {
            let mut control = SCAN_CONTROL
                .lock()
                .expect("Failed to acquire scan control lock for reset");
            *control = ScanControl::Running;
        }

        let patterns = PiiPatterns::new();

        // Get user home directory
        let home_dir = match directories::UserDirs::new() {
            Some(dirs) => dirs.home_dir().to_path_buf(),
            None => {
                error!("Failed to get user home directory");
                let _ = app.emit(
                    "discovery:error",
                    serde_json::json!({
                        "error": "Failed to get user home directory"
                    }),
                );
                return;
            }
        };

        // Directories to scan
        let scan_dirs: Vec<std::path::PathBuf> = if let Some(custom_dirs) = custom_directories {
            // Use custom directories specified by user
            custom_dirs
                .into_iter()
                .map(std::path::PathBuf::from)
                .collect()
        } else {
            // Scan entire user profile by default
            vec![home_dir.clone()]
        };

        let mut total_findings = 0;
        let mut files_scanned = 0;
        let mut was_stopped = false;

        for dir in scan_dirs {
            // Check if scan was stopped
            {
                let control = SCAN_CONTROL
                    .lock()
                    .expect("Failed to acquire scan control lock");
                if matches!(*control, ScanControl::Stopped) {
                    info!("Scan stopped by user");
                    was_stopped = true;
                    break;
                }
            }

            if !dir.exists() {
                continue;
            }

            // Emit progress event for directory
            let dir_name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");
            let _ = app.emit(
                "discovery:progress",
                serde_json::json!({
                    "directory": dir_name,
                    "path": dir.to_string_lossy(),
                    "files_scanned": files_scanned
                }),
            );

            info!("Scanning directory: {:?}", dir);

            // Scan directory with progress updates
            let results =
                scan_directory_with_progress(&dir, &patterns, &app, &mut files_scanned).await;
            let findings = process_scan_results(results, &pool, &vault_id_clone).await;
            total_findings += findings;
        }

        // Emit final progress update to ensure UI shows correct count
        let _ = app.emit(
            "discovery:progress",
            serde_json::json!({
                "directory": "Finalizing...",
                "path": "",
                "files_scanned": files_scanned
            }),
        );

        if was_stopped {
            info!(
                "Discovery scan stopped: {} findings in {} files",
                total_findings, files_scanned
            );
            // Emit stopped event
            let _ = app.emit(
                "discovery:stopped",
                serde_json::json!({
                    "vault_id": vault_id_clone,
                    "findings_count": total_findings
                }),
            );
        } else {
            info!(
                "Discovery scan complete: {} findings in {} files",
                total_findings, files_scanned
            );
            // Emit completion event
            let _ = app.emit(
                "discovery:complete",
                serde_json::json!({
                    "vault_id": vault_id_clone,
                    "findings_count": total_findings
                }),
            );
        }
    });

    Ok("Scan started".to_string())
}

/// Get all discovery findings for a vault
#[tauri::command]
pub async fn get_discovery_findings(
    state: State<'_, AppState>,
    vault_id: String,
    include_ignored: Option<bool>,
) -> Result<Vec<DiscoveryFinding>, String> {
    let include_ignored = include_ignored.unwrap_or(false);
    info!(
        "get_discovery_findings: vault_id={}, include_ignored={}",
        vault_id, include_ignored
    );

    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {e}"))?;

    // Query findings
    let findings = spectral_db::discovery_findings::get_discovery_findings(
        db.pool(),
        &vault_id,
        include_ignored,
    )
    .await
    .map_err(|e| format!("Failed to get discovery findings: {e}"))?;

    // Convert to response format
    let response: Vec<DiscoveryFinding> = findings
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
        })
        .collect();

    Ok(response)
}

/// Mark a finding as remediated
#[tauri::command]
pub async fn mark_finding_remediated(
    state: State<'_, AppState>,
    vault_id: String,
    finding_id: String,
) -> Result<(), String> {
    info!(
        "mark_finding_remediated: vault_id={}, finding_id={}",
        vault_id, finding_id
    );

    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {e}"))?;

    // Update finding
    spectral_db::discovery_findings::update_finding_remediated(db.pool(), &finding_id, true)
        .await
        .map_err(|e| format!("Failed to mark finding as remediated: {e}"))?;

    Ok(())
}

/// Mark a finding as ignored (false positive or acceptable)
#[tauri::command]
pub async fn mark_finding_ignored(
    state: State<'_, AppState>,
    vault_id: String,
    finding_id: String,
    ignored: bool,
) -> Result<(), String> {
    info!(
        "mark_finding_ignored: vault_id={}, finding_id={}, ignored={}",
        vault_id, finding_id, ignored
    );

    // Get the unlocked vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault '{vault_id}' is not unlocked"))?;

    // Get the vault's database
    let db = vault
        .database()
        .map_err(|e| format!("Failed to get vault database: {e}"))?;

    // Update finding
    spectral_db::discovery_findings::mark_finding_ignored(db.pool(), &finding_id, ignored)
        .await
        .map_err(|e| format!("Failed to mark finding as ignored: {e}"))?;

    Ok(())
}

/// Pause the current discovery scan
#[tauri::command]
pub fn pause_discovery_scan() -> Result<(), String> {
    info!("pause_discovery_scan");
    let mut control = SCAN_CONTROL
        .lock()
        .expect("Failed to acquire scan control lock for pause");
    *control = ScanControl::Paused;
    Ok(())
}

/// Resume a paused discovery scan
#[tauri::command]
pub fn resume_discovery_scan() -> Result<(), String> {
    info!("resume_discovery_scan");
    let mut control = SCAN_CONTROL
        .lock()
        .expect("Failed to acquire scan control lock for resume");
    *control = ScanControl::Running;
    Ok(())
}

/// Stop the current discovery scan
#[tauri::command]
pub fn stop_discovery_scan() -> Result<(), String> {
    info!("stop_discovery_scan");
    let mut control = SCAN_CONTROL
        .lock()
        .expect("Failed to acquire scan control lock for stop");
    *control = ScanControl::Stopped;
    Ok(())
}

/// Open the folder containing a file
#[tauri::command]
pub fn open_file_location(file_path: String) -> Result<(), String> {
    use std::process::Command;

    #[cfg(target_os = "windows")]
    {
        // Windows: Use explorer /select to highlight the file
        Command::new("explorer")
            .args(["/select,", &file_path])
            .spawn()
            .map_err(|e| format!("Failed to open file location: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: Use open -R to reveal the file in Finder
        Command::new("open")
            .args(["-R", &file_path])
            .spawn()
            .map_err(|e| format!("Failed to open file location: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: Open the parent directory with xdg-open
        let path = std::path::Path::new(&file_path);
        let dir = path
            .parent()
            .ok_or_else(|| "Could not determine parent directory".to_string())?;
        Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("Failed to open file location: {}", e))?;
    }

    Ok(())
}
