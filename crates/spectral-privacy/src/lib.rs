//! Spectral Privacy - Centralized privacy controls and settings management
//!
//! This crate provides the Privacy Engine, a centralized enforcement point for all
//! privacy-related decisions in Spectral. The engine manages privacy levels, feature
//! flags, and LLM provider preferences, ensuring consistent privacy controls across
//! the application.
//!
//! ## Architecture
//!
//! The Privacy Engine follows the Orchestrator pattern, serving as a single authority
//! that all services must consult before performing privacy-sensitive operations:
//!
//! - **Privacy Levels**: Predefined presets (Paranoid, `LocalPrivacy`, Balanced, Custom)
//! - **Feature Flags**: Granular controls for LLM, automation, scanning, and email
//! - **LLM Routing**: Privacy-aware provider selection with PII filtering
//! - **Settings Storage**: Encrypted vault-scoped configuration in `SQLCipher` database
//!
//! ## Example
//!
//! ```rust,ignore
//! use spectral_privacy::{PrivacyEngine, PrivacyLevel, Feature};
//!
//! let engine = PrivacyEngine::new(pool);
//!
//! // Set privacy level
//! engine.set_privacy_level(PrivacyLevel::Balanced).await?;
//!
//! // Check permission before using feature
//! let permission = engine.check_permission(Feature::CloudLlm).await?;
//! if permission.is_allowed() {
//!     // Safe to use cloud LLM
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

/// Privacy engine orchestrator.
pub mod engine;
/// Error types for privacy operations.
pub mod error;
/// Privacy-aware LLM router.
pub mod llm_router;
/// LLM provider settings management.
pub mod llm_settings;
/// Core types for privacy controls.
pub mod types;

pub use engine::PrivacyEngine;
pub use error::{PrivacyError, Result};
pub use llm_router::PrivacyAwareLlmRouter;
pub use llm_settings::{
    delete_api_key, get_api_key, get_primary_provider, get_provider_preference, set_api_key,
    set_primary_provider, set_provider_preference, LlmProvider, TaskType,
};
pub use types::{Feature, FeatureFlags, PermissionResult, PrivacyLevel};

// Re-export commonly used LLM types for convenience
pub use spectral_llm::{CompletionRequest, CompletionResponse};
