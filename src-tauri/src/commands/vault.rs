//! Vault management commands.

#![allow(unused_imports, dead_code)] // Stub file - will be used in subsequent tasks

use crate::error::CommandError;
use crate::metadata::VaultMetadata;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

/// Response for vault_status command.
#[derive(Debug, Serialize)]
pub struct VaultStatus {
    pub exists: bool,
    pub unlocked: bool,
    pub display_name: Option<String>,
}

/// Response for list_vaults command.
#[derive(Debug, Serialize)]
pub struct VaultInfo {
    pub vault_id: String,
    pub display_name: String,
    pub created_at: String,
    pub last_accessed: String,
    pub unlocked: bool,
}

// Commands will be implemented in following tasks
