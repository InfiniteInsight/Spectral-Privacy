//! LM Studio local provider implementation.

use super::common::{
    build_http_client, convert_role_standard, streaming_not_implemented, StandardMessage,
    StandardUsage,
};
use crate::error::{LlmError, Result};
use crate::provider::{
    CompletionRequest, CompletionResponse, CompletionStream, LlmProvider, ProviderCapabilities,
    Usage,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// LM Studio local provider.
///
/// LM Studio provides an `OpenAI`-compatible API for local models.
/// Default endpoint is `http://localhost:1234`.
pub struct LmStudioProvider {
    model: String,
    client: Client,
    base_url: String,
}

impl LmStudioProvider {
    /// Create a new LM Studio provider with default settings.
    ///
    /// # Errors
    /// Returns error if the HTTP client cannot be created.
    pub fn new() -> Result<Self> {
        Self::with_url("http://localhost:1234")
    }

    /// Create a new LM Studio provider with a custom URL.
    ///
    /// # Errors
    /// Returns error if the HTTP client cannot be created.
    pub fn with_url(base_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            model: "local-model".to_string(), // Will be determined at runtime
            client: build_http_client(Some(120))?,
            base_url: base_url.into(),
        })
    }

    /// Check if LM Studio is available at the configured endpoint.
    ///
    /// Performs a lightweight health check to detect if LM Studio is running.
    ///
    /// # Errors
    /// Returns error if the health check request fails.
    pub async fn is_available(&self) -> Result<bool> {
        // Try to get models list - if this succeeds, LM Studio is available
        let result = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .send()
            .await;

        Ok(result.is_ok() && result.map(|r| r.status().is_success()).unwrap_or(false))
    }

    /// Convert internal request to LM Studio (OpenAI-compatible) API format.
    #[allow(clippy::unused_self)]
    fn to_api_request(&self, request: &CompletionRequest) -> LmStudioRequest {
        let mut messages: Vec<StandardMessage> = Vec::new();

        // Add system message if present
        if let Some(system) = &request.system_prompt {
            messages.push(StandardMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        // Add conversation messages
        for message in &request.messages {
            messages.push(StandardMessage {
                role: convert_role_standard(message.role),
                content: message.content.clone(),
            });
        }

        LmStudioRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stop: if request.stop_sequences.is_empty() {
                None
            } else {
                Some(request.stop_sequences.clone())
            },
        }
    }

    /// Convert LM Studio API response to internal format.
    #[allow(dead_code)]
    fn convert_api_response(response: LmStudioResponse) -> Result<CompletionResponse> {
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::ParseError {
                provider: "lmstudio".to_string(),
                message: "no choices in response".to_string(),
            })?;

        Ok(CompletionResponse {
            content: choice.message.content,
            model: response.model,
            stop_reason: choice.finish_reason,
            usage: response.usage.map(|u| Usage {
                input_tokens: u.prompt_tokens,
                output_tokens: u.completion_tokens,
            }),
        })
    }
}

impl Default for LmStudioProvider {
    fn default() -> Self {
        Self::new().expect("create default LM Studio provider")
    }
}

#[async_trait]
impl LlmProvider for LmStudioProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let api_request = self.to_api_request(&request);

        // Make actual API call to LM Studio
        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Content-Type", "application/json")
            .json(&api_request)
            .send()
            .await?;

        // Check for HTTP errors
        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmError::ApiError {
                provider: "lmstudio".to_string(),
                status: status.as_u16(),
                message: error_text,
            });
        }

        // Parse the JSON response
        let api_response: LmStudioResponse =
            response.json().await.map_err(|e| LlmError::ParseError {
                provider: "lmstudio".to_string(),
                message: format!("Failed to parse response: {e}"),
            })?;

        // Convert to internal format
        Self::convert_api_response(api_response)
    }

    async fn stream(&self, _request: CompletionRequest) -> Result<CompletionStream> {
        streaming_not_implemented("LM Studio")
    }

    fn capabilities(&self) -> ProviderCapabilities {
        // Capabilities vary by loaded model, these are reasonable defaults
        ProviderCapabilities {
            max_context_tokens: 8192,
            is_local: true, // LM Studio is always local
            supports_vision: false,
            supports_tool_use: false,
            supports_structured_output: false,
            model_name: self.model.clone(),
            cost_tier: 0, // Local is free
        }
    }

    fn provider_id(&self) -> &'static str {
        "lmstudio"
    }
}

// LM Studio API types (OpenAI-compatible)

#[derive(Debug, Serialize)]
struct LmStudioRequest {
    model: String,
    messages: Vec<StandardMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LmStudioResponse {
    model: String,
    choices: Vec<LmStudioChoice>,
    usage: Option<StandardUsage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LmStudioChoice {
    message: StandardMessage,
    finish_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = LmStudioProvider::new().expect("create provider");
        assert_eq!(provider.provider_id(), "lmstudio");
        assert_eq!(provider.base_url, "http://localhost:1234");
    }

    #[test]
    fn test_provider_with_custom_url() {
        let provider = LmStudioProvider::with_url("http://custom:8080").expect("create provider");
        assert_eq!(provider.base_url, "http://custom:8080");
    }

    #[test]
    fn test_capabilities() {
        let provider = LmStudioProvider::new().expect("create provider");
        let caps = provider.capabilities();

        assert_eq!(caps.max_context_tokens, 8192);
        assert!(caps.is_local); // LM Studio is always local
        assert!(!caps.supports_vision);
        assert!(!caps.supports_tool_use);
        assert_eq!(caps.cost_tier, 0); // Local is free
    }

    #[test]
    fn test_api_request_conversion() {
        let provider = LmStudioProvider::new().expect("create provider");
        let request = CompletionRequest::new("Hello")
            .with_max_tokens(1000)
            .with_temperature(0.7)
            .with_system_prompt("You are helpful");

        let api_request = provider.to_api_request(&request);

        assert_eq!(api_request.model, "local-model");
        assert_eq!(api_request.max_tokens, Some(1000));
        assert_eq!(api_request.temperature, Some(0.7));
        assert_eq!(api_request.messages.len(), 2); // System + User
        assert_eq!(api_request.messages[0].role, "system");
        assert_eq!(api_request.messages[0].content, "You are helpful");
        assert_eq!(api_request.messages[1].role, "user");
        assert_eq!(api_request.messages[1].content, "Hello");
    }

    #[test]
    fn test_default() {
        let provider = LmStudioProvider::default();
        assert_eq!(provider.base_url, "http://localhost:1234");
    }

    #[tokio::test]
    async fn test_is_available_method() {
        // Test that is_available() method works without panicking
        // In CI/test environments, LM Studio won't be running, so it should return false
        // But we just verify the method can be called
        let provider = LmStudioProvider::new().expect("create provider");
        let _available = provider.is_available().await.expect("check availability");
        // Method completed successfully - that's what we're testing
    }
}
