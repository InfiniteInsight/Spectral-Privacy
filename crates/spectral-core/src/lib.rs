//! Spectral Core - Foundation crate for the Spectral privacy application.
//!
//! This crate provides shared types, error handling, configuration management,
//! and capability detection that all other Spectral crates depend on.
//!
//! # Modules
//!
//! - [`error`] - Central error types using thiserror
//! - [`config`] - TOML-based configuration with XDG paths
//! - [`types`] - Shared newtypes and enums (`ProfileId`, `BrokerId`, `PiiField`, `Timestamp`)
//! - [`capabilities`] - Feature capability registry for LLM-optional architecture
//!
//! # Example
//!
//! ```rust
//! use spectral_core::{AppConfig, CapabilityRegistry, FeatureId};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load configuration
//! let config = AppConfig::default();
//!
//! // Check feature availability
//! let registry = CapabilityRegistry::new();
//! if registry.is_feature_available(FeatureId::ManualScanning) {
//!     println!("Manual scanning is available");
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

pub mod capabilities;
pub mod config;
pub mod error;
pub mod types;

// Re-export commonly used types
pub use capabilities::{CapabilityRegistry, FeatureId};
pub use config::{
    AppConfig, BrowserConfig, GeneralConfig, LlmConfig, NotificationConfig, ScanningConfig,
    VaultConfig,
};
pub use error::{ConfigError, ConfigResult, Result, SpectralError};
pub use types::{BrokerId, PiiField, ProfileId, Timestamp};
