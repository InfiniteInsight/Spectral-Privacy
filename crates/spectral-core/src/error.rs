//! Core error types for the Spectral application.
//!
//! This module defines the central error type used across all subsystems.
//! Each subsystem error is represented as a variant for clear error propagation.

use thiserror::Error;

/// Central error type for all Spectral operations.
///
/// Each variant represents an error from a specific subsystem, allowing
/// for clear error propagation and handling across module boundaries.
#[derive(Error, Debug)]
pub enum SpectralError {
    /// Configuration errors (file loading, parsing, validation)
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Vault errors (encryption, decryption, locking)
    #[error("vault error: {0}")]
    Vault(String),

    /// Database errors (connection, queries, migrations)
    #[error("database error: {0}")]
    Database(String),

    /// Broker errors (definitions, scanning, parsing)
    #[error("broker error: {0}")]
    Broker(String),

    /// LLM errors (provider connection, completions, routing)
    #[error("LLM error: {0}")]
    Llm(String),

    /// Browser automation errors (navigation, element not found)
    #[error("browser error: {0}")]
    Browser(String),

    /// Network errors (HTTP requests, DNS)
    #[error("network error: {0}")]
    Network(String),

    /// Permission errors (action not allowed)
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Validation errors (invalid input, constraints)
    #[error("validation error: {0}")]
    Validation(String),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic internal errors
    #[error("internal error: {0}")]
    Internal(String),
}

/// Configuration-specific errors.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Failed to determine config directory path
    #[error("could not determine config directory (XDG base directories not available)")]
    NoConfigDir,

    /// Config file not found (may be first run)
    #[error("config file not found at {path}")]
    NotFound {
        /// Path where config was expected
        path: String,
    },

    /// Failed to parse TOML
    #[error("failed to parse config TOML: {0}")]
    ParseError(#[from] toml::de::Error),

    /// Failed to serialize config
    #[error("failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),

    /// I/O error reading/writing config
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid configuration value
    #[error("invalid config value for {field}: {reason}")]
    InvalidValue {
        /// Field name
        field: String,
        /// Reason for invalidity
        reason: String,
    },
}

/// Result type alias using `SpectralError`.
pub type Result<T> = std::result::Result<T, SpectralError>;

/// Result type alias for configuration operations.
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SpectralError::Validation("invalid email".to_string());
        assert_eq!(err.to_string(), "validation error: invalid email");

        let err = ConfigError::NoConfigDir;
        assert_eq!(
            err.to_string(),
            "could not determine config directory (XDG base directories not available)"
        );
    }

    #[test]
    fn test_error_from_config() {
        let config_err = ConfigError::NoConfigDir;
        let spectral_err: SpectralError = config_err.into();
        assert!(matches!(spectral_err, SpectralError::Config(_)));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let spectral_err: SpectralError = io_err.into();
        assert!(matches!(spectral_err, SpectralError::Io(_)));
    }
}
