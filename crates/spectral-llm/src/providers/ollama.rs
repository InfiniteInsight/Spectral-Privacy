//! Ollama local LLM provider implementation.

use crate::error::{LlmError, Result};
use crate::provider::{
    CompletionRequest, CompletionResponse, CompletionStream, LlmProvider, ProviderCapabilities,
    Role, StreamChunk,
};
use async_trait::async_trait;
use futures::stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Ollama local LLM provider.
///
/// Connects to a local Ollama instance for privacy-preserving LLM operations.
/// This is a stub implementation that provides the interface for Ollama API.
pub struct OllamaProvider {
    model: String,
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider with default settings.
    ///
    /// # Errors
    /// Returns error if the HTTP client cannot be created.
    pub fn new() -> Result<Self> {
        Self::with_model("llama3.1:8b")
    }

    /// Create a new Ollama provider with a specific model.
    ///
    /// # Errors
    /// Returns error if the HTTP client cannot be created.
    pub fn with_model(model: impl Into<String>) -> Result<Self> {
        Self::with_url("http://localhost:11434", model)
    }

    /// Create a new Ollama provider with custom URL and model.
    ///
    /// # Errors
    /// Returns error if the HTTP client cannot be created.
    pub fn with_url(base_url: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| LlmError::Internal(format!("failed to create HTTP client: {e}")))?;

        Ok(Self {
            model: model.into(),
            client,
            base_url: base_url.into(),
        })
    }

    /// Convert internal request to Ollama API format.
    fn to_api_request(&self, request: &CompletionRequest) -> OllamaRequest {
        // Build prompt from messages
        let mut prompt_parts = Vec::new();

        if let Some(system) = &request.system_prompt {
            prompt_parts.push(format!("System: {system}"));
        }

        for message in &request.messages {
            let prefix = match message.role {
                Role::User => "User:",
                Role::Assistant => "Assistant:",
                Role::System => "System:",
            };
            prompt_parts.push(format!("{prefix} {}", message.content));
        }

        // Add final "Assistant:" to prompt continuation
        prompt_parts.push("Assistant:".to_string());

        let prompt = prompt_parts.join("\n\n");

        OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
            options: OllamaOptions {
                temperature: request.temperature,
                num_predict: request
                    .max_tokens
                    .map(i32::try_from)
                    .transpose()
                    .ok()
                    .flatten(),
                stop: if request.stop_sequences.is_empty() {
                    None
                } else {
                    Some(request.stop_sequences.clone())
                },
            },
        }
    }

    /// Convert Ollama API response to internal format.
    #[allow(dead_code)]
    fn convert_api_response(response: OllamaResponse) -> CompletionResponse {
        CompletionResponse {
            content: response.response,
            model: response.model,
            stop_reason: if response.done {
                Some("stop".to_string())
            } else {
                None
            },
            usage: None, // Ollama doesn't provide token counts in simple API
            provider_id: None,
            pii_filtered: None,
        }
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new().expect("create default Ollama provider")
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let api_request = self.to_api_request(&request);

        // Make actual API call to Ollama
        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
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
                provider: "ollama".to_string(),
                status: status.as_u16(),
                message: error_text,
            });
        }

        // Parse the JSON response
        let api_response: OllamaResponse =
            response.json().await.map_err(|e| LlmError::ParseError {
                provider: "ollama".to_string(),
                message: format!("Failed to parse response: {e}"),
            })?;

        // Convert to internal format
        Ok(Self::convert_api_response(api_response))
    }

    async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        // Stub implementation - would setup streaming here
        let content = format!(
            "[Stub] Ollama {} would stream response to: {}",
            self.model,
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
                stop_reason: Some("stop".to_string()),
            }),
        ];

        Ok(Box::pin(stream::iter(chunks)))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        // Capabilities vary by model, these are reasonable defaults for llama3.1:8b
        ProviderCapabilities {
            max_context_tokens: 8192,
            is_local: true, // Ollama is always local
            supports_vision: false,
            supports_tool_use: false,
            supports_structured_output: false,
            model_name: self.model.clone(),
            cost_tier: 0, // Local is free
        }
    }

    fn provider_id(&self) -> &'static str {
        "ollama"
    }
}

// Ollama API types

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Default, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OllamaResponse {
    model: String,
    response: String,
    done: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = OllamaProvider::new().expect("create provider");
        assert_eq!(provider.provider_id(), "ollama");
        assert_eq!(provider.model, "llama3.1:8b");
        assert_eq!(provider.base_url, "http://localhost:11434");
    }

    #[test]
    fn test_provider_with_custom_model() {
        let provider = OllamaProvider::with_model("llama3.1:70b").expect("create provider");
        assert_eq!(provider.model, "llama3.1:70b");
    }

    #[test]
    fn test_provider_with_custom_url() {
        let provider = OllamaProvider::with_url("http://custom:11434", "llama3.1:8b")
            .expect("create provider");
        assert_eq!(provider.base_url, "http://custom:11434");
    }

    #[test]
    fn test_capabilities() {
        let provider = OllamaProvider::new().expect("create provider");
        let caps = provider.capabilities();

        assert_eq!(caps.max_context_tokens, 8192);
        assert!(caps.is_local); // Ollama is always local
        assert!(!caps.supports_vision);
        assert!(!caps.supports_tool_use);
        assert_eq!(caps.cost_tier, 0); // Local is free
    }

    #[test]
    fn test_api_request_conversion() {
        let provider = OllamaProvider::new().expect("create provider");
        let request = CompletionRequest::new("Hello")
            .with_max_tokens(1000)
            .with_temperature(0.7)
            .with_system_prompt("You are helpful");

        let api_request = provider.to_api_request(&request);

        assert_eq!(api_request.model, "llama3.1:8b");
        assert!(!api_request.stream);
        assert!(api_request.prompt.contains("System: You are helpful"));
        assert!(api_request.prompt.contains("User: Hello"));
        assert_eq!(api_request.options.temperature, Some(0.7));
        assert_eq!(api_request.options.num_predict, Some(1000));
    }

    #[test]
    fn test_default() {
        let provider = OllamaProvider::default();
        assert_eq!(provider.model, "llama3.1:8b");
    }
}
