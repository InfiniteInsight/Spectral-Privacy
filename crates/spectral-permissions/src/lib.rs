//! Spectral Permissions - Granular permission system for privacy-sensitive operations.
//!
//! This crate provides fine-grained permission controls for all sensitive actions
//! that Spectral can take, including:
//! - Network access (broker scanning, removal submissions, email sending)
//! - LLM usage (cloud vs local, PII filtering)
//! - Local data scanning (filesystem, browser data, emails)
//! - Automation (scheduled scans, automatic removals)
//!
//! # Example
//!
//! ```rust
//! use spectral_permissions::{Permission, PermissionManager, PermissionPreset};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a permission manager with the Balanced preset
//! let manager = PermissionManager::new_with_preset(PermissionPreset::Balanced);
//!
//! // Check if a permission is granted
//! if manager.is_granted(Permission::ScanBrokers) {
//!     println!("Broker scanning is allowed");
//! }
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

mod audit;
mod manager;
mod presets;
mod prompts;

pub use audit::{AuditEntry, AuditLogger, AuditOutcome};
pub use manager::{PermissionDecision, PermissionManager};
pub use presets::PermissionPreset;
pub use prompts::PermissionPrompt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use spectral_core::PiiField;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during permission operations.
#[derive(Error, Debug)]
pub enum PermissionError {
    /// Permission was denied by user or policy
    #[error("permission denied: {0}")]
    Denied(String),

    /// Permission check timed out waiting for user response
    #[error("permission request timed out")]
    Timeout,

    /// Invalid permission request
    #[error("invalid permission request: {0}")]
    Invalid(String),

    /// Audit logging failed
    #[error("audit log error: {0}")]
    AuditError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type for permission operations.
pub type Result<T> = std::result::Result<T, PermissionError>;

/// Individual permission that controls a specific sensitive action.
///
/// Each permission is granular and explicitly named to make it clear
/// what action is being authorized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    // ── Network Permissions ──────────────────────────────
    /// Scan data broker websites for PII
    ScanBrokers,

    /// Submit opt-out/removal forms to data brokers
    SubmitRemovalForms,

    /// Send emails (for removal requests, notifications)
    SendEmails,

    /// Make general HTTP requests
    NetworkAccess,

    // ── LLM Permissions ──────────────────────────────────
    /// Use cloud-based LLM providers (Anthropic, `OpenAI`, etc.)
    UseLlmCloud,

    /// Use local LLM providers (Ollama, llama.cpp)
    UseLlmLocal,

    /// Allow LLM to guide browser automation
    LlmGuidedBrowsing,

    // ── Local Scanning Permissions ───────────────────────
    /// Scan local filesystem for PII
    ScanFilesystem,

    /// Scan browser data (history, saved passwords, cookies)
    ScanBrowserData,

    /// Scan email via IMAP or local files
    ScanEmails,

    // ── Automation Permissions ───────────────────────────
    /// Automatically schedule periodic scans
    AutoScheduleScans,

    /// Automatically submit removal requests without prompting
    AutoSubmitRemovals,

    /// Run in background and show notifications
    BackgroundExecution,
}

impl Permission {
    /// Get a human-readable name for the permission.
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ScanBrokers => "Scan Data Brokers",
            Self::SubmitRemovalForms => "Submit Removal Forms",
            Self::SendEmails => "Send Emails",
            Self::NetworkAccess => "Network Access",
            Self::UseLlmCloud => "Use Cloud LLM",
            Self::UseLlmLocal => "Use Local LLM",
            Self::LlmGuidedBrowsing => "LLM-Guided Browsing",
            Self::ScanFilesystem => "Scan Filesystem",
            Self::ScanBrowserData => "Scan Browser Data",
            Self::ScanEmails => "Scan Emails",
            Self::AutoScheduleScans => "Auto-Schedule Scans",
            Self::AutoSubmitRemovals => "Auto-Submit Removals",
            Self::BackgroundExecution => "Background Execution",
        }
    }

    /// Get a detailed description of what this permission allows.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::ScanBrokers => "Allow Spectral to scan data broker websites to search for your personal information",
            Self::SubmitRemovalForms => "Allow Spectral to automatically submit opt-out and removal forms to data brokers",
            Self::SendEmails => "Allow Spectral to send emails for removal requests and notifications",
            Self::NetworkAccess => "Allow Spectral to make HTTP requests to external websites",
            Self::UseLlmCloud => "Allow using cloud-based AI providers (data sent to external servers)",
            Self::UseLlmLocal => "Allow using locally-running AI models (data stays on your machine)",
            Self::LlmGuidedBrowsing => "Allow AI to guide browser automation for complex broker sites",
            Self::ScanFilesystem => "Allow scanning local files to discover PII exposure",
            Self::ScanBrowserData => "Allow reading browser history, saved passwords, and cookies",
            Self::ScanEmails => "Allow scanning email via IMAP or local mailbox files",
            Self::AutoScheduleScans => "Allow Spectral to automatically schedule periodic broker scans",
            Self::AutoSubmitRemovals => "Allow Spectral to submit removals automatically without asking each time",
            Self::BackgroundExecution => "Allow Spectral to run in the background and show notifications",
        }
    }

    /// Get the risk level of this permission (0-3, higher is riskier).
    #[must_use]
    pub fn risk_level(&self) -> u8 {
        match self {
            // High risk: data leaves the machine
            Self::UseLlmCloud | Self::SendEmails => 3,

            // Medium-high risk: automatic actions or sensitive local data
            Self::SubmitRemovalForms | Self::AutoSubmitRemovals | Self::ScanEmails => 2,

            // Medium risk: network access or local scanning
            Self::ScanBrokers
            | Self::NetworkAccess
            | Self::ScanFilesystem
            | Self::ScanBrowserData
            | Self::LlmGuidedBrowsing => 1,

            // Low risk: local operations or automation
            Self::UseLlmLocal | Self::AutoScheduleScans | Self::BackgroundExecution => 0,
        }
    }

    /// Get which PII fields this permission might access.
    #[must_use]
    pub fn pii_access(&self) -> Vec<PiiField> {
        match self {
            // These can access all PII
            Self::ScanBrokers
            | Self::SubmitRemovalForms
            | Self::SendEmails
            | Self::UseLlmCloud
            | Self::UseLlmLocal
            | Self::LlmGuidedBrowsing
            | Self::AutoSubmitRemovals => vec![
                PiiField::FullName,
                PiiField::Email,
                PiiField::Phone,
                PiiField::Address,
                PiiField::DateOfBirth,
            ],

            // Local scanning can discover PII
            Self::ScanFilesystem | Self::ScanBrowserData | Self::ScanEmails => vec![
                PiiField::FullName,
                PiiField::Email,
                PiiField::Phone,
                PiiField::Address,
            ],

            // These don't directly access PII
            Self::NetworkAccess | Self::AutoScheduleScans | Self::BackgroundExecution => vec![],
        }
    }
}

