//! Audit logging for permission decisions and usage.

use crate::{GrantSource, Permission};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use uuid::Uuid;

/// Records permission-related events for transparency and debugging.
///
/// The audit log maintains a complete history of permission grants, denials,
/// and usage, allowing users to see exactly what Spectral has been allowed
/// to do and when.
#[derive(Debug)]
pub struct AuditLogger {
    entries: Vec<AuditEntry>,
}

impl AuditLogger {
    /// Create a new audit logger.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Log a permission check.
    pub fn log_permission_check(&mut self, permission: Permission, outcome: &AuditOutcome) {
        debug!(permission = %permission.display_name(), ?outcome, "permission check");

        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: AuditEventType::PermissionCheck {
                permission,
                outcome: outcome.clone(),
            },
            metadata: String::new(),
        };

        self.entries.push(entry);
    }

    /// Log a permission being granted.
    pub fn log_permission_granted(&mut self, permission: Permission, source: GrantSource) {
        info!(permission = %permission.display_name(), ?source, "permission granted");

        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: AuditEventType::PermissionGranted { permission, source },
            metadata: String::new(),
        };

        self.entries.push(entry);
    }

    /// Log a permission being denied.
    pub fn log_permission_denied(&mut self, permission: Permission) {
        info!(permission = %permission.display_name(), "permission denied");

        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: AuditEventType::PermissionDenied { permission },
            metadata: String::new(),
        };

        self.entries.push(entry);
    }

    /// Log a permission being revoked.
    pub fn log_permission_revoked(&mut self, permission: Permission) {
        info!(permission = %permission.display_name(), "permission revoked");

        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: AuditEventType::PermissionRevoked { permission },
            metadata: String::new(),
        };

        self.entries.push(entry);
    }

    /// Get all audit entries.
    #[must_use]
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Get audit entries filtered by permission.
    #[must_use]
    pub fn entries_for_permission(&self, permission: Permission) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.involves_permission(permission))
            .collect()
    }

    /// Get recent audit entries (last N).
    #[must_use]
    pub fn recent_entries(&self, count: usize) -> Vec<&AuditEntry> {
        let start = self.entries.len().saturating_sub(count);
        self.entries[start..].iter().collect()
    }

    /// Clear all audit entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get count of entries.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique identifier for this entry
    pub id: Uuid,

    /// When this event occurred
    pub timestamp: DateTime<Utc>,

    /// Type of event
    pub event_type: AuditEventType,

    /// Additional metadata (JSON string)
    pub metadata: String,
}

impl AuditEntry {
    /// Check if this entry involves a specific permission.
    #[must_use]
    pub fn involves_permission(&self, permission: Permission) -> bool {
        match &self.event_type {
            AuditEventType::PermissionCheck {
                permission: p,
                outcome: _,
            }
            | AuditEventType::PermissionGranted {
                permission: p,
                source: _,
            }
            | AuditEventType::PermissionDenied { permission: p }
            | AuditEventType::PermissionRevoked { permission: p } => *p == permission,
        }
    }

    /// Get a human-readable description of this event.
    #[must_use]
    pub fn description(&self) -> String {
        match &self.event_type {
            AuditEventType::PermissionCheck {
                permission,
                outcome,
            } => {
                format!(
                    "Permission '{}' was {}",
                    permission.display_name(),
                    match outcome {
                        AuditOutcome::Allowed => "allowed",
                        AuditOutcome::Denied => "denied",
                        AuditOutcome::Error(_) => "error",
                    }
                )
            }
            AuditEventType::PermissionGranted { permission, source } => {
                format!(
                    "Permission '{}' was granted by {:?}",
                    permission.display_name(),
                    source
                )
            }
            AuditEventType::PermissionDenied { permission } => {
                format!("Permission '{}' was denied", permission.display_name())
            }
            AuditEventType::PermissionRevoked { permission } => {
                format!("Permission '{}' was revoked", permission.display_name())
            }
        }
    }
}

/// Type of audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum AuditEventType {
    /// A permission was checked during an operation
    PermissionCheck {
        /// Which permission was checked
        permission: Permission,
        /// The outcome of the check
        outcome: AuditOutcome,
    },

    /// A permission was granted
    PermissionGranted {
        /// Which permission was granted
        permission: Permission,
        /// Who granted it
        source: GrantSource,
    },

    /// A permission was explicitly denied
    PermissionDenied {
        /// Which permission was denied
        permission: Permission,
    },

    /// A permission was revoked
    PermissionRevoked {
        /// Which permission was revoked
        permission: Permission,
    },
}

