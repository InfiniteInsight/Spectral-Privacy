//! Capability registry for LLM-optional architecture.
//!
//! This module provides a way to check which features are available at runtime,
//! allowing the application to gracefully degrade when optional features
//! (like LLM integration) are not configured or available.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Identifies features that can be enabled or disabled at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureId {
    /// LLM-powered natural language interface
    LlmChat,
    /// LLM-guided browser automation
    LlmGuidedBrowsing,
    /// LLM-based form field detection
    LlmFormDetection,
    /// LLM content extraction from broker sites
    LlmContentExtraction,
    /// Browser automation (without LLM)
    BrowserAutomation,
    /// Manual scanning via URL templates
    ManualScanning,
    /// Encrypted vault for PII storage
    EncryptedVault,
    /// Scheduled background tasks
    Scheduling,
    /// Desktop notifications
    Notifications,
    /// Local PII discovery
    LocalDiscovery,
    /// Network telemetry
    NetworkTelemetry,
    /// Plugin system
    Plugins,
}

impl FeatureId {
    /// Get a human-readable name for this feature.
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::LlmChat => "LLM Chat Interface",
            Self::LlmGuidedBrowsing => "LLM-Guided Browsing",
            Self::LlmFormDetection => "LLM Form Detection",
            Self::LlmContentExtraction => "LLM Content Extraction",
            Self::BrowserAutomation => "Browser Automation",
            Self::ManualScanning => "Manual Scanning",
            Self::EncryptedVault => "Encrypted Vault",
            Self::Scheduling => "Background Scheduling",
            Self::Notifications => "Desktop Notifications",
            Self::LocalDiscovery => "Local PII Discovery",
            Self::NetworkTelemetry => "Network Telemetry",
            Self::Plugins => "Plugin System",
        }
    }

    /// Get a description of what this feature provides.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::LlmChat => "Natural language interface for querying and controlling Spectral",
            Self::LlmGuidedBrowsing => {
                "AI-assisted navigation and interaction with data broker sites"
            }
            Self::LlmFormDetection => "Automatic detection and filling of opt-out forms",
            Self::LlmContentExtraction => "Extract structured data from unstructured broker pages",
            Self::BrowserAutomation => "Headless browser for JavaScript-heavy broker sites",
            Self::ManualScanning => "URL template-based scanning without browser automation",
            Self::EncryptedVault => "Secure storage for your personal information",
            Self::Scheduling => "Automatic periodic scans and re-checks",
            Self::Notifications => "Desktop alerts for scan results and removals",
            Self::LocalDiscovery => "Scan local files for PII exposure",
            Self::NetworkTelemetry => "Monitor network connections for privacy insights",
            Self::Plugins => "Extend Spectral with custom broker definitions and integrations",
        }
    }

    /// Get all LLM-dependent features.
    #[must_use]
    pub fn llm_features() -> &'static [FeatureId] {
        &[
            Self::LlmChat,
            Self::LlmGuidedBrowsing,
            Self::LlmFormDetection,
            Self::LlmContentExtraction,
        ]
    }

    /// Check if this is an LLM-dependent feature.
    #[must_use]
    pub fn requires_llm(&self) -> bool {
        Self::llm_features().contains(self)
    }
}