/// A granted permission with metadata about when and why it was granted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGrant {
    /// Unique identifier for this grant
    pub id: Uuid,

    /// The permission being granted
    pub permission: Permission,

    /// When this permission was granted
    pub granted_at: DateTime<Utc>,

    /// Who granted this permission
    pub granted_by: GrantSource,

    /// Optional expiration time
    pub expires_at: Option<DateTime<Utc>>,

    /// How many times this permission has been used
    pub use_count: u64,

    /// When this permission was last used
    pub last_used: Option<DateTime<Utc>>,
}

impl PermissionGrant {
    /// Create a new permission grant.
    #[must_use]
    pub fn new(permission: Permission, granted_by: GrantSource) -> Self {
        Self {
            id: Uuid::new_v4(),
            permission,
            granted_at: Utc::now(),
            granted_by,
            expires_at: None,
            use_count: 0,
            last_used: None,
        }
    }

    /// Check if this grant has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Record that this permission was used.
    pub fn record_use(&mut self) {
        self.use_count += 1;
        self.last_used = Some(Utc::now());
    }
}

/// Source that granted a permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantSource {
    /// User explicitly granted in UI
    UserExplicit,

    /// Granted via preset during first-run wizard
    FirstRunWizard,

    /// Granted via settings change
    Settings,

    /// System default
    Default,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_display() {
        assert_eq!(Permission::ScanBrokers.display_name(), "Scan Data Brokers");
        assert_eq!(Permission::UseLlmCloud.display_name(), "Use Cloud LLM");
    }

    #[test]
    fn test_permission_risk_levels() {
        assert_eq!(Permission::UseLlmCloud.risk_level(), 3); // High risk
        assert_eq!(Permission::ScanBrokers.risk_level(), 1); // Medium risk
        assert_eq!(Permission::UseLlmLocal.risk_level(), 0); // Low risk
    }

    #[test]
    fn test_permission_pii_access() {
        let pii = Permission::ScanBrokers.pii_access();
        assert!(!pii.is_empty());
        assert!(pii.contains(&PiiField::FullName));

        let no_pii = Permission::BackgroundExecution.pii_access();
        assert!(no_pii.is_empty());
    }

    #[test]
    fn test_permission_grant_expiry() {
        let mut grant = PermissionGrant::new(Permission::ScanBrokers, GrantSource::UserExplicit);
        assert!(!grant.is_expired());

        // Set expiry in the past
        grant.expires_at = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(grant.is_expired());
    }

    #[test]
    fn test_permission_grant_use_tracking() {
        let mut grant = PermissionGrant::new(Permission::ScanBrokers, GrantSource::UserExplicit);
        assert_eq!(grant.use_count, 0);
        assert!(grant.last_used.is_none());

        grant.record_use();
        assert_eq!(grant.use_count, 1);
        assert!(grant.last_used.is_some());

        grant.record_use();
        assert_eq!(grant.use_count, 2);
    }

    #[test]
    fn test_permission_serialization() {
        let permission = Permission::ScanBrokers;
        let json = serde_json::to_string(&permission).expect("serialize permission");
        assert_eq!(json, "\"scan_brokers\"");

        let deserialized: Permission = serde_json::from_str(&json).expect("deserialize permission");
        assert_eq!(deserialized, permission);
    }
}
