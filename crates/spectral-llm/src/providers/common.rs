//! Common utilities shared across LLM providers.

use crate::error::{LlmError, Result};
use crate::provider::Role;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Build a standard HTTP client with common timeout settings.
///
/// # Arguments
/// * `timeout_secs` - Timeout in seconds (defaults to 60 if not specified)
///
/// # Errors
/// Returns error if the HTTP client cannot be created.
pub fn build_http_client(timeout_secs: Option<u64>) -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(timeout_secs.unwrap_or(60)))
        .build()
        .map_err(|e| LlmError::Internal(format!("failed to create HTTP client: {e}")))
}

/// Convert internal Role enum to standard role string.
///
/// Most providers use "system", "user", "assistant" roles.
#[must_use]
pub fn convert_role_standard(role: Role) -> String {
    match role {
        Role::System => "system".to_string(),
        Role::User => "user".to_string(),
        Role::Assistant => "assistant".to_string(),
    }
}

/// Convert internal Role enum to Gemini-specific role string.
///
/// Gemini uses "user" and "model" instead of "user" and "assistant".
/// System prompts are handled separately.
#[must_use]
pub fn convert_role_gemini(role: Role) -> String {
    match role {
        Role::System | Role::User => "user".to_string(),
        Role::Assistant => "model".to_string(),
    }
}

/// Common message structure for `OpenAI`-compatible APIs.
///
/// Used by `OpenAI`, LM Studio, and other `OpenAI`-compatible providers.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StandardMessage {
    /// The role of the message sender (e.g., "system", "user", "assistant")
    pub role: String,
    /// The text content of the message
    pub content: String,
}

/// Common usage statistics structure.
///
/// Used by multiple providers to report token usage.
#[derive(Debug, Deserialize, Clone)]
pub struct StandardUsage {
    /// Number of tokens in the prompt/input
    pub prompt_tokens: u32,
    /// Number of tokens in the completion/output
    pub completion_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_http_client() {
        // Verify client can be created with custom timeout
        let client = build_http_client(Some(30));
        assert!(client.is_ok());
    }

    #[test]
    fn test_build_http_client_default() {
        // Verify client can be created with default timeout
        let client = build_http_client(None);
        assert!(client.is_ok());
    }

    #[test]
    fn test_convert_role_standard() {
        assert_eq!(convert_role_standard(Role::System), "system");
        assert_eq!(convert_role_standard(Role::User), "user");
        assert_eq!(convert_role_standard(Role::Assistant), "assistant");
    }

    #[test]
    fn test_convert_role_gemini() {
        assert_eq!(convert_role_gemini(Role::System), "user");
        assert_eq!(convert_role_gemini(Role::User), "user");
        assert_eq!(convert_role_gemini(Role::Assistant), "model");
    }
}
