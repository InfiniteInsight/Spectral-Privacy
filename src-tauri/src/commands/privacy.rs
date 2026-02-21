//! Privacy settings commands.

use crate::error::CommandError;
use crate::state::AppState;
use serde::Serialize;
use spectral_llm::{
    AnthropicProvider, CompletionRequest, GeminiProvider, LmStudioProvider, OllamaProvider,
    OpenAiProvider,
};
use spectral_privacy::{FeatureFlags, LlmProvider, PrivacyEngine, PrivacyLevel, TaskType};
use sqlx::SqlitePool;
use tauri::State;
use tracing::info;

/// Helper function to get database pool for a vault.
///
/// # Errors
/// Returns `CommandError` if vault is locked or database access fails.
fn get_vault_pool(state: &AppState, vault_id: &str) -> Result<SqlitePool, CommandError> {
    let vault = state
        .get_vault(vault_id)
        .ok_or_else(|| CommandError::new("VAULT_LOCKED", "Vault is locked"))?;

    let pool = vault
        .database()
        .map_err(|e| {
            CommandError::new(
                "VAULT_ERROR",
                format!("Failed to access vault database: {e}"),
            )
        })?
        .pool()
        .clone();

    Ok(pool)
}

/// Privacy settings response
#[derive(Debug, Serialize)]
pub struct PrivacySettings {
    pub privacy_level: PrivacyLevel,
    pub feature_flags: FeatureFlags,
}

/// LLM provider settings response
#[derive(Debug, Serialize)]
pub struct LlmProviderSettings {
    pub primary_provider: Option<LlmProvider>,
    pub email_draft_provider: Option<LlmProvider>,
    pub form_fill_provider: Option<LlmProvider>,
    pub has_openai_key: bool,
    pub has_gemini_key: bool,
    pub has_claude_key: bool,
}

