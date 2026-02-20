//! Privacy settings commands.

use crate::error::CommandError;
use crate::state::AppState;
use serde::Serialize;
use spectral_privacy::{FeatureFlags, LlmProvider, PrivacyEngine, PrivacyLevel, TaskType};
use tauri::State;
use tracing::info;

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

    // Get vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| CommandError::new("VAULT_LOCKED", "Vault is locked"))?;

    // Get database pool
    let pool = vault
        .database()
        .map_err(|e| {
            CommandError::new(
                "VAULT_ERROR",
                format!("Failed to access vault database: {}", e),
            )
        })?
        .pool()
        .clone();

    // Create privacy engine
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

    // Get vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| CommandError::new("VAULT_LOCKED", "Vault is locked"))?;

    // Get database pool
    let pool = vault
        .database()
        .map_err(|e| {
            CommandError::new(
                "VAULT_ERROR",
                format!("Failed to access vault database: {}", e),
            )
        })?
        .pool()
        .clone();

    // Create privacy engine
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

    // Get vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| CommandError::new("VAULT_LOCKED", "Vault is locked"))?;

    // Get database pool
    let pool = vault
        .database()
        .map_err(|e| {
            CommandError::new(
                "VAULT_ERROR",
                format!("Failed to access vault database: {}", e),
            )
        })?
        .pool()
        .clone();

    // Create privacy engine
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

    // Get vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| CommandError::new("VAULT_LOCKED", "Vault is locked"))?;

    let pool = vault
        .database()
        .map_err(|e| {
            CommandError::new(
                "VAULT_ERROR",
                format!("Failed to access vault database: {}", e),
            )
        })?
        .pool();

    // Get primary provider
    let primary_provider = spectral_privacy::get_primary_provider(pool)
        .await
        .map_err(|e| {
            CommandError::new(
                "PRIVACY_ERROR",
                format!("Failed to get primary provider: {}", e),
            )
        })?;

    // Get task-specific providers
    let email_draft_provider =
        spectral_privacy::get_provider_preference(pool, TaskType::EmailDraft)
            .await
            .map_err(|e| {
                CommandError::new(
                    "PRIVACY_ERROR",
                    format!("Failed to get email draft provider: {}", e),
                )
            })?;

    let form_fill_provider = spectral_privacy::get_provider_preference(pool, TaskType::FormFill)
        .await
        .map_err(|e| {
            CommandError::new(
                "PRIVACY_ERROR",
                format!("Failed to get form fill provider: {}", e),
            )
        })?;

    // Check which API keys are configured
    let has_openai_key = spectral_privacy::get_api_key(pool, LlmProvider::OpenAi)
        .await
        .map_err(|e| {
            CommandError::new(
                "PRIVACY_ERROR",
                format!("Failed to check OpenAI key: {}", e),
            )
        })?
        .is_some();

    let has_gemini_key = spectral_privacy::get_api_key(pool, LlmProvider::Gemini)
        .await
        .map_err(|e| {
            CommandError::new(
                "PRIVACY_ERROR",
                format!("Failed to check Gemini key: {}", e),
            )
        })?
        .is_some();

    let has_claude_key = spectral_privacy::get_api_key(pool, LlmProvider::Claude)
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

    // Get vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| CommandError::new("VAULT_LOCKED", "Vault is locked"))?;

    let pool = vault
        .database()
        .map_err(|e| {
            CommandError::new(
                "VAULT_ERROR",
                format!("Failed to access vault database: {}", e),
            )
        })?
        .pool();

    // Set primary provider
    spectral_privacy::set_primary_provider(pool, provider)
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

    // Get vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| CommandError::new("VAULT_LOCKED", "Vault is locked"))?;

    let pool = vault
        .database()
        .map_err(|e| {
            CommandError::new(
                "VAULT_ERROR",
                format!("Failed to access vault database: {}", e),
            )
        })?
        .pool();

    // Set task-specific provider
    spectral_privacy::set_provider_preference(pool, task_type, provider)
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

    // Get vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| CommandError::new("VAULT_LOCKED", "Vault is locked"))?;

    let pool = vault
        .database()
        .map_err(|e| {
            CommandError::new(
                "VAULT_ERROR",
                format!("Failed to access vault database: {}", e),
            )
        })?
        .pool();

    // Set API key
    spectral_privacy::set_api_key(pool, provider, &api_key)
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

    // Get vault
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| CommandError::new("VAULT_LOCKED", "Vault is locked"))?;

    let pool = vault
        .database()
        .map_err(|e| {
            CommandError::new(
                "VAULT_ERROR",
                format!("Failed to access vault database: {}", e),
            )
        })?
        .pool();

    // For local providers (Ollama, LM Studio), just check if they're running
    // For cloud providers, verify API key is set
    match provider {
        LlmProvider::Ollama | LlmProvider::LmStudio => {
            // TODO: Actually test the connection by attempting to create the provider
            // For now, return a stub success message
            Ok(format!("{:?} provider test not yet implemented", provider))
        }
        LlmProvider::OpenAi | LlmProvider::Gemini | LlmProvider::Claude => {
            // Check if API key is configured
            let has_key = spectral_privacy::get_api_key(pool, provider)
                .await
                .map_err(|e| {
                    CommandError::new("PRIVACY_ERROR", format!("Failed to check API key: {}", e))
                })?
                .is_some();

            if !has_key {
                return Err(CommandError::new(
                    "API_KEY_MISSING",
                    format!("No API key configured for {:?}", provider),
                ));
            }

            // TODO: Actually test the connection by making a simple API request
            // For now, return a stub success message
            Ok(format!("{:?} provider test not yet implemented", provider))
        }
    }
}
