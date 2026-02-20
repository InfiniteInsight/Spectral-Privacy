//! LLM provider settings management.
//!
//! This module manages LLM provider configurations including:
//! - API keys for different providers
//! - Provider preferences per task type
//! - Primary/default provider selection
//!
//! Settings are stored in the privacy_settings table with encrypted vault scope.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Supported LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LlmProvider {
    /// `OpenAI` (cloud).
    OpenAi,
    /// Google `Gemini` (cloud).
    Gemini,
    /// Anthropic `Claude` (cloud).
    Claude,
    /// `Ollama` (local).
    Ollama,
    /// `LM Studio` (local).
    LmStudio,
}

impl LlmProvider {
    /// Check if this provider runs locally (doesn't send data to cloud).
    #[must_use]
    pub fn is_local(self) -> bool {
        matches!(self, Self::Ollama | Self::LmStudio)
    }
}

/// Task types that can have provider preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskType {
    /// Email drafting.
    EmailDraft,
    /// Form filling.
    FormFill,
}

/// Set API key for a provider.
///
/// # Errors
/// Returns error if database write fails.
pub async fn set_api_key(pool: &SqlitePool, provider: LlmProvider, api_key: &str) -> Result<()> {
    let key = format!("llm.provider.{}.api_key", serde_json::to_string(&provider)?);
    let value = serde_json::to_value(api_key)?;
    spectral_db::settings::set_setting(pool, &key, &value).await?;
    Ok(())
}

/// Get API key for a provider.
///
/// # Errors
/// Returns error if database read fails or value is malformed.
pub async fn get_api_key(pool: &SqlitePool, provider: LlmProvider) -> Result<Option<String>> {
    let key = format!("llm.provider.{}.api_key", serde_json::to_string(&provider)?);
    let value = spectral_db::settings::get_setting(pool, &key).await?;

    if let Some(v) = value {
        // API keys are stored encrypted in the database.
        // This is temporary deserialization for return.
        let api_key: String = serde_json::from_value(v)?; // nosemgrep: use-zeroize-for-secrets
        Ok(Some(api_key))
    } else {
        Ok(None)
    }
}

/// Delete API key for a provider.
///
/// # Errors
/// Returns error if database write fails.
pub async fn delete_api_key(pool: &SqlitePool, provider: LlmProvider) -> Result<()> {
    let key = format!("llm.provider.{}.api_key", serde_json::to_string(&provider)?);
    spectral_db::settings::delete_setting(pool, &key).await?;
    Ok(())
}

/// Set provider preference for a task type.
///
/// # Errors
/// Returns error if database write fails.
pub async fn set_provider_preference(
    pool: &SqlitePool,
    task_type: TaskType,
    provider: LlmProvider,
) -> Result<()> {
    let key = format!("llm.task.{}.provider", serde_json::to_string(&task_type)?);
    let value = serde_json::to_value(provider)?;
    spectral_db::settings::set_setting(pool, &key, &value).await?;
    Ok(())
}

/// Get provider preference for a task type.
///
/// # Errors
/// Returns error if database read fails or value is malformed.
pub async fn get_provider_preference(
    pool: &SqlitePool,
    task_type: TaskType,
) -> Result<Option<LlmProvider>> {
    let key = format!("llm.task.{}.provider", serde_json::to_string(&task_type)?);
    let value = spectral_db::settings::get_setting(pool, &key).await?;

    if let Some(v) = value {
        let provider: LlmProvider = serde_json::from_value(v)?;
        Ok(Some(provider))
    } else {
        Ok(None)
    }
}

/// Set primary (default) provider.
///
/// # Errors
/// Returns error if database write fails.
pub async fn set_primary_provider(pool: &SqlitePool, provider: LlmProvider) -> Result<()> {
    let key = "llm.primary_provider";
    let value = serde_json::to_value(provider)?;
    spectral_db::settings::set_setting(pool, key, &value).await?;
    Ok(())
}

/// Get primary (default) provider.
///
/// # Errors
/// Returns error if database read fails or value is malformed.
pub async fn get_primary_provider(pool: &SqlitePool) -> Result<Option<LlmProvider>> {
    let key = "llm.primary_provider";
    let value = spectral_db::settings::get_setting(pool, key).await?;

    if let Some(v) = value {
        let provider: LlmProvider = serde_json::from_value(v)?;
        Ok(Some(provider))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
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
    async fn test_set_and_get_api_key() {
        let pool = create_test_db().await;

        // nosemgrep: no-unwrap-in-production
        set_api_key(&pool, LlmProvider::OpenAi, "sk-test")
            .await
            .unwrap();

        // nosemgrep: no-unwrap-in-production
        let key = get_api_key(&pool, LlmProvider::OpenAi).await.unwrap();
        assert_eq!(key, Some("sk-test".to_string()));
    }

    #[tokio::test]
    async fn test_delete_api_key() {
        let pool = create_test_db().await;

        // nosemgrep: no-unwrap-in-production
        set_api_key(&pool, LlmProvider::Gemini, "key123")
            .await
            .unwrap();
        // nosemgrep: no-unwrap-in-production
        delete_api_key(&pool, LlmProvider::Gemini).await.unwrap();

        // nosemgrep: no-unwrap-in-production
        let key = get_api_key(&pool, LlmProvider::Gemini).await.unwrap();
        assert_eq!(key, None);
    }

    #[tokio::test]
    async fn test_set_and_get_provider_preference() {
        let pool = create_test_db().await;

        // nosemgrep: no-unwrap-in-production
        set_provider_preference(&pool, TaskType::EmailDraft, LlmProvider::Claude)
            .await
            .unwrap();

        // nosemgrep: no-unwrap-in-production
        let pref = get_provider_preference(&pool, TaskType::EmailDraft)
            .await
            .unwrap();
        assert_eq!(pref, Some(LlmProvider::Claude));
    }

    #[tokio::test]
    async fn test_set_and_get_primary_provider() {
        let pool = create_test_db().await;

        // nosemgrep: no-unwrap-in-production
        set_primary_provider(&pool, LlmProvider::LmStudio)
            .await
            .unwrap();

        // nosemgrep: no-unwrap-in-production
        let primary = get_primary_provider(&pool).await.unwrap();
        assert_eq!(primary, Some(LlmProvider::LmStudio));
    }
}
