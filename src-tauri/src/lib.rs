//! Spectral Tauri Application Shell
//!
//! This is the thin application shell that registers commands and manages windows.
//! Core business logic lives in the `crates/` directory.

pub mod commands;
mod error;
mod metadata;
pub mod removal_worker;
pub mod state;
pub mod types;

#[cfg(debug_assertions)]
use tauri::Manager;
use tracing::info;

/// Tauri command: Health check
#[tauri::command]
fn health_check() -> String {
    info!("Health check called");
    "ok".to_string()
}

/// Tauri command: Get application version
#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Initialize tracing subscriber for logging
fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,spectral=debug"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(filter)
        .init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    info!("Starting Spectral v{}", env!("CARGO_PKG_VERSION"));

    // Initialize application state
    let app_state = state::AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                // Open devtools in debug builds
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

            // Set up system tray if supported
            if spectral_scheduler::tray::is_tray_supported() {
                use spectral_scheduler::tray;
                use tauri::menu::{MenuBuilder, MenuItemBuilder};
                use tauri::tray::TrayIconBuilder;

                let open_item = MenuItemBuilder::with_id("open", "Open Spectral").build(app)?;
                let scan_item = MenuItemBuilder::with_id("scan_now", "Run Scan Now").build(app)?;
                let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

                let menu = MenuBuilder::new(app)
                    .items(&[&open_item, &scan_item, &quit_item])
                    .build()?;

                TrayIconBuilder::new()
                    .menu(&menu)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        tray::MENU_OPEN => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        tray::MENU_QUIT => {
                            app.exit(0);
                        }
                        _ => {}
                    })
                    .build(app)?;
            } else {
                info!("Tray not supported on this platform — running without tray icon");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health_check,
            get_version,
            commands::vault::vault_create,
            commands::vault::vault_unlock,
            commands::vault::vault_lock,
            commands::vault::vault_status,
            commands::vault::list_vaults,
            commands::vault::rename_vault,
            commands::vault::change_vault_password,
            commands::vault::delete_vault,
            commands::profile::profile_create,
            commands::profile::profile_get,
            commands::profile::profile_update,
            commands::profile::profile_list,
            commands::profile::get_profile_completeness,
            commands::removal::submit_removal,
            commands::removal::mark_attempt_verified,
            commands::scan::start_scan,
            commands::scan::get_scan_status,
            commands::scan::get_findings,
            commands::scan::verify_finding,
            commands::scan::submit_removals_for_confirmed,
            commands::scan::process_removal_batch,
            commands::scan::get_captcha_queue,
            commands::scan::get_failed_queue,
            commands::scan::retry_removal,
            commands::scan::get_removal_attempts_by_scan_job,
            commands::scan::get_removal_job_history,
            commands::scan::get_privacy_score,
            commands::scan::get_dashboard_summary,
            commands::scan::get_removal_evidence,
            commands::scan::send_removal_email,
            commands::settings::test_smtp_connection,
            commands::settings::test_imap_connection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check() {
        assert_eq!(health_check(), "ok");
    }

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
    }
}
