//! User prompt generation for permission requests.

use crate::Permission;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A prompt asking the user for permission to perform an action.
///
/// This structure provides all the information needed to display
/// a permission request to the user in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPrompt {
    /// Unique identifier for this prompt
    pub id: Uuid,

    /// The permission being requested
    pub permission: Permission,

    /// Human-readable title for the prompt
    pub title: String,

    /// Detailed explanation of what this permission allows
    pub message: String,

    /// Risk assessment for this permission
    pub risk_level: RiskLevel,

    /// What data this permission can access
    pub data_access: String,

    /// What happens if the user denies this permission
    pub if_denied: String,

    /// Optional: Alternative actions the user can take
    pub alternatives: Vec<String>,
}

impl PermissionPrompt {
    /// Create a new permission prompt.
    #[must_use]
    pub fn new(permission: Permission) -> Self {
        let id = Uuid::new_v4();
        let title = format!("Allow {}?", permission.display_name());
        let message = permission.description().to_string();
        let risk_level = RiskLevel::from_permission_risk(permission.risk_level());

        let data_access = if permission.pii_access().is_empty() {
            "No personal data access".to_string()
        } else {
            let fields: Vec<String> = permission
                .pii_access()
                .iter()
                .map(|f| f.display_name().to_string())
                .collect();
            format!("May access: {}", fields.join(", "))
        };

        let if_denied = Self::denial_consequence(permission);
        let alternatives = Self::permission_alternatives(permission);

        Self {
            id,
            permission,
            title,
            message,
            risk_level,
            data_access,
            if_denied,
            alternatives,
        }
    }

    /// Get the consequence of denying this permission.
    fn denial_consequence(permission: Permission) -> String {
        match permission {
            Permission::ScanBrokers => {
                "You will need to manually search broker sites to find your information."
                    .to_string()
            }
            Permission::SubmitRemovalForms => {
                "You will need to manually fill out and submit removal forms.".to_string()
            }
            Permission::SendEmails => {
                "Email-based removal requests will not be available.".to_string()
            }
            Permission::NetworkAccess => {
                "Spectral will not be able to connect to external websites.".to_string()
            }
            Permission::UseLlmCloud => {
                "AI features will be limited to local models only (if available).".to_string()
            }
            Permission::UseLlmLocal => "AI-assisted features will not be available.".to_string(),
            Permission::LlmGuidedBrowsing => {
                "Complex broker sites may be harder to navigate automatically.".to_string()
            }
            Permission::ScanFilesystem => {
                "Spectral will not be able to find PII in your local files.".to_string()
            }
            Permission::ScanBrowserData => {
                "Spectral will not check your browser for saved credentials or tracking."
                    .to_string()
            }
            Permission::ScanEmails => {
                "Spectral will not be able to scan your emails for PII exposure.".to_string()
            }
            Permission::AutoScheduleScans => {
                "You will need to manually start each broker scan.".to_string()
            }
            Permission::AutoSubmitRemovals => {
                "You will need to approve each removal submission manually.".to_string()
            }
            Permission::BackgroundExecution => {
                "Spectral will not run in the background or show notifications.".to_string()
            }
        }
    }

    /// Get alternative actions for this permission.
    fn permission_alternatives(permission: Permission) -> Vec<String> {
        match permission {
            Permission::UseLlmCloud => {
                vec![
                    "Use local LLM instead".to_string(),
                    "Disable AI features entirely".to_string(),
                ]
            }
            Permission::ScanFilesystem | Permission::ScanBrowserData | Permission::ScanEmails => {
                vec!["Manually review specific files".to_string()]
            }
            Permission::AutoSubmitRemovals => {
                vec!["Review and approve each removal manually".to_string()]
            }
            _ => vec![],
        }
    }
}

/// Risk level for a permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// Low risk - local operations, no sensitive data
    Low,

    /// Medium risk - network access or local PII scanning
    Medium,

    /// High risk - PII sent to external services
    High,

    /// Critical risk - automatic actions with PII
    Critical,
}

impl RiskLevel {
    /// Convert from numeric risk level (0-3).
    #[must_use]
    pub fn from_permission_risk(level: u8) -> Self {
        match level {
            0 => Self::Low,
            1 => Self::Medium,
            2 => Self::High,
            _ => Self::Critical,
        }
    }

    /// Get display color for UI (Tailwind CSS class).
    #[must_use]
    pub fn color_class(self) -> &'static str {
        match self {
            Self::Low => "text-green-600",
            Self::Medium => "text-yellow-600",
            Self::High => "text-orange-600",
            Self::Critical => "text-red-600",
        }
    }

    /// Get display label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low Risk",
            Self::Medium => "Medium Risk",
            Self::High => "High Risk",
            Self::Critical => "Critical Risk",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_creation() {
        let prompt = PermissionPrompt::new(Permission::ScanBrokers);
        assert_eq!(prompt.permission, Permission::ScanBrokers);
        assert!(prompt.title.contains("Scan Data Brokers"));
        assert!(!prompt.message.is_empty());
        assert!(!prompt.if_denied.is_empty());
    }

    #[test]
    fn test_risk_levels() {
        assert_eq!(RiskLevel::from_permission_risk(0), RiskLevel::Low);
        assert_eq!(RiskLevel::from_permission_risk(1), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_permission_risk(2), RiskLevel::High);
        assert_eq!(RiskLevel::from_permission_risk(3), RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_colors() {
        assert_eq!(RiskLevel::Low.color_class(), "text-green-600");
        assert_eq!(RiskLevel::Critical.color_class(), "text-red-600");
    }

    #[test]
    fn test_high_risk_permissions() {
        let cloud_llm = PermissionPrompt::new(Permission::UseLlmCloud);
        assert!(matches!(cloud_llm.risk_level, RiskLevel::Critical));

        let scan = PermissionPrompt::new(Permission::ScanBrokers);
        assert!(matches!(scan.risk_level, RiskLevel::Medium));
    }

    #[test]
    fn test_data_access_string() {
        let prompt = PermissionPrompt::new(Permission::ScanBrokers);
        assert!(
            prompt.data_access.contains("Full Name") || prompt.data_access.contains("May access")
        );

        let background = PermissionPrompt::new(Permission::BackgroundExecution);
        assert_eq!(background.data_access, "No personal data access");
    }

    #[test]
    fn test_alternatives() {
        let prompt = PermissionPrompt::new(Permission::UseLlmCloud);
        assert!(!prompt.alternatives.is_empty());
        assert!(prompt.alternatives.iter().any(|a| a.contains("local")));

        let basic = PermissionPrompt::new(Permission::ScanBrokers);
        assert!(basic.alternatives.is_empty());
    }

    #[test]
    fn test_prompt_serialization() {
        let prompt = PermissionPrompt::new(Permission::ScanBrokers);
        let json = serde_json::to_string(&prompt).expect("serialize prompt");
        let deserialized: PermissionPrompt =
            serde_json::from_str(&json).expect("deserialize prompt");
        assert_eq!(deserialized.permission, prompt.permission);
        assert_eq!(deserialized.id, prompt.id);
    }
}
