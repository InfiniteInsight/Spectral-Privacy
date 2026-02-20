//! Google Gemini API provider implementation.

use crate::error::{LlmError, Result};
use crate::provider::{
    CompletionRequest, CompletionResponse, CompletionStream, LlmProvider, ProviderCapabilities,
    Role, Usage,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Google Gemini API provider.
///
/// Supports Gemini models via Google's `generateContent` API.
/// Note: Gemini uses "user"/"model" roles instead of "user"/"assistant".
pub struct GeminiProvider {
    api_key: String,
    model: String,
    client: Client,
    base_url: String,
}

impl GeminiProvider {
    /// Create a new Gemini provider with the given API key.
    ///
    /// # Errors
    /// Returns error if the HTTP client cannot be created.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::with_model(api_key, "gemini-2.0-flash-exp")
    }

    /// Create a new Gemini provider with a specific model.
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
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        })
    }

    /// Convert internal request to Gemini API format.
    #[allow(clippy::unused_self)]
    fn to_api_request(&self, request: &CompletionRequest) -> GeminiRequest {
        let mut contents: Vec<GeminiContent> = Vec::new();

        // Build system instruction if present
        let system_instruction = request.system_prompt.as_ref().map(|prompt| GeminiContent {
            role: "user".to_string(),
            parts: vec![GeminiPart {
                text: prompt.clone(),
            }],
        });

        // Convert messages to Gemini format
        // Gemini uses "user" and "model" roles (not "assistant")
        for message in &request.messages {
            let role = match message.role {
                Role::User | Role::System => "user".to_string(),
                Role::Assistant => "model".to_string(),
            };

            contents.push(GeminiContent {
                role,
                parts: vec![GeminiPart {
                    text: message.content.clone(),
                }],
            });
        }

        GeminiRequest {
            contents,
            system_instruction,
            generation_config: Some(GeminiGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens.and_then(|t| i32::try_from(t).ok()),
                stop_sequences: if request.stop_sequences.is_empty() {
                    None
                } else {
                    Some(request.stop_sequences.clone())
                },
            }),
        }
    }

    /// Convert Gemini API response to internal format.
    #[allow(dead_code)]
    fn convert_api_response(response: GeminiResponse) -> Result<CompletionResponse> {
        let candidate =
            response
                .candidates
                .into_iter()
                .next()
                .ok_or_else(|| LlmError::ParseError {
                    provider: "gemini".to_string(),
                    message: "no candidates in response".to_string(),
                })?;

        let text = candidate
            .content
            .parts
            .into_iter()
            .map(|p| p.text)
            .collect::<Vec<_>>()
            .join("\n");

        Ok(CompletionResponse {
            content: text,
            model: response
                .model_version
                .unwrap_or_else(|| "unknown".to_string()),
            stop_reason: candidate.finish_reason,
            usage: response.usage_metadata.map(|u| Usage {
                input_tokens: u.prompt_token_count,
                output_tokens: u.candidates_token_count,
            }),
        })
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let api_request = self.to_api_request(&request);

        // Stub implementation - would make actual API call here
        let _response = self
            .client
            .post(format!(
                "{}/models/{}:generateContent?key={}",
                self.base_url, self.model, self.api_key
            ))
            .header("Content-Type", "application/json")
            .json(&api_request);

        // Mock response for stub
        Ok(CompletionResponse {
            content: format!(
                "[Stub] Gemini {} would respond to: {}",
                self.model,
                request.messages.last().map_or("", |m| &m.content)
            ),
            model: self.model.clone(),
            stop_reason: Some("STOP".to_string()),
            usage: Some(Usage {
                input_tokens: 50,
                output_tokens: 100,
            }),
        })
    }

    async fn stream(&self, _request: CompletionRequest) -> Result<CompletionStream> {
        Err(LlmError::Internal(
            "streaming not yet implemented for Gemini".to_string(),
        ))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        // Default capabilities for Gemini 2.0 Flash
        ProviderCapabilities {
            max_context_tokens: 1_048_576, // Gemini 2.0 Flash context window
            is_local: false,
            supports_vision: true,
            supports_tool_use: true,
            supports_structured_output: true,
            model_name: self.model.clone(),
            cost_tier: 2, // Cloud API with cost
        }
    }

    fn provider_id(&self) -> &'static str {
        "gemini"
    }
}

