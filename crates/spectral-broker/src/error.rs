//! Error types for the broker subsystem.

use thiserror::Error;

/// Errors that can occur in broker operations.
#[derive(Error, Debug)]
pub enum BrokerError {
    /// Broker definition not found
    #[error("broker definition not found: {broker_id}")]
    NotFound {
        /// The broker ID that was not found
        broker_id: String,
    },

    /// Failed to load broker definition from file
    #[error("failed to load broker definition from {path}: {source}")]
    LoadError {
        /// Path to the definition file
        path: String,
        /// Underlying error
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to parse broker definition TOML
    #[error("failed to parse broker definition TOML in {path}: {source}")]
    ParseError {
        /// Path to the definition file
        path: String,
        /// TOML parse error
        #[source]
        source: toml::de::Error,
    },

    /// Invalid broker definition (validation failed)
    #[error("invalid broker definition for {broker_id}: {reason}")]
    ValidationError {
        /// Broker ID being validated
        broker_id: String,
        /// Reason for validation failure
        reason: String,
    },

    /// Broker definition directory not found
    #[error("broker definitions directory not found at {path}")]
    DirectoryNotFound {
        /// Expected directory path
        path: String,
    },

    /// I/O error while accessing broker definitions
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid broker ID format
    #[error("invalid broker ID: {0}")]
    InvalidId(#[from] spectral_core::SpectralError),
}

/// Result type for broker operations.
pub type Result<T> = std::result::Result<T, BrokerError>;
