//! Core LLM provider trait and request/response types.

use crate::error::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

/// Trait for LLM providers supporting completion and streaming.
///
/// All LLM backends must implement this trait. Provider implementations
/// should be thread-safe (Send + Sync) for use in async contexts.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Complete a prompt with a single response.
    ///
    /// # Errors
    /// Returns error if the provider fails, network issues occur, or response parsing fails.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Stream a completion response token by token.
    ///
    /// # Errors
    /// Returns error if the provider fails or network issues occur.
    async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream>;

    /// Get the capabilities of this provider.
    fn capabilities(&self) -> ProviderCapabilities;

    /// Get the unique identifier for this provider.
    fn provider_id(&self) -> &str;
}

/// Capabilities of an LLM provider.
///
/// Used by the router to determine which provider is suitable for a given request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct ProviderCapabilities {
    /// Maximum context window size in tokens
    pub max_context_tokens: usize,

    /// Whether this is a local provider (no data leaves the machine)
    pub is_local: bool,

    /// Supports vision/image inputs
    pub supports_vision: bool,

    /// Supports tool/function calling
    pub supports_tool_use: bool,

    /// Supports structured output (JSON mode, etc.)
    pub supports_structured_output: bool,

    /// Model name or identifier
    pub model_name: String,

    /// Relative cost per token (0 = free/local, higher = more expensive)
    pub cost_tier: u8,
}

/// Request for LLM completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// The prompt or conversation messages
    pub messages: Vec<Message>,

    /// Maximum tokens to generate (optional)
    pub max_tokens: Option<u32>,

    /// Temperature for sampling (0.0 = deterministic, 1.0 = creative)
    pub temperature: Option<f32>,

    /// System prompt (optional)
    pub system_prompt: Option<String>,

    /// Stop sequences (optional)
    pub stop_sequences: Vec<String>,

    /// Additional provider-specific options
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

impl CompletionRequest {
    /// Create a new completion request with a simple user message.
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            messages: vec![Message::user(content)],
            max_tokens: None,
            temperature: None,
            system_prompt: None,
            stop_sequences: Vec::new(),
            extra: serde_json::Value::Null,
        }
    }

    /// Set the maximum tokens to generate.
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the temperature.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the system prompt.
    #[must_use]
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Add a stop sequence.
    #[must_use]
    pub fn with_stop_sequence(mut self, stop: impl Into<String>) -> Self {
        self.stop_sequences.push(stop.into());
        self
    }
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: Role,

    /// Content of the message
    pub content: String,
}

impl Message {
    /// Create a user message.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    /// Create an assistant message.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }

    /// Create a system message.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
}

/// Role of a message in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message (instructions, context)
    System,
    /// User message (prompt, question)
    User,
    /// Assistant message (response)
    Assistant,
}

/// Response from LLM completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// The generated text
    pub content: String,

    /// Model that generated the response
    pub model: String,

    /// Stop reason (e.g., "`end_turn`", "`max_tokens`")
    pub stop_reason: Option<String>,

    /// Usage statistics
    pub usage: Option<Usage>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens consumed
    pub input_tokens: u32,

    /// Output tokens generated
    pub output_tokens: u32,
}

impl Usage {
    /// Get total tokens used.
    #[must_use]
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Streaming completion response.
pub type CompletionStream = BoxStream<'static, Result<StreamChunk>>;

/// A chunk in a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Text delta (incremental content)
    pub delta: String,

    /// Whether this is the final chunk
    pub is_final: bool,

    /// Stop reason (only present in final chunk)
    pub stop_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_request_builder() {
        let req = CompletionRequest::new("Hello")
            .with_max_tokens(100)
            .with_temperature(0.7)
            .with_system_prompt("You are a helpful assistant");

        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].content, "Hello");
        assert_eq!(req.max_tokens, Some(100));
        assert_eq!(req.temperature, Some(0.7));
        assert_eq!(
            req.system_prompt,
            Some("You are a helpful assistant".to_string())
        );
    }

    #[test]
    fn test_message_constructors() {
        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, Role::User);
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = Message::assistant("Hi there");
        assert_eq!(assistant_msg.role, Role::Assistant);

        let system_msg = Message::system("You are helpful");
        assert_eq!(system_msg.role, Role::System);
    }

    #[test]
    fn test_usage_total_tokens() {
        let usage = Usage {
            input_tokens: 10,
            output_tokens: 20,
        };
        assert_eq!(usage.total_tokens(), 30);
    }

    #[test]
    fn test_provider_capabilities_serialization() {
        let caps = ProviderCapabilities {
            max_context_tokens: 8192,
            is_local: true,
            supports_vision: false,
            supports_tool_use: false,
            supports_structured_output: true,
            model_name: "llama3.1:8b".to_string(),
            cost_tier: 0,
        };

        let json = serde_json::to_string(&caps).expect("serialize capabilities");
        let deserialized: ProviderCapabilities =
            serde_json::from_str(&json).expect("deserialize capabilities");

        assert_eq!(deserialized.max_context_tokens, 8192);
        assert!(deserialized.is_local);
        assert_eq!(deserialized.cost_tier, 0);
    }
}