/// Get current privacy settings.
///
/// Returns the current privacy level and feature flags.
#[tauri::command]
pub async fn get_privacy_settings(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<PrivacySettings, CommandError> {
    info!("Getting privacy settings for vault: {}", vault_id);

    let pool = get_vault_pool(&state, &vault_id)?;
    let engine = PrivacyEngine::new(pool);

    // Get settings
    let privacy_level = engine.get_privacy_level().await.map_err(|e| {
        CommandError::new(
            "PRIVACY_ERROR",
            format!("Failed to get privacy level: {}", e),
        )
    })?;

    let feature_flags = engine.get_feature_flags().await.map_err(|e| {
        CommandError::new(
            "PRIVACY_ERROR",
            format!("Failed to get feature flags: {}", e),
        )
    })?;

    Ok(PrivacySettings {
        privacy_level,
        feature_flags,
    })
}

/// Set privacy level.
///
/// Updates the privacy level and automatically updates feature flags if not Custom.
#[tauri::command]
pub async fn set_privacy_level(
    state: State<'_, AppState>,
    vault_id: String,
    level: PrivacyLevel,
) -> Result<(), CommandError> {
    info!(
        "Setting privacy level to {:?} for vault: {}",
        level, vault_id
    );

    let pool = get_vault_pool(&state, &vault_id)?;
    let engine = PrivacyEngine::new(pool);

    // Set privacy level
    engine.set_privacy_level(level).await.map_err(|e| {
        CommandError::new(
            "PRIVACY_ERROR",
            format!("Failed to set privacy level: {}", e),
        )
    })?;

    // If not Custom, also set the feature flags to match the level
    if level != PrivacyLevel::Custom {
        let flags = level.to_feature_flags();
        engine.set_feature_flags(flags).await.map_err(|e| {
            CommandError::new(
                "PRIVACY_ERROR",
                format!("Failed to set feature flags: {}", e),
            )
        })?;
    }

    Ok(())
}

/// Set custom feature flags.
///
/// Only takes effect when privacy level is set to Custom.
#[tauri::command]
pub async fn set_custom_feature_flags(
    state: State<'_, AppState>,
    vault_id: String,
    flags: FeatureFlags,
) -> Result<(), CommandError> {
    info!("Setting custom feature flags for vault: {}", vault_id);

    let pool = get_vault_pool(&state, &vault_id)?;
    let engine = PrivacyEngine::new(pool);

    // Set feature flags
    engine.set_feature_flags(flags).await.map_err(|e| {
        CommandError::new(
            "PRIVACY_ERROR",
            format!("Failed to set feature flags: {}", e),
        )
    })?;

    Ok(())
}

/// Get LLM provider settings.
///
/// Returns the primary provider, task-specific providers, and which providers have API keys configured.
#[tauri::command]
pub async fn get_llm_provider_settings(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<LlmProviderSettings, CommandError> {
    info!("Getting LLM provider settings for vault: {}", vault_id);

    let pool = get_vault_pool(&state, &vault_id)?;

    // Get primary provider
    let primary_provider = spectral_privacy::get_primary_provider(&pool)
        .await
        .map_err(|e| {
            CommandError::new(
                "PRIVACY_ERROR",
                format!("Failed to get primary provider: {}", e),
            )
        })?;

    // Get task-specific providers
    let email_draft_provider =
        spectral_privacy::get_provider_preference(&pool, TaskType::EmailDraft)
            .await
            .map_err(|e| {
                CommandError::new(
                    "PRIVACY_ERROR",
                    format!("Failed to get email draft provider: {}", e),
                )
            })?;

    let form_fill_provider = spectral_privacy::get_provider_preference(&pool, TaskType::FormFill)
        .await
        .map_err(|e| {
            CommandError::new(
                "PRIVACY_ERROR",
                format!("Failed to get form fill provider: {}", e),
            )
        })?;

    // Check which API keys are configured
    let has_openai_key = spectral_privacy::get_api_key(&pool, LlmProvider::OpenAi)
        .await
        .map_err(|e| {
            CommandError::new(
                "PRIVACY_ERROR",
                format!("Failed to check OpenAI key: {}", e),
            )
        })?
        .is_some();

    let has_gemini_key = spectral_privacy::get_api_key(&pool, LlmProvider::Gemini)
        .await
        .map_err(|e| {
            CommandError::new(
                "PRIVACY_ERROR",
                format!("Failed to check Gemini key: {}", e),
            )
        })?
        .is_some();

    let has_claude_key = spectral_privacy::get_api_key(&pool, LlmProvider::Claude)
        .await
        .map_err(|e| {
            CommandError::new(
                "PRIVACY_ERROR",
                format!("Failed to check Claude key: {}", e),
            )
        })?
        .is_some();

    Ok(LlmProviderSettings {
        primary_provider,
        email_draft_provider,
        form_fill_provider,
        has_openai_key,
        has_gemini_key,
        has_claude_key,
    })
}

/// Set primary LLM provider.
///
/// Sets the default provider to use when no task-specific preference is configured.
#[tauri::command]
pub async fn set_llm_primary_provider(
    state: State<'_, AppState>,
    vault_id: String,
    provider: LlmProvider,
) -> Result<(), CommandError> {
    info!(
        "Setting primary LLM provider to {:?} for vault: {}",
        provider, vault_id
    );

    let pool = get_vault_pool(&state, &vault_id)?;

    // Set primary provider
    spectral_privacy::set_primary_provider(&pool, provider)
        .await
        .map_err(|e| {
            CommandError::new(
                "PRIVACY_ERROR",
                format!("Failed to set primary provider: {}", e),
            )
        })?;

    Ok(())
}

/// Set task-specific LLM provider.
///
/// Sets the provider to use for a specific task type (email drafts, form filling, etc.).
#[tauri::command]
pub async fn set_llm_task_provider(
    state: State<'_, AppState>,
    vault_id: String,
    task_type: TaskType,
    provider: LlmProvider,
) -> Result<(), CommandError> {
    info!(
        "Setting {:?} provider to {:?} for vault: {}",
        task_type, provider, vault_id
    );

    let pool = get_vault_pool(&state, &vault_id)?;

    // Set task-specific provider
    spectral_privacy::set_provider_preference(&pool, task_type, provider)
        .await
        .map_err(|e| {
            CommandError::new(
                "PRIVACY_ERROR",
                format!("Failed to set task provider: {}", e),
            )
        })?;

    Ok(())
}

/// Set API key for an LLM provider.
///
/// Stores the API key encrypted in the vault database.
#[tauri::command]
pub async fn set_llm_api_key(
    state: State<'_, AppState>,
    vault_id: String,
    provider: LlmProvider,
    api_key: String,
) -> Result<(), CommandError> {
    info!(
        "Setting API key for {:?} provider in vault: {}",
        provider, vault_id
    );

    let pool = get_vault_pool(&state, &vault_id)?;

    // Set API key
    spectral_privacy::set_api_key(&pool, provider, &api_key)
        .await
        .map_err(|e| CommandError::new("PRIVACY_ERROR", format!("Failed to set API key: {}", e)))?;

    Ok(())
}

/// Test connection to an LLM provider.
///
/// Attempts to connect to the provider and make a simple test request.
/// Returns success if the provider is reachable and configured correctly.
#[tauri::command]
pub async fn test_llm_provider(
    state: State<'_, AppState>,
    vault_id: String,
    provider: LlmProvider,
) -> Result<String, CommandError> {
    info!(
        "Testing LLM provider {:?} for vault: {}",
        provider, vault_id
    );

    let pool = get_vault_pool(&state, &vault_id)?;

    // Create a minimal test request
    let test_request = CompletionRequest::new("test").with_max_tokens(5);

    // Test the provider by attempting to create it and make a request
    match provider {
        LlmProvider::Ollama => {
            // Try to create Ollama provider
            let ollama = OllamaProvider::new().map_err(|e| {
                CommandError::new(
                    "PROVIDER_ERROR",
                    format!("Failed to create Ollama provider: {}", e),
                )
            })?;

            // Test connection by making a minimal request
            spectral_llm::LlmProvider::complete(&ollama, test_request)
                .await
                .map_err(|e| {
                    CommandError::new(
                        "CONNECTION_ERROR",
                        format!("Ollama connection test failed: {}", e),
                    )
                })?;

            Ok("Ollama provider is reachable and responding".to_string())
        }
        LlmProvider::LmStudio => {
            // Try to create LM Studio provider
            let lm_studio = LmStudioProvider::new().map_err(|e| {
                CommandError::new(
                    "PROVIDER_ERROR",
                    format!("Failed to create LM Studio provider: {}", e),
                )
            })?;

            // Test connection by making a minimal request
            spectral_llm::LlmProvider::complete(&lm_studio, test_request)
                .await
                .map_err(|e| {
                    CommandError::new(
                        "CONNECTION_ERROR",
                        format!("LM Studio connection test failed: {}", e),
                    )
                })?;

            Ok("LM Studio provider is reachable and responding".to_string())
        }
        LlmProvider::OpenAi => {
            // Get API key from database
            let api_key = spectral_privacy::get_api_key(&pool, provider)
                .await
                .map_err(|e| {
                    CommandError::new("PRIVACY_ERROR", format!("Failed to get API key: {}", e))
                })?
                .ok_or_else(|| {
                    CommandError::new(
                        "API_KEY_MISSING",
                        "No API key configured for OpenAI. Use 'spectral privacy llm set-key openai <key>' to configure.".to_string(),
                    )
                })?;

            // Try to create OpenAI provider
            let openai = OpenAiProvider::new(api_key).map_err(|e| {
                CommandError::new(
                    "PROVIDER_ERROR",
                    format!("Failed to create OpenAI provider: {}", e),
                )
            })?;

            // Test connection by making a minimal request
            spectral_llm::LlmProvider::complete(&openai, test_request)
                .await
                .map_err(|e| {
                    CommandError::new(
                        "CONNECTION_ERROR",
                        format!("OpenAI connection test failed: {}", e),
                    )
                })?;

            Ok("OpenAI provider is reachable and responding".to_string())
        }
        LlmProvider::Gemini => {
            // Get API key from database
            let api_key = spectral_privacy::get_api_key(&pool, provider)
                .await
                .map_err(|e| {
                    CommandError::new("PRIVACY_ERROR", format!("Failed to get API key: {}", e))
                })?
                .ok_or_else(|| {
                    CommandError::new(
                        "API_KEY_MISSING",
                        "No API key configured for Gemini. Use 'spectral privacy llm set-key gemini <key>' to configure.".to_string(),
                    )
                })?;

            // Try to create Gemini provider
            let gemini = GeminiProvider::new(api_key).map_err(|e| {
                CommandError::new(
                    "PROVIDER_ERROR",
                    format!("Failed to create Gemini provider: {}", e),
                )
            })?;

            // Test connection by making a minimal request
            spectral_llm::LlmProvider::complete(&gemini, test_request)
                .await
                .map_err(|e| {
                    CommandError::new(
                        "CONNECTION_ERROR",
                        format!("Gemini connection test failed: {}", e),
                    )
                })?;

            Ok("Gemini provider is reachable and responding".to_string())
        }
        LlmProvider::Claude => {
            // Get API key from database
            let api_key = spectral_privacy::get_api_key(&pool, provider)
                .await
                .map_err(|e| {
                    CommandError::new("PRIVACY_ERROR", format!("Failed to get API key: {}", e))
                })?
                .ok_or_else(|| {
                    CommandError::new(
                        "API_KEY_MISSING",
                        "No API key configured for Claude. Use 'spectral privacy llm set-key claude <key>' to configure.".to_string(),
                    )
                })?;

            // Try to create Claude provider
            let claude = AnthropicProvider::new(api_key).map_err(|e| {
                CommandError::new(
                    "PROVIDER_ERROR",
                    format!("Failed to create Claude provider: {}", e),
                )
            })?;

            // Test connection by making a minimal request
            spectral_llm::LlmProvider::complete(&claude, test_request)
                .await
                .map_err(|e| {
                    CommandError::new(
                        "CONNECTION_ERROR",
                        format!("Claude connection test failed: {}", e),
                    )
                })?;

            Ok("Claude provider is reachable and responding".to_string())
        }
    }
}
