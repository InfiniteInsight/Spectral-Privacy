//! Permission manager for checking, granting, and revoking permissions.

use crate::{
    audit::{AuditLogger, AuditOutcome},
    presets::PermissionPreset,
    prompts::PermissionPrompt,
    GrantSource, Permission, PermissionError, PermissionGrant, Result,
};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

/// Manages permission grants and handles permission checks.
///
/// The `PermissionManager` maintains the set of granted permissions,
/// handles permission requests, and logs all permission-related events
/// to the audit log.
#[derive(Debug, Clone)]
pub struct PermissionManager {
    grants: Arc<RwLock<HashMap<Permission, PermissionGrant>>>,
    denials: Arc<RwLock<HashSet<Permission>>>,
    audit_logger: Arc<RwLock<AuditLogger>>,
}

impl PermissionManager {
    /// Create a new permission manager with no permissions granted.
    #[must_use]
    pub fn new() -> Self {
        Self {
            grants: Arc::new(RwLock::new(HashMap::new())),
            denials: Arc::new(RwLock::new(HashSet::new())),
            audit_logger: Arc::new(RwLock::new(AuditLogger::new())),
        }
    }

    /// Create a new permission manager initialized with a preset.
    #[must_use]
    pub fn new_with_preset(preset: PermissionPreset) -> Self {
        let manager = Self::new();
        manager.apply_preset(preset, GrantSource::FirstRunWizard);
        manager
    }

    /// Check if a permission is currently granted.
    ///
    /// This performs a simple check without triggering prompts or logging.
    #[must_use]
    pub fn is_granted(&self, permission: Permission) -> bool {
        let grants = self.grants.read().expect("grants lock poisoned");

        if let Some(grant) = grants.get(&permission) {
            !grant.is_expired()
        } else {
            false
        }
    }

    /// Check if a permission is explicitly denied.
    #[must_use]
    pub fn is_denied(&self, permission: Permission) -> bool {
        let denials = self.denials.read().expect("denials lock poisoned");
        denials.contains(&permission)
    }

    /// Request a permission and record the usage.
    ///
    /// This is the main permission check method that should be called
    /// before performing any sensitive action.
    ///
    /// # Errors
    /// Returns `PermissionError::Denied` if the permission is denied.
    pub fn request(&self, permission: Permission) -> Result<()> {
        debug!(permission = %permission.display_name(), "requesting permission");

        // Check explicit denials first
        if self.is_denied(permission) {
            warn!(permission = %permission.display_name(), "permission explicitly denied");
            self.audit_logger
                .write()
                .expect("audit logger lock poisoned")
                .log_permission_check(permission, &AuditOutcome::Denied);
            return Err(PermissionError::Denied(format!(
                "permission {} was previously denied",
                permission.display_name()
            )));
        }

        // Check existing grants
        let mut grants = self.grants.write().expect("grants lock poisoned");

        if let Some(grant) = grants.get_mut(&permission) {
            if grant.is_expired() {
                // Grant expired, remove it
                debug!(permission = %permission.display_name(), "permission grant expired");
                grants.remove(&permission);
            } else {
                // Grant is valid, record usage
                grant.record_use();
                info!(
                    permission = %permission.display_name(),
                    use_count = grant.use_count,
                    "permission granted"
                );
                self.audit_logger
                    .write()
                    .expect("audit logger lock poisoned")
                    .log_permission_check(permission, &AuditOutcome::Allowed);
                return Ok(());
            }
        }

        // Permission not granted
        warn!(permission = %permission.display_name(), "permission not granted");
        self.audit_logger
            .write()
            .expect("audit logger lock poisoned")
            .log_permission_check(permission, &AuditOutcome::Denied);
        Err(PermissionError::Denied(format!(
            "permission {} not granted",
            permission.display_name()
        )))
    }

    /// Grant a permission.
    pub fn grant(&self, permission: Permission, source: GrantSource) {
        info!(permission = %permission.display_name(), ?source, "granting permission");

        let grant = PermissionGrant::new(permission, source);
        self.grants
            .write()
            .expect("grants lock poisoned")
            .insert(permission, grant);

        // Remove from denials if present
        self.denials
            .write()
            .expect("denials lock poisoned")
            .remove(&permission);

        self.audit_logger
            .write()
            .expect("audit logger lock poisoned")
            .log_permission_granted(permission, source);
    }

    /// Deny a permission explicitly.
    pub fn deny(&self, permission: Permission) {
        info!(permission = %permission.display_name(), "denying permission");

        // Remove any existing grant
        self.grants
            .write()
            .expect("grants lock poisoned")
            .remove(&permission);

        // Add to denials
        self.denials
            .write()
            .expect("denials lock poisoned")
            .insert(permission);

        self.audit_logger
            .write()
            .expect("audit logger lock poisoned")
            .log_permission_denied(permission);
    }

    /// Revoke a permission (removes both grant and denial).
    pub fn revoke(&self, permission: Permission) {
        info!(permission = %permission.display_name(), "revoking permission");

        self.grants
            .write()
            .expect("grants lock poisoned")
            .remove(&permission);

        self.denials
            .write()
            .expect("denials lock poisoned")
            .remove(&permission);

        self.audit_logger
            .write()
            .expect("audit logger lock poisoned")
            .log_permission_revoked(permission);
    }

