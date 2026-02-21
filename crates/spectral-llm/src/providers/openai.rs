//! `OpenAI` API provider implementation.

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

/// `OpenAI` API provider.
///
/// Supports GPT models via `OpenAI`'s chat completions API.
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    client: Client,
    base_url: String,
}

impl OpenAiProvider {
    /// Create a new `OpenAI` provider with the given API key.
    ///
    /// # Errors
    /// Returns error if the HTTP client cannot be created.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::with_model(api_key, "gpt-4o")
    }

    /// Create a new `OpenAI` provider with a specific model.
    ///
    /// # Errors
    /// Returns error if the HTTP client cannot be created.
    pub fn with_model(api_key: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        Ok(Self {
            api_key: api_key.into(),
            model: model.into(),
            client: build_http_client(Some(60))?,
            base_url: "https://api.openai.com/v1".to_string(),
        })
    }

    /// Convert internal request to `OpenAI` API format.
    fn to_api_request(&self, request: &CompletionRequest) -> OpenAiRequest {
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

        OpenAiRequest {
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

    /// Convert `OpenAI` API response to internal format.
    #[allow(dead_code)]
    fn convert_api_response(response: OpenAiResponse) -> Result<CompletionResponse> {
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::ParseError {
                provider: "openai".to_string(),
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
            provider_id: None,
            pii_filtered: None,
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let api_request = self.to_api_request(&request);

        // Make actual API call to OpenAI
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
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
                provider: "openai".to_string(),
                status: status.as_u16(),
                message: error_text,
            });
        }

        // Parse the JSON response
        let api_response: OpenAiResponse =
            response.json().await.map_err(|e| LlmError::ParseError {
                provider: "openai".to_string(),
                message: format!("Failed to parse response: {e}"),
            })?;

        // Convert to internal format
        Self::convert_api_response(api_response)
    }

    async fn stream(&self, _request: CompletionRequest) -> Result<CompletionStream> {
        streaming_not_implemented("OpenAI")
    }

    fn capabilities(&self) -> ProviderCapabilities {
        // Default capabilities for GPT-4o
        ProviderCapabilities {
            max_context_tokens: 128_000, // GPT-4o context window
            is_local: false,
            supports_vision: true,
            supports_tool_use: true,
            supports_structured_output: true,
            model_name: self.model.clone(),
            cost_tier: 3, // Cloud API with moderate cost
        }
    }

    fn provider_id(&self) -> &'static str {
        "openai"
    }
}

// OpenAI API types

#[derive(Debug, Serialize)]
struct OpenAiRequest {
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
struct OpenAiResponse {
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: Option<StandardUsage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAiChoice {
    message: StandardMessage,
    finish_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = OpenAiProvider::new("test-key").expect("create provider");
        assert_eq!(provider.provider_id(), "openai");
        assert_eq!(provider.model, "gpt-4o");
    }

    #[test]
    fn test_provider_with_custom_model() {
        let provider =
            OpenAiProvider::with_model("test-key", "gpt-4-turbo").expect("create provider");
        assert_eq!(provider.model, "gpt-4-turbo");
    }

    #[test]
    fn test_capabilities() {
        let provider = OpenAiProvider::new("test-key").expect("create provider");
        let caps = provider.capabilities();

        assert_eq!(caps.max_context_tokens, 128_000);
        assert!(!caps.is_local);
        assert!(caps.supports_vision);
        assert!(caps.supports_tool_use);
        assert!(caps.supports_structured_output);
        assert_eq!(caps.cost_tier, 3);
    }

    #[test]
    fn test_api_request_conversion() {
        let provider = OpenAiProvider::new("test-key").expect("create provider");
        let request = CompletionRequest::new("Hello")
            .with_max_tokens(1000)
            .with_temperature(0.7)
            .with_system_prompt("You are helpful");

        let api_request = provider.to_api_request(&request);

        assert_eq!(api_request.model, "gpt-4o");
        assert_eq!(api_request.max_tokens, Some(1000));
        assert_eq!(api_request.temperature, Some(0.7));
        assert_eq!(api_request.messages.len(), 2); // System + User
        assert_eq!(api_request.messages[0].role, "system");
        assert_eq!(api_request.messages[0].content, "You are helpful");
        assert_eq!(api_request.messages[1].role, "user");
        assert_eq!(api_request.messages[1].content, "Hello");
    }
}
