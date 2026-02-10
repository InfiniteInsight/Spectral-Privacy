//! Spectral Tauri Application Shell
//!
//! This is the thin application shell that registers commands and manages windows.
//! Core business logic lives in the `crates/` directory.

pub mod commands;
mod error;
mod metadata;
pub mod state;

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
        .manage(app_state)
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                // Open devtools in debug builds
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
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