/// Outcome of a permission check or request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    /// Permission was allowed
    Allowed,

    /// Permission was denied
    Denied,

    /// Error occurred during permission check
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_logger_new() {
        let logger = AuditLogger::new();
        assert_eq!(logger.entry_count(), 0);
    }

    #[test]
    fn test_log_permission_check() {
        let mut logger = AuditLogger::new();
        logger.log_permission_check(Permission::ScanBrokers, &AuditOutcome::Allowed);

        assert_eq!(logger.entry_count(), 1);
        let entries = logger.entries();
        assert!(matches!(
            entries[0].event_type,
            AuditEventType::PermissionCheck { .. }
        ));
    }

    #[test]
    fn test_log_permission_granted() {
        let mut logger = AuditLogger::new();
        logger.log_permission_granted(Permission::ScanBrokers, GrantSource::UserExplicit);

        assert_eq!(logger.entry_count(), 1);
        let entries = logger.entries();
        assert!(matches!(
            entries[0].event_type,
            AuditEventType::PermissionGranted { .. }
        ));
    }

    #[test]
    fn test_entries_for_permission() {
        let mut logger = AuditLogger::new();
        logger.log_permission_granted(Permission::ScanBrokers, GrantSource::UserExplicit);
        logger.log_permission_granted(Permission::UseLlmLocal, GrantSource::UserExplicit);
        logger.log_permission_check(Permission::ScanBrokers, &AuditOutcome::Allowed);

        let entries = logger.entries_for_permission(Permission::ScanBrokers);
        assert_eq!(entries.len(), 2);

        let entries = logger.entries_for_permission(Permission::UseLlmLocal);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_recent_entries() {
        let mut logger = AuditLogger::new();
        for i in 0..10 {
            logger.log_permission_check(
                Permission::ScanBrokers,
                if i % 2 == 0 {
                    &AuditOutcome::Allowed
                } else {
                    &AuditOutcome::Denied
                },
            );
        }

        let recent = logger.recent_entries(5);
        assert_eq!(recent.len(), 5);
    }

    #[test]
    fn test_clear() {
        let mut logger = AuditLogger::new();
        logger.log_permission_check(Permission::ScanBrokers, &AuditOutcome::Allowed);
        assert_eq!(logger.entry_count(), 1);

        logger.clear();
        assert_eq!(logger.entry_count(), 0);
    }

    #[test]
    fn test_audit_entry_description() {
        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: AuditEventType::PermissionGranted {
                permission: Permission::ScanBrokers,
                source: GrantSource::UserExplicit,
            },
            metadata: String::new(),
        };

        let desc = entry.description();
        assert!(desc.contains("Scan Data Brokers"));
        assert!(desc.contains("granted"));
    }

    #[test]
    fn test_audit_entry_involves_permission() {
        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: AuditEventType::PermissionGranted {
                permission: Permission::ScanBrokers,
                source: GrantSource::UserExplicit,
            },
            metadata: String::new(),
        };

        assert!(entry.involves_permission(Permission::ScanBrokers));
        assert!(!entry.involves_permission(Permission::UseLlmCloud));
    }

    #[test]
    fn test_multiple_event_types() {
        let mut logger = AuditLogger::new();

        logger.log_permission_granted(Permission::ScanBrokers, GrantSource::FirstRunWizard);
        logger.log_permission_check(Permission::ScanBrokers, &AuditOutcome::Allowed);
        logger.log_permission_revoked(Permission::ScanBrokers);
        logger.log_permission_denied(Permission::UseLlmCloud);

        assert_eq!(logger.entry_count(), 4);

        let broker_entries = logger.entries_for_permission(Permission::ScanBrokers);
        assert_eq!(broker_entries.len(), 3);

        let llm_entries = logger.entries_for_permission(Permission::UseLlmCloud);
        assert_eq!(llm_entries.len(), 1);
    }

    #[test]
    fn test_audit_entry_serialization() {
        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: AuditEventType::PermissionCheck {
                permission: Permission::ScanBrokers,
                outcome: AuditOutcome::Allowed,
            },
            metadata: String::new(),
        };

        let json = serde_json::to_string(&entry).expect("serialize audit entry");
        let deserialized: AuditEntry =
            serde_json::from_str(&json).expect("deserialize audit entry");

        assert_eq!(deserialized.id, entry.id);
        assert!(deserialized.involves_permission(Permission::ScanBrokers));
    }
}
