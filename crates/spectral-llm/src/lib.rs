//! Spectral LLM - LLM abstraction layer with privacy-aware routing.
//!
//! This crate provides a unified interface for working with multiple LLM backends
//! while maintaining privacy through PII filtering and local-preferred routing.
//!
//! # Features
//!
//! - **Provider Abstraction**: Unified trait for multiple LLM backends
//! - **Privacy-Aware Routing**: Route requests based on data sensitivity
//! - **PII Filtering**: Detect and sanitize personally identifiable information
//! - **Multiple Strategies**: Redact, tokenize, or block PII in requests
//! - **Local-First**: Prefer local models for sensitive data
//!
//! # Example
//!
//! ```rust
//! use spectral_llm::{
//!     LlmRouter, RoutingPreference, OllamaProvider, CompletionRequest,
//! };
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a router that prefers local providers
//! let mut router = LlmRouter::new(RoutingPreference::LocalOnly);
//!
//! // Add a local provider
//! let ollama = OllamaProvider::new()?;
//! router.add_provider(Arc::new(ollama));
//!
//! // Make a request
//! let request = CompletionRequest::new("How do I opt out of Spokeo?");
//! let response = router.complete(request).await?;
//!
//! println!("Response: {}", response.content);
//! # Ok(())
//! # }
//! ```
//!
//! # Privacy Model
//!
//! The LLM router automatically applies PII filtering for cloud providers:
//!
//! ```text
//! User Query → PII Filter (tokenize/redact) → Route to Provider
//!                                                     ↓
//! User Response ← PII Detokenize (if tokenized) ← Raw Response
//! ```
//!
//! When using `RoutingPreference::LocalOnly`, no data leaves the machine.
//! When using `RoutingPreference::PreferLocal`, the router tries local providers
//! first and only falls back to cloud if needed and allowed.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod error;
pub mod pii_filter;
pub mod provider;
pub mod providers;
pub mod router;

// Re-export commonly used types
pub use error::{LlmError, Result};
pub use pii_filter::{FilterResult, FilterStrategy, PiiFilter, PiiType};
pub use provider::{
    CompletionRequest, CompletionResponse, CompletionStream, LlmProvider, Message,
    ProviderCapabilities, Role, StreamChunk, Usage,
};
pub use providers::{AnthropicProvider, OllamaProvider};
pub use router::{LlmRouter, RoutingPreference, TaskType};
