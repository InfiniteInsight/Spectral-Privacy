//! LLM routing with privacy-aware provider selection.

use crate::error::{LlmError, Result};
use crate::pii_filter::{FilterStrategy, PiiFilter};
use crate::provider::{
    CompletionRequest, CompletionResponse, CompletionStream, LlmProvider, ProviderCapabilities,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Router that selects appropriate LLM providers based on routing preferences.
///
/// The router applies PII filtering before sending requests to cloud providers
/// and can fallback between providers based on availability and preferences.
pub struct LlmRouter {
    providers: Vec<Arc<dyn LlmProvider>>,
    pii_filter: PiiFilter,
    preference: RoutingPreference,
}

impl LlmRouter {
    /// Create a new router with the specified routing preference.
    #[must_use]
    pub fn new(preference: RoutingPreference) -> Self {
        Self {
            providers: Vec::new(),
            pii_filter: PiiFilter::with_strategy(FilterStrategy::Tokenize),
            preference,
        }
    }

    /// Add a provider to the router.
    pub fn add_provider(&mut self, provider: Arc<dyn LlmProvider>) {
        self.providers.push(provider);
    }

    /// Set the PII filter strategy.
    pub fn set_filter_strategy(&mut self, strategy: FilterStrategy) {
        self.pii_filter = PiiFilter::with_strategy(strategy);
    }

    /// Get the current routing preference.
    #[must_use]
    pub fn preference(&self) -> &RoutingPreference {
        &self.preference
    }

    /// Set the routing preference.
    pub fn set_preference(&mut self, preference: RoutingPreference) {
        self.preference = preference;
    }

    /// Complete a request by routing to an appropriate provider.
    ///
    /// # Errors
    /// Returns error if no suitable provider is available or if the request fails.
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let provider = self.select_provider(&request)?;

        // Apply PII filtering for cloud providers
        let (filtered_request, token_map) = if provider.capabilities().is_local {
            (request, None)
        } else {
            let filter_result = self.pii_filter.filter(&Self::extract_text(&request))?;

            // Replace the request content with filtered version
            let mut filtered_req = request.clone();
            if let Some(last_message) = filtered_req.messages.last_mut() {
                last_message.content = filter_result.filtered_text;
            }

            (filtered_req, filter_result.token_map)
        };

        // Send request to provider
        let mut response = provider.complete(filtered_request).await?;

        // Detokenize response if needed
        if let Some(token_map) = token_map {
            response.content = self.pii_filter.detokenize(&response.content, &token_map);
        }

        Ok(response)
    }

    /// Stream a completion by routing to an appropriate provider.
    ///
    /// # Errors
    /// Returns error if no suitable provider is available.
    pub async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        let provider = self.select_provider(&request)?;

        // For streaming, we apply PII filtering but don't tokenize (more complex)
        let filtered_request = if provider.capabilities().is_local {
            request
        } else {
            let filter_result = self.pii_filter.filter(&Self::extract_text(&request))?;
            let mut filtered_req = request.clone();
            if let Some(last_message) = filtered_req.messages.last_mut() {
                last_message.content = filter_result.filtered_text;
            }
            filtered_req
        };

        provider.stream(filtered_request).await
    }

    /// Select the best provider for the given request.
    fn select_provider(&self, _request: &CompletionRequest) -> Result<&Arc<dyn LlmProvider>> {
        if self.providers.is_empty() {
            return Err(LlmError::NoProviderAvailable);
        }

        match &self.preference {
            RoutingPreference::LocalOnly => {
                // Find first local provider
                self.providers
                    .iter()
                    .find(|p| p.capabilities().is_local)
                    .ok_or(LlmError::NoProviderAvailable)
            }
            RoutingPreference::PreferLocal {
                cloud_allowed_tasks,
            } => {
                // Try local first
                if let Some(provider) = self.providers.iter().find(|p| p.capabilities().is_local) {
                    return Ok(provider);
                }

                // Fallback to cloud if task is allowed
                if cloud_allowed_tasks.contains(&TaskType::General) {
                    self.providers
                        .iter()
                        .find(|p| !p.capabilities().is_local)
                        .ok_or(LlmError::NoProviderAvailable)
                } else {
                    Err(LlmError::NoProviderAvailable)
                }
            }
            RoutingPreference::BestAvailable => {
                // Select based on capabilities and cost
                self.providers
                    .iter()
                    .max_by_key(|p| {
                        let caps = p.capabilities();
                        // Prefer local, then highest context window, then lowest cost
                        (
                            caps.is_local,
                            caps.max_context_tokens,
                            std::cmp::Reverse(caps.cost_tier),
                        )
                    })
                    .ok_or(LlmError::NoProviderAvailable)
            }
        }
    }

    /// Extract text content from a request for PII filtering.
    fn extract_text(request: &CompletionRequest) -> String {
        let mut parts = Vec::new();

        if let Some(system) = &request.system_prompt {
            parts.push(system.clone());
        }

        for message in &request.messages {
            parts.push(message.content.clone());
        }

        parts.join("\n")
    }

    /// Get list of available providers.
    #[must_use]
    pub fn providers(&self) -> &[Arc<dyn LlmProvider>] {
        &self.providers
    }

    /// Get capabilities of all registered providers.
    #[must_use]
    pub fn all_capabilities(&self) -> Vec<(String, ProviderCapabilities)> {
        self.providers
            .iter()
            .map(|p| (p.provider_id().to_string(), p.capabilities()))
            .collect()
    }
}

