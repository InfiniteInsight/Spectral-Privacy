use serde::{Deserialize, Serialize};

/// Privacy level presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// No automation, no LLM, manual only
    Paranoid,
    /// Local LLM only, automation allowed
    LocalPrivacy,
    /// Cloud LLM + PII filtering, all features
    Balanced,
    /// User-defined feature flags
    Custom,
}

/// Granular feature control flags
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct FeatureFlags {
    /// Allow local LLM usage
    pub allow_local_llm: bool,
    /// Allow cloud LLM usage
    pub allow_cloud_llm: bool,
    /// Allow browser automation
    pub allow_browser_automation: bool,
    /// Allow email sending
    pub allow_email_sending: bool,
    /// Allow IMAP monitoring
    pub allow_imap_monitoring: bool,
    /// Allow PII scanning
    pub allow_pii_scanning: bool,
}

impl FeatureFlags {
    /// Create feature flags from privacy level preset
    #[must_use]
    pub fn from_privacy_level(level: PrivacyLevel) -> Self {
        match level {
            PrivacyLevel::Paranoid => Self {
                allow_local_llm: false,
                allow_cloud_llm: false,
                allow_browser_automation: false,
                allow_email_sending: false,
                allow_imap_monitoring: false,
                allow_pii_scanning: false,
            },
            PrivacyLevel::LocalPrivacy => Self {
                allow_local_llm: true,
                allow_cloud_llm: false,
                allow_browser_automation: true,
                allow_email_sending: true,
                allow_imap_monitoring: true,
                allow_pii_scanning: true,
            },
            PrivacyLevel::Balanced => Self {
                allow_local_llm: true,
                allow_cloud_llm: true,
                allow_browser_automation: true,
                allow_email_sending: true,
                allow_imap_monitoring: true,
                allow_pii_scanning: true,
            },
            PrivacyLevel::Custom => Self::default(),
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::from_privacy_level(PrivacyLevel::Balanced)
    }
}

/// Features that can be permission-checked
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// Local LLM usage
    LocalLlm,
    /// Cloud LLM usage
    CloudLlm,
    /// Browser automation
    BrowserAutomation,
    /// Email sending
    EmailSending,
    /// IMAP monitoring
    ImapMonitoring,
    /// PII scanning
    PiiScanning,
}

/// Result of permission check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionResult {
    /// Permission granted
    Allowed,
    /// Permission denied with reason
    Denied {
        /// Reason for denial
        reason: String,
    },
}

impl PermissionResult {
    /// Check if permission is allowed
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }

    /// Get denial reason if denied
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Denied { reason } => Some(reason),
            Self::Allowed => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_level_serialization() {
        let level = PrivacyLevel::Balanced;
        let json = serde_json::to_string(&level).expect("Failed to serialize PrivacyLevel");
        assert_eq!(json, r#""Balanced""#);

        let deserialized: PrivacyLevel =
            serde_json::from_str(&json).expect("Failed to deserialize PrivacyLevel");
        assert_eq!(deserialized, PrivacyLevel::Balanced);
    }

    #[test]
    fn test_paranoid_feature_flags() {
        let flags = FeatureFlags::from_privacy_level(PrivacyLevel::Paranoid);
        assert!(!flags.allow_local_llm);
        assert!(!flags.allow_cloud_llm);
        assert!(!flags.allow_browser_automation);
    }

    #[test]
    fn test_local_privacy_feature_flags() {
        let flags = FeatureFlags::from_privacy_level(PrivacyLevel::LocalPrivacy);
        assert!(flags.allow_local_llm);
        assert!(!flags.allow_cloud_llm);
        assert!(flags.allow_browser_automation);
    }

    #[test]
    fn test_balanced_feature_flags() {
        let flags = FeatureFlags::from_privacy_level(PrivacyLevel::Balanced);
        assert!(flags.allow_local_llm);
        assert!(flags.allow_cloud_llm);
        assert!(flags.allow_browser_automation);
    }
}