/// Registry tracking which features are currently available.
///
/// In Phase 1, this is a stub that returns false for LLM features.
/// In later phases, it will dynamically check configuration and provider availability.
#[derive(Debug, Clone)]
pub struct CapabilityRegistry {
    /// Set of currently enabled features
    enabled_features: HashSet<FeatureId>,
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityRegistry {
    /// Create a new capability registry with default features enabled.
    ///
    /// Initially, only non-LLM features are enabled:
    /// - `ManualScanning`
    /// - `EncryptedVault`
    #[must_use]
    pub fn new() -> Self {
        let mut enabled_features = HashSet::new();
        // Phase 1: Only non-LLM features are enabled by default
        enabled_features.insert(FeatureId::ManualScanning);
        enabled_features.insert(FeatureId::EncryptedVault);

        Self { enabled_features }
    }

    /// Check if a feature is currently available.
    #[must_use]
    pub fn is_feature_available(&self, feature: FeatureId) -> bool {
        self.enabled_features.contains(&feature)
    }

    /// Enable a feature.
    ///
    /// This will be used in later phases when LLM providers are configured.
    pub fn enable_feature(&mut self, feature: FeatureId) {
        tracing::debug!("Enabling feature: {:?}", feature);
        self.enabled_features.insert(feature);
    }

    /// Disable a feature.
    pub fn disable_feature(&mut self, feature: FeatureId) {
        tracing::debug!("Disabling feature: {:?}", feature);
        self.enabled_features.remove(&feature);
    }

    /// Get all currently enabled features.
    #[must_use]
    pub fn enabled_features(&self) -> Vec<FeatureId> {
        self.enabled_features.iter().copied().collect()
    }

    /// Get all features that could be enabled.
    #[must_use]
    pub fn all_features() -> Vec<FeatureId> {
        vec![
            FeatureId::LlmChat,
            FeatureId::LlmGuidedBrowsing,
            FeatureId::LlmFormDetection,
            FeatureId::LlmContentExtraction,
            FeatureId::BrowserAutomation,
            FeatureId::ManualScanning,
            FeatureId::EncryptedVault,
            FeatureId::Scheduling,
            FeatureId::Notifications,
            FeatureId::LocalDiscovery,
            FeatureId::NetworkTelemetry,
            FeatureId::Plugins,
        ]
    }

    /// Check if any LLM features are available.
    #[must_use]
    pub fn has_llm_capabilities(&self) -> bool {
        FeatureId::llm_features()
            .iter()
            .any(|f| self.is_feature_available(*f))
    }

    /// Enable all LLM features (called when LLM provider is configured).
    pub fn enable_llm_features(&mut self) {
        for feature in FeatureId::llm_features() {
            self.enable_feature(*feature);
        }
    }

    /// Disable all LLM features.
    pub fn disable_llm_features(&mut self) {
        for feature in FeatureId::llm_features() {
            self.disable_feature(*feature);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry() {
        let registry = CapabilityRegistry::new();

        // Non-LLM features should be enabled
        assert!(registry.is_feature_available(FeatureId::ManualScanning));
        assert!(registry.is_feature_available(FeatureId::EncryptedVault));

        // LLM features should be disabled initially
        assert!(!registry.is_feature_available(FeatureId::LlmChat));
        assert!(!registry.is_feature_available(FeatureId::LlmGuidedBrowsing));
    }

    #[test]
    fn test_enable_disable_feature() {
        let mut registry = CapabilityRegistry::new();

        assert!(!registry.is_feature_available(FeatureId::BrowserAutomation));

        registry.enable_feature(FeatureId::BrowserAutomation);
        assert!(registry.is_feature_available(FeatureId::BrowserAutomation));

        registry.disable_feature(FeatureId::BrowserAutomation);
        assert!(!registry.is_feature_available(FeatureId::BrowserAutomation));
    }

    #[test]
    fn test_llm_features() {
        let mut registry = CapabilityRegistry::new();
        assert!(!registry.has_llm_capabilities());

        registry.enable_llm_features();
        assert!(registry.has_llm_capabilities());
        assert!(registry.is_feature_available(FeatureId::LlmChat));
        assert!(registry.is_feature_available(FeatureId::LlmGuidedBrowsing));

        registry.disable_llm_features();
        assert!(!registry.has_llm_capabilities());
    }

    #[test]
    fn test_feature_metadata() {
        assert_eq!(FeatureId::LlmChat.display_name(), "LLM Chat Interface");
        assert!(!FeatureId::LlmChat.description().is_empty());
        assert!(FeatureId::LlmChat.requires_llm());
        assert!(!FeatureId::ManualScanning.requires_llm());
    }

    #[test]
    fn test_all_features() {
        let features = CapabilityRegistry::all_features();
        assert!(!features.is_empty());
        assert!(features.contains(&FeatureId::LlmChat));
        assert!(features.contains(&FeatureId::ManualScanning));
    }

    #[test]
    fn test_enabled_features_list() {
        let mut registry = CapabilityRegistry::new();
        registry.enable_feature(FeatureId::BrowserAutomation);

        let enabled = registry.enabled_features();
        assert!(enabled.contains(&FeatureId::ManualScanning));
        assert!(enabled.contains(&FeatureId::EncryptedVault));
        assert!(enabled.contains(&FeatureId::BrowserAutomation));
    }
}