/// Routing preference for provider selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RoutingPreference {
    /// Never send data to cloud APIs (local models only)
    LocalOnly,

    /// Prefer local providers, fallback to cloud for specific tasks
    PreferLocal {
        /// Tasks that are allowed to use cloud providers
        cloud_allowed_tasks: Vec<TaskType>,
    },

    /// Use the most capable provider available
    BestAvailable,
}

impl Default for RoutingPreference {
    fn default() -> Self {
        Self::PreferLocal {
            cloud_allowed_tasks: vec![TaskType::General],
        }
    }
}

/// Types of tasks for routing decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// General purpose tasks (non-sensitive)
    General,

    /// Tasks involving PII (profile management)
    PiiSensitive,

    /// Browser automation guidance
    BrowserAutomation,

    /// Email generation
    EmailGeneration,

    /// Natural language queries
    NaturalLanguage,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures::stream;

    // Mock provider for testing
    struct MockProvider {
        id: String,
        is_local: bool,
        max_tokens: usize,
    }

    impl MockProvider {
        fn new(id: &str, is_local: bool) -> Self {
            Self {
                id: id.to_string(),
                is_local,
                max_tokens: 4096,
            }
        }
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
            Ok(CompletionResponse {
                content: format!("Response from {}", self.id),
                model: self.id.clone(),
                stop_reason: Some("end_turn".to_string()),
                usage: None,
            })
        }

        async fn stream(&self, _request: CompletionRequest) -> Result<CompletionStream> {
            Ok(Box::pin(stream::empty()))
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                max_context_tokens: self.max_tokens,
                is_local: self.is_local,
                supports_vision: false,
                supports_tool_use: false,
                supports_structured_output: false,
                model_name: self.id.clone(),
                cost_tier: u8::from(!self.is_local),
            }
        }

        fn provider_id(&self) -> &str {
            &self.id
        }
    }

    #[tokio::test]
    async fn test_local_only_routing() {
        let mut router = LlmRouter::new(RoutingPreference::LocalOnly);
        router.add_provider(Arc::new(MockProvider::new("ollama", true)));
        router.add_provider(Arc::new(MockProvider::new("anthropic", false)));

        let request = CompletionRequest::new("Hello");
        let response = router.complete(request).await.expect("complete request");

        assert!(response.model.contains("ollama"));
    }

    #[tokio::test]
    async fn test_prefer_local_routing() {
        let mut router = LlmRouter::new(RoutingPreference::PreferLocal {
            cloud_allowed_tasks: vec![TaskType::General],
        });
        router.add_provider(Arc::new(MockProvider::new("anthropic", false)));
        router.add_provider(Arc::new(MockProvider::new("ollama", true)));

        let request = CompletionRequest::new("Hello");
        let response = router.complete(request).await.expect("complete request");

        // Should prefer local even though cloud was added first
        assert!(response.model.contains("ollama"));
    }

    #[tokio::test]
    async fn test_best_available_routing() {
        let mut router = LlmRouter::new(RoutingPreference::BestAvailable);
        router.add_provider(Arc::new(MockProvider::new("anthropic", false)));
        router.add_provider(Arc::new(MockProvider::new("ollama", true)));

        let request = CompletionRequest::new("Hello");
        let response = router.complete(request).await.expect("complete request");

        // Should prefer local (is_local is highest priority in BestAvailable)
        assert!(response.model.contains("ollama"));
    }

    #[tokio::test]
    async fn test_no_provider_available() {
        let router = LlmRouter::new(RoutingPreference::LocalOnly);
        let request = CompletionRequest::new("Hello");
        let result = router.complete(request).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(LlmError::NoProviderAvailable)));
    }

    #[test]
    fn test_all_capabilities() {
        let mut router = LlmRouter::new(RoutingPreference::BestAvailable);
        router.add_provider(Arc::new(MockProvider::new("ollama", true)));
        router.add_provider(Arc::new(MockProvider::new("anthropic", false)));

        let caps = router.all_capabilities();
        assert_eq!(caps.len(), 2);

        let local_caps = caps.iter().find(|(id, _)| id == "ollama");
        assert!(local_caps.is_some());
        let (_, caps) = local_caps.expect("ollama capabilities");
        assert!(caps.is_local);
    }

    #[test]
    fn test_routing_preference_default() {
        let pref = RoutingPreference::default();
        match pref {
            RoutingPreference::PreferLocal {
                cloud_allowed_tasks,
            } => {
                assert!(cloud_allowed_tasks.contains(&TaskType::General));
            }
            _ => panic!("expected PreferLocal as default"),
        }
    }
}
