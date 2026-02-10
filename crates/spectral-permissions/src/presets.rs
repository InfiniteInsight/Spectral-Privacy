//! Permission presets for quick configuration.

use crate::Permission;
use serde::{Deserialize, Serialize};

/// Pre-configured permission profiles for different user preferences.
///
/// These presets provide sensible defaults for different privacy/automation
/// trade-offs, making it easy for users to configure permissions during
/// first-run setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionPreset {
    /// Minimal permissions - manual everything, no automation.
    ///
    /// Best for: Maximum control, paranoid users
    /// - Manual broker scanning only (when user initiates)
    /// - Manual removal submission (user reviews each)
    /// - No LLM usage
    /// - No local scanning
    /// - No automation
    Minimal,

    /// Balanced permissions - automatic scanning, manual removal.
    ///
    /// Best for: Most users, good balance of convenience and control
    /// - Automatic broker scanning
    /// - Manual removal submission
    /// - Local LLM allowed
    /// - Local scanning enabled
    /// - Background execution for scheduled scans
    Balanced,

    /// Maximum permissions - full automation.
    ///
    /// Best for: "Set it and forget it" users who trust the system
    /// - Automatic broker scanning
    /// - Automatic removal submission
    /// - Cloud LLM allowed (with PII filtering)
    /// - All local scanning enabled
    /// - Full automation
    Maximum,
}

impl PermissionPreset {
    /// Get the display name for this preset.
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Minimal => "Minimal",
            Self::Balanced => "Balanced",
            Self::Maximum => "Maximum",
        }
    }

    /// Get a description of this preset.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Minimal => "Manual control over everything. No automation, no LLM, maximum privacy.",
            Self::Balanced => "Automated scanning with manual removal. Local LLM only. Good balance of privacy and convenience.",
            Self::Maximum => "Fully automated. Cloud LLM enabled. Maximum convenience with reasonable privacy protections.",
        }
    }

    /// Get the permissions granted by this preset.
    #[must_use]
    pub fn permissions(&self) -> Vec<Permission> {
        match self {
            Self::Minimal => vec![
                // Only basic manual operations
                Permission::ScanBrokers,
                Permission::SubmitRemovalForms,
            ],

            Self::Balanced => vec![
                // Scanning and manual removal
                Permission::ScanBrokers,
                Permission::SubmitRemovalForms,
                // Local LLM only
                Permission::UseLlmLocal,
                Permission::LlmGuidedBrowsing,
                // Local scanning
                Permission::ScanFilesystem,
                Permission::ScanBrowserData,
                Permission::ScanEmails,
                // Background for scheduled scans
                Permission::AutoScheduleScans,
                Permission::BackgroundExecution,
            ],

            Self::Maximum => vec![
                // Full network access
                Permission::ScanBrokers,
                Permission::SubmitRemovalForms,
                Permission::SendEmails,
                Permission::NetworkAccess,
                // Both local and cloud LLM
                Permission::UseLlmLocal,
                Permission::UseLlmCloud,
                Permission::LlmGuidedBrowsing,
                // All local scanning
                Permission::ScanFilesystem,
                Permission::ScanBrowserData,
                Permission::ScanEmails,
                // Full automation
                Permission::AutoScheduleScans,
                Permission::AutoSubmitRemovals,
                Permission::BackgroundExecution,
            ],
        }
    }

    /// Get the recommended preset for most users.
    #[must_use]
    pub fn recommended() -> Self {
        Self::Balanced
    }

    /// Get all available presets.
    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![Self::Minimal, Self::Balanced, Self::Maximum]
    }
}

impl Default for PermissionPreset {
    fn default() -> Self {
        Self::recommended()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_permissions() {
        let minimal = PermissionPreset::Minimal.permissions();
        let balanced = PermissionPreset::Balanced.permissions();
        let maximum = PermissionPreset::Maximum.permissions();

        // Minimal should have fewest permissions
        assert!(minimal.len() < balanced.len());
        assert!(balanced.len() < maximum.len());

        // Minimal should not have automation
        assert!(!minimal.contains(&Permission::AutoSubmitRemovals));
        assert!(!minimal.contains(&Permission::UseLlmCloud));

        // Balanced should have local LLM but not cloud
        assert!(balanced.contains(&Permission::UseLlmLocal));
        assert!(!balanced.contains(&Permission::UseLlmCloud));

        // Maximum should have everything
        assert!(maximum.contains(&Permission::AutoSubmitRemovals));
        assert!(maximum.contains(&Permission::UseLlmCloud));
    }

    #[test]
    fn test_preset_display() {
        assert_eq!(PermissionPreset::Minimal.display_name(), "Minimal");
        assert_eq!(PermissionPreset::Balanced.display_name(), "Balanced");
        assert_eq!(PermissionPreset::Maximum.display_name(), "Maximum");
    }

    #[test]
    fn test_preset_recommended() {
        assert_eq!(PermissionPreset::recommended(), PermissionPreset::Balanced);
    }

    #[test]
    fn test_preset_all() {
        let all = PermissionPreset::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&PermissionPreset::Minimal));
        assert!(all.contains(&PermissionPreset::Balanced));
        assert!(all.contains(&PermissionPreset::Maximum));
    }

    #[test]
    fn test_preset_serialization() {
        let preset = PermissionPreset::Balanced;
        let json = serde_json::to_string(&preset).expect("serialize preset");
        assert_eq!(json, "\"balanced\"");

        let deserialized: PermissionPreset =
            serde_json::from_str(&json).expect("deserialize preset");
        assert_eq!(deserialized, preset);
    }

    #[test]
    fn test_minimal_no_llm() {
        let permissions = PermissionPreset::Minimal.permissions();
        assert!(!permissions.contains(&Permission::UseLlmLocal));
        assert!(!permissions.contains(&Permission::UseLlmCloud));
    }

    #[test]
    fn test_balanced_no_auto_removal() {
        let permissions = PermissionPreset::Balanced.permissions();
        assert!(!permissions.contains(&Permission::AutoSubmitRemovals));
        assert!(permissions.contains(&Permission::SubmitRemovalForms));
    }

    #[test]
    fn test_maximum_has_all_automation() {
        let permissions = PermissionPreset::Maximum.permissions();
        assert!(permissions.contains(&Permission::AutoScheduleScans));
        assert!(permissions.contains(&Permission::AutoSubmitRemovals));
        assert!(permissions.contains(&Permission::BackgroundExecution));
    }
}
