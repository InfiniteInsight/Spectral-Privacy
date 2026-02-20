use thiserror::Error;

/// Error types for privacy operations.
#[derive(Debug, Error)]
pub enum PrivacyError {
    /// Database operation failed.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Database layer error.
    #[error("Database layer error: {0}")]
    DatabaseLayer(#[from] spectral_db::DatabaseError),

    /// JSON serialization/deserialization failed.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Access denied due to privacy settings or permissions.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// LLM provider not found or not configured.
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    /// Invalid privacy configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// LLM request failed.
    #[error("LLM request failed: {0}")]
    LlmRequest(String),
}

/// Result type alias for privacy operations.
pub type Result<T> = std::result::Result<T, PrivacyError>;
