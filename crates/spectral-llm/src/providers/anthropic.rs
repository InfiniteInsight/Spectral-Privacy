//! Anthropic Claude provider implementation.

use crate::error::{LlmError, Result};
use crate::provider::{
    CompletionRequest, CompletionResponse, CompletionStream, LlmProvider, ProviderCapabilities,
    Role, StreamChunk, Usage,
};
use async_trait::async_trait;
use futures::stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Anthropic Claude API provider.
///
/// This is a stub implementation that provides the interface for Claude API.
/// Actual API calls can be implemented when API keys are available.
pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: Client,
    base_url: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider with the given API key.
    ///
    /// # Errors
    /// Returns error if the HTTP client cannot be created.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::with_model(api_key, "claude-3-5-sonnet-20241022")
    }

    /// Create a new Anthropic provider with a specific model.
    ///
    /// # Errors
    /// Returns error if the HTTP client cannot be created.
    pub fn with_model(api_key: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| LlmError::Internal(format!("failed to create HTTP client: {e}")))?;

        Ok(Self {
            api_key: api_key.into(),
            model: model.into(),
            client,
            base_url: "https://api.anthropic.com/v1".to_string(),
        })
    }

    /// Convert internal request to Anthropic API format.
    fn to_api_request(&self, request: &CompletionRequest) -> AnthropicRequest {
        let messages: Vec<AnthropicMessage> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System) // System messages handled separately
            .map(|m| AnthropicMessage {
                role: match m.role {
                    Role::Assistant => "assistant".to_string(),
                    Role::User | Role::System => "user".to_string(), // System falls back to user
                },
                content: m.content.clone(),
            })
            .collect();

        AnthropicRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens.unwrap_or(4096),
            temperature: request.temperature,
            system: request.system_prompt.clone(),
            stop_sequences: if request.stop_sequences.is_empty() {
                None
            } else {
                Some(request.stop_sequences.clone())
            },
        }
    }

    /// Convert Anthropic API response to internal format.
    #[allow(dead_code)]
    fn convert_api_response(response: AnthropicResponse) -> CompletionResponse {
        let content = response
            .content
            .into_iter()
            .map(|c| match c {
                ContentBlock::Text { text } => text,
            })
            .collect::<Vec<_>>()
            .join("\n");

        CompletionResponse {
            content,
            model: response.model,
            stop_reason: response.stop_reason,
            usage: response.usage.map(|u| Usage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
            }),
            provider_id: None,
            pii_filtered: None,
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let api_request = self.to_api_request(&request);

        // Make actual API call to Anthropic
        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
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
                provider: "anthropic".to_string(),
                status: status.as_u16(),
                message: error_text,
            });
        }

        // Parse the JSON response
        let api_response: AnthropicResponse =
            response.json().await.map_err(|e| LlmError::ParseError {
                provider: "anthropic".to_string(),
                message: format!("Failed to parse response: {e}"),
            })?;

        // Convert to internal format
        Ok(Self::convert_api_response(api_response))
    }

    async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        // Stub implementation - would setup SSE stream here
        let content = format!(
            "[Stub] Anthropic Claude would stream response to: {}",
            request.messages.last().map_or("", |m| &m.content)
        );

        // Mock stream with single chunk
        let chunks = vec![
            Ok(StreamChunk {
                delta: content,
                is_final: false,
                stop_reason: None,
            }),
            Ok(StreamChunk {
                delta: String::new(),
                is_final: true,
                stop_reason: Some("end_turn".to_string()),
            }),
        ];

        Ok(Box::pin(stream::iter(chunks)))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_context_tokens: 200_000, // Claude 3.5 Sonnet context window
            is_local: false,
            supports_vision: true,
            supports_tool_use: true,
            supports_structured_output: true,
            model_name: self.model.clone(),
            cost_tier: 2, // Cloud API with cost
        }
    }

    fn provider_id(&self) -> &'static str {
        "anthropic"
    }
}

// Anthropic API types

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = AnthropicProvider::new("test-key").expect("create provider");
        assert_eq!(provider.provider_id(), "anthropic");
        assert_eq!(provider.model, "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_provider_with_custom_model() {
        let provider = AnthropicProvider::with_model("test-key", "claude-3-opus-20240229")
            .expect("create provider");
        assert_eq!(provider.model, "claude-3-opus-20240229");
    }

    #[test]
    fn test_capabilities() {
        let provider = AnthropicProvider::new("test-key").expect("create provider");
        let caps = provider.capabilities();

        assert_eq!(caps.max_context_tokens, 200_000);
        assert!(!caps.is_local);
        assert!(caps.supports_vision);
        assert!(caps.supports_tool_use);
        assert!(caps.supports_structured_output);
        assert_eq!(caps.cost_tier, 2);
    }

    #[test]
    fn test_api_request_conversion() {
        let provider = AnthropicProvider::new("test-key").expect("create provider");
        let request = CompletionRequest::new("Hello")
            .with_max_tokens(1000)
            .with_temperature(0.7)
            .with_system_prompt("You are helpful");

        let api_request = provider.to_api_request(&request);

        assert_eq!(api_request.model, "claude-3-5-sonnet-20241022");
        assert_eq!(api_request.max_tokens, 1000);
        assert_eq!(api_request.temperature, Some(0.7));
        assert_eq!(api_request.system, Some("You are helpful".to_string()));
        assert_eq!(api_request.messages.len(), 1);
        assert_eq!(api_request.messages[0].content, "Hello");
    }
}
