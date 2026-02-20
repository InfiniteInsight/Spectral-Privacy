//! Privacy-aware LLM router that enforces privacy settings.
//!
//! This module routes LLM requests to appropriate providers based on:
//! - Privacy level permissions
//! - Task-specific preferences
//! - Primary provider configuration
//! - Provider capabilities (local vs cloud)

use crate::engine::PrivacyEngine;
use crate::error::Result;
use crate::llm_settings::{get_primary_provider, get_provider_preference, LlmProvider, TaskType};
use crate::types::Feature;
use spectral_llm::{
    AnthropicProvider, CompletionRequest, CompletionResponse, FilterStrategy, GeminiProvider,
    LlmProvider as LlmProviderTrait, LmStudioProvider, OllamaProvider, OpenAiProvider, PiiFilter,
};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;

/// Privacy-aware LLM router.
///
/// Routes LLM requests to appropriate providers while enforcing privacy settings.
/// The router checks permissions before allowing cloud providers and applies PII
/// filtering when necessary.
pub struct PrivacyAwareLlmRouter {
    pool: SqlitePool,
    engine: PrivacyEngine,
}

impl PrivacyAwareLlmRouter {
    /// Create a new privacy-aware LLM router.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        let engine = PrivacyEngine::new(pool.clone());
        Self { pool, engine }
    }

    /// Route a request to the appropriate LLM provider.
    ///
    /// # Errors
    /// Returns error if:
    /// - Privacy settings deny the requested provider
    /// - No suitable provider is available
    /// - Provider initialization fails
    /// - LLM request fails
    pub async fn route(
        &self,
        task_type: TaskType,
        request: CompletionRequest,
    ) -> Result<CompletionResponse> {
        // 1. Determine which provider to use based on preferences
        let provider_type = self.select_provider(task_type).await?;

        // 2. Check privacy permissions
        self.check_permissions(&provider_type).await?;

        // 3. Create provider instance
        let provider = self.create_provider(provider_type)?;

        // 4. Apply PII filtering for cloud providers
        let (filtered_request, token_map) = if provider_type.is_local() {
            // Local providers: no filtering needed
            (request, None)
        } else {
            // Cloud providers: apply tokenization to protect PII
            Self::apply_pii_filtering(request)?
        };

        // 5. Make request
        let response = provider
            .complete(filtered_request)
            .await
            .map_err(|e| crate::error::PrivacyError::LlmRequest(e.to_string()))?;

        // 6. Detokenize PII in response if filtering was applied
        let final_response = if let Some(token_map) = token_map {
            Self::detokenize_response(response, &token_map)
        } else {
            response
        };

        Ok(final_response)
    }

    /// Select the provider to use based on task preferences.
    async fn select_provider(&self, task_type: TaskType) -> Result<LlmProvider> {
        // Check task-specific preference first
        if let Some(provider) = get_provider_preference(&self.pool, task_type).await? {
            return Ok(provider);
        }

        // Fall back to primary provider
        if let Some(provider) = get_primary_provider(&self.pool).await? {
            return Ok(provider);
        }

        // Default to Ollama (local)
        Ok(LlmProvider::Ollama)
    }

    /// Check if the selected provider is allowed under current privacy settings.
    async fn check_permissions(&self, provider: &LlmProvider) -> Result<()> {
        // Local providers are always allowed
        if matches!(provider, LlmProvider::Ollama | LlmProvider::LmStudio) {
            return Ok(());
        }

        // Cloud providers require permission
        let permission = self.engine.check_permission(Feature::CloudLlm).await?;

        if !permission.is_allowed() {
            return Err(crate::error::PrivacyError::PermissionDenied(
                "Permission denied: Cloud LLM usage not allowed under current privacy settings"
                    .to_string(),
            ));
        }

        Ok(())
    }

    /// Create a provider instance.
    #[allow(clippy::unused_self)] // Will use self for API key retrieval in Phase 5
    fn create_provider(&self, provider_type: LlmProvider) -> Result<Arc<dyn LlmProviderTrait>> {
        match provider_type {
            LlmProvider::Ollama => {
                let provider = OllamaProvider::new().map_err(|e| {
                    crate::error::PrivacyError::LlmRequest(format!(
                        "Failed to create Ollama provider: {e}"
                    ))
                })?;
                Ok(Arc::new(provider))
            }
            LlmProvider::LmStudio => {
                let provider = LmStudioProvider::new().map_err(|e| {
                    crate::error::PrivacyError::LlmRequest(format!(
                        "Failed to create LM Studio provider: {e}"
                    ))
                })?;
                Ok(Arc::new(provider))
            }
            LlmProvider::OpenAi => {
                // TODO: Get API key from settings using self.pool
                let api_key = "stub-key".to_string(); // pragma: allowlist secret
                let provider = OpenAiProvider::new(api_key).map_err(|e| {
                    crate::error::PrivacyError::LlmRequest(format!(
                        "Failed to create OpenAI provider: {e}"
                    ))
                })?;
                Ok(Arc::new(provider))
            }
            LlmProvider::Gemini => {
                // TODO: Get API key from settings using self.pool
                let api_key = "stub-key".to_string(); // pragma: allowlist secret
                let provider = GeminiProvider::new(api_key).map_err(|e| {
                    crate::error::PrivacyError::LlmRequest(format!(
                        "Failed to create Gemini provider: {e}"
                    ))
                })?;
                Ok(Arc::new(provider))
            }
            LlmProvider::Claude => {
                // TODO: Get API key from settings using self.pool
                let api_key = "stub-key".to_string(); // pragma: allowlist secret
                let provider = AnthropicProvider::new(api_key).map_err(|e| {
                    crate::error::PrivacyError::LlmRequest(format!(
                        "Failed to create Claude provider: {e}"
                    ))
                })?;
                Ok(Arc::new(provider))
            }
        }
    }

    /// Apply PII filtering to a request for cloud providers.
    ///
    /// Uses tokenization strategy to replace PII with reversible tokens.
    /// Returns the filtered request and the token map for detokenization.
    fn apply_pii_filtering(
        request: CompletionRequest,
    ) -> Result<(CompletionRequest, Option<HashMap<String, String>>)> {
        let filter = PiiFilter::with_strategy(FilterStrategy::Tokenize);

        // Filter each message's content
        let mut filtered_messages = Vec::new();
        let mut combined_token_map = HashMap::new();

        for message in request.messages {
            let filter_result = filter
                .filter(&message.content)
                .map_err(|e| crate::error::PrivacyError::LlmRequest(e.to_string()))?;

            // Merge token maps from all messages
            if let Some(token_map) = filter_result.token_map {
                combined_token_map.extend(token_map);
            }

            filtered_messages.push(spectral_llm::Message {
                role: message.role,
                content: filter_result.filtered_text,
            });
        }

        // Also filter system prompt if present
        let filtered_system_prompt = if let Some(system_prompt) = request.system_prompt {
            let filter_result = filter
                .filter(&system_prompt)
                .map_err(|e| crate::error::PrivacyError::LlmRequest(e.to_string()))?;

            if let Some(token_map) = filter_result.token_map {
                combined_token_map.extend(token_map);
            }

            Some(filter_result.filtered_text)
        } else {
            None
        };

        let filtered_request = CompletionRequest {
            messages: filtered_messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            system_prompt: filtered_system_prompt,
            stop_sequences: request.stop_sequences,
            extra: request.extra,
        };

        let token_map = if combined_token_map.is_empty() {
            None
        } else {
            Some(combined_token_map)
        };

        Ok((filtered_request, token_map))
    }

    /// Detokenize a response by replacing tokens with original PII values.
    fn detokenize_response(
        response: CompletionResponse,
        token_map: &HashMap<String, String>,
    ) -> CompletionResponse {
        let filter = PiiFilter::with_strategy(FilterStrategy::Tokenize);
        let detokenized_content = filter.detokenize(&response.content, token_map);

        CompletionResponse {
            content: detokenized_content,
            ..response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PrivacyLevel;
    use spectral_db::Database;
    use sqlx::SqlitePool;

    async fn create_test_db() -> SqlitePool {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create test database");
        db.run_migrations().await.expect("run migrations");
        db.pool().clone()
    }

    #[tokio::test]
    async fn test_route_with_permission_denied() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool.clone());
        // nosemgrep: no-unwrap-in-production
        engine
            .set_privacy_level(PrivacyLevel::Paranoid)
            .await
            .unwrap();

        // Set a cloud provider preference to force the permission check
        // nosemgrep: no-unwrap-in-production
        crate::llm_settings::set_primary_provider(&pool, LlmProvider::OpenAi)
            .await
            .unwrap();

        let router = PrivacyAwareLlmRouter::new(pool);

        let request = CompletionRequest::new("Draft an email");

        let result = router.route(TaskType::EmailDraft, request).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Permission denied"));
    }
}