    /// Apply a permission preset.
    pub fn apply_preset(&self, preset: PermissionPreset, source: GrantSource) {
        info!(?preset, ?source, "applying permission preset");

        let permissions = preset.permissions();
        for permission in permissions {
            self.grant(permission, source);
        }
    }

    /// Get all currently granted permissions.
    #[must_use]
    pub fn granted_permissions(&self) -> Vec<Permission> {
        let grants = self.grants.read().expect("grants lock poisoned");
        grants
            .iter()
            .filter(|(_, grant)| !grant.is_expired())
            .map(|(perm, _)| *perm)
            .collect()
    }

    /// Get all explicitly denied permissions.
    #[must_use]
    pub fn denied_permissions(&self) -> Vec<Permission> {
        let denials = self.denials.read().expect("denials lock poisoned");
        denials.iter().copied().collect()
    }

    /// Create a permission prompt for user interaction.
    #[must_use]
    pub fn create_prompt(&self, permission: Permission) -> PermissionPrompt {
        PermissionPrompt::new(permission)
    }

    /// Get a reference to the audit logger.
    #[must_use]
    pub fn audit_logger(&self) -> Arc<RwLock<AuditLogger>> {
        Arc::clone(&self.audit_logger)
    }

    /// Get statistics about a permission's usage.
    #[must_use]
    pub fn get_usage_stats(&self, permission: Permission) -> Option<PermissionUsageStats> {
        let grants = self.grants.read().expect("grants lock poisoned");
        grants.get(&permission).map(|grant| PermissionUsageStats {
            use_count: grant.use_count,
            last_used: grant.last_used,
            granted_at: grant.granted_at,
            granted_by: grant.granted_by,
        })
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about how a permission has been used.
#[derive(Debug, Clone)]
pub struct PermissionUsageStats {
    /// Number of times the permission was used
    pub use_count: u64,

    /// When the permission was last used
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,

    /// When the permission was initially granted
    pub granted_at: chrono::DateTime<chrono::Utc>,

    /// Who granted the permission
    pub granted_by: GrantSource,
}

/// User's decision in response to a permission request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Allow and remember the decision
    Allow,

    /// Allow once (session-only, not implemented yet)
    AllowOnce,

    /// Deny and remember the decision
    Deny,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_new() {
        let manager = PermissionManager::new();
        assert!(!manager.is_granted(Permission::ScanBrokers));
        assert!(!manager.is_denied(Permission::ScanBrokers));
    }

    #[test]
    fn test_grant_and_check() {
        let manager = PermissionManager::new();
        manager.grant(Permission::ScanBrokers, GrantSource::UserExplicit);

        assert!(manager.is_granted(Permission::ScanBrokers));
        assert!(manager.request(Permission::ScanBrokers).is_ok());
    }

    #[test]
    fn test_deny_and_check() {
        let manager = PermissionManager::new();
        manager.deny(Permission::ScanBrokers);

        assert!(manager.is_denied(Permission::ScanBrokers));
        assert!(manager.request(Permission::ScanBrokers).is_err());
    }

    #[test]
    fn test_revoke() {
        let manager = PermissionManager::new();
        manager.grant(Permission::ScanBrokers, GrantSource::UserExplicit);
        assert!(manager.is_granted(Permission::ScanBrokers));

        manager.revoke(Permission::ScanBrokers);
        assert!(!manager.is_granted(Permission::ScanBrokers));
        assert!(!manager.is_denied(Permission::ScanBrokers));
    }

    #[test]
    fn test_grant_overrides_deny() {
        let manager = PermissionManager::new();
        manager.deny(Permission::ScanBrokers);
        assert!(manager.is_denied(Permission::ScanBrokers));

        manager.grant(Permission::ScanBrokers, GrantSource::UserExplicit);
        assert!(manager.is_granted(Permission::ScanBrokers));
        assert!(!manager.is_denied(Permission::ScanBrokers));
    }

    #[test]
    fn test_usage_tracking() {
        let manager = PermissionManager::new();
        manager.grant(Permission::ScanBrokers, GrantSource::UserExplicit);

        // First use
        manager
            .request(Permission::ScanBrokers)
            .expect("should be granted");
        let stats = manager
            .get_usage_stats(Permission::ScanBrokers)
            .expect("should have stats");
        assert_eq!(stats.use_count, 1);

        // Second use
        manager
            .request(Permission::ScanBrokers)
            .expect("should be granted");
        let stats = manager
            .get_usage_stats(Permission::ScanBrokers)
            .expect("should have stats");
        assert_eq!(stats.use_count, 2);
    }

    #[test]
    fn test_preset_application() {
        let manager = PermissionManager::new();
        manager.apply_preset(PermissionPreset::Minimal, GrantSource::FirstRunWizard);

        let granted = manager.granted_permissions();
        assert!(!granted.is_empty());
    }

    #[test]
    fn test_list_permissions() {
        let manager = PermissionManager::new();
        manager.grant(Permission::ScanBrokers, GrantSource::UserExplicit);
        manager.grant(Permission::UseLlmLocal, GrantSource::UserExplicit);
        manager.deny(Permission::UseLlmCloud);

        let granted = manager.granted_permissions();
        assert_eq!(granted.len(), 2);
        assert!(granted.contains(&Permission::ScanBrokers));
        assert!(granted.contains(&Permission::UseLlmLocal));

        let denied = manager.denied_permissions();
        assert_eq!(denied.len(), 1);
        assert!(denied.contains(&Permission::UseLlmCloud));
    }
}