// Gemini API types

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "modelVersion")]
    model_version: Option<String>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GeminiCandidate {
    content: GeminiContent,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = GeminiProvider::new("test-key").expect("create provider");
        assert_eq!(provider.provider_id(), "gemini");
        assert_eq!(provider.model, "gemini-2.0-flash-exp");
    }

    #[test]
    fn test_provider_with_custom_model() {
        let provider =
            GeminiProvider::with_model("test-key", "gemini-1.5-pro").expect("create provider");
        assert_eq!(provider.model, "gemini-1.5-pro");
    }

    #[test]
    fn test_capabilities() {
        let provider = GeminiProvider::new("test-key").expect("create provider");
        let caps = provider.capabilities();

        assert_eq!(caps.max_context_tokens, 1_048_576);
        assert!(!caps.is_local);
        assert!(caps.supports_vision);
        assert!(caps.supports_tool_use);
        assert!(caps.supports_structured_output);
        assert_eq!(caps.cost_tier, 2);
    }

    #[tokio::test]
    async fn test_complete_stub() {
        let provider = GeminiProvider::new("test-key").expect("create provider");
        let request = CompletionRequest::new("Hello");

        let response = provider.complete(request).await.expect("complete request");

        assert!(response.content.contains("Stub"));
        assert!(response.content.contains("gemini-2.0-flash-exp"));
        assert_eq!(response.model, "gemini-2.0-flash-exp");
        assert!(response.usage.is_some());
    }

    #[tokio::test]
    async fn test_stream_not_implemented() {
        let provider = GeminiProvider::new("test-key").expect("create provider");
        let request = CompletionRequest::new("Hello");

        let result = provider.stream(request).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not yet implemented"));
        }
    }

    #[test]
    fn test_api_request_conversion() {
        let provider = GeminiProvider::new("test-key").expect("create provider");
        let request = CompletionRequest::new("Hello")
            .with_max_tokens(1000)
            .with_temperature(0.7)
            .with_system_prompt("You are helpful");

        let api_request = provider.to_api_request(&request);

        // System instruction should be separate
        assert!(api_request.system_instruction.is_some());
        if let Some(system_instr) = api_request.system_instruction {
            assert_eq!(system_instr.parts[0].text, "You are helpful");
        }

        // User message should be in contents
        assert_eq!(api_request.contents.len(), 1);
        assert_eq!(api_request.contents[0].role, "user");
        assert_eq!(api_request.contents[0].parts[0].text, "Hello");

        // Generation config
        assert!(api_request.generation_config.is_some());
        if let Some(config) = api_request.generation_config {
            assert_eq!(config.temperature, Some(0.7));
            assert_eq!(config.max_output_tokens, Some(1000));
        }
    }

    #[test]
    fn test_role_conversion() {
        let provider = GeminiProvider::new("test-key").expect("create provider");
        let request = CompletionRequest {
            messages: vec![
                crate::provider::Message::user("User message"),
                crate::provider::Message::assistant("Assistant message"),
            ],
            max_tokens: None,
            temperature: None,
            system_prompt: None,
            stop_sequences: Vec::new(),
            extra: serde_json::Value::Null,
        };

        let api_request = provider.to_api_request(&request);

        assert_eq!(api_request.contents.len(), 2);
        assert_eq!(api_request.contents[0].role, "user");
        assert_eq!(api_request.contents[1].role, "model"); // Assistant -> model
    }
}
