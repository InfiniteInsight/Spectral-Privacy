//! Integration tests for privacy engine
//!
//! Tests the complete flow of privacy settings, LLM preferences,
//! and permission checking.

use spectral_db::Database;
use spectral_privacy::{
    delete_api_key, get_api_key, get_primary_provider, get_provider_preference, set_api_key,
    set_primary_provider, set_provider_preference, Feature, LlmProvider, PrivacyEngine,
    PrivacyLevel, TaskType,
};
use sqlx::SqlitePool;

/// Create a test database with migrations
async fn create_test_db() -> SqlitePool {
    let key = vec![0u8; 32];
    let db = Database::new(":memory:", key)
        .await
        .expect("create test database");
    db.run_migrations().await.expect("run migrations");
    db.pool().clone()
}

#[tokio::test]
async fn test_privacy_level_flow() {
    let pool = create_test_db().await;
    let engine = PrivacyEngine::new(pool.clone());

    // Test default level (should be Balanced)
    let level = engine.get_privacy_level().await.unwrap();
    assert_eq!(level, PrivacyLevel::Balanced);

    // Test switching to LocalPrivacy (local-only)
    engine
        .set_privacy_level(PrivacyLevel::LocalPrivacy)
        .await
        .unwrap();
    let level = engine.get_privacy_level().await.unwrap();
    assert_eq!(level, PrivacyLevel::LocalPrivacy);

    // LocalPrivacy should disallow cloud LLMs
    let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
    assert!(
        !result.is_allowed(),
        "LocalPrivacy mode should reject cloud LLMs"
    );

    // LocalPrivacy should allow local LLMs
    let result = engine.check_permission(Feature::LocalLlm).await.unwrap();
    assert!(
        result.is_allowed(),
        "LocalPrivacy mode should allow local LLMs"
    );

    // Test switching to Balanced
    engine
        .set_privacy_level(PrivacyLevel::Balanced)
        .await
        .unwrap();
    let level = engine.get_privacy_level().await.unwrap();
    assert_eq!(level, PrivacyLevel::Balanced);

    // Balanced should allow both local and cloud LLMs
    let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
    assert!(result.is_allowed(), "Balanced mode should allow cloud LLMs");

    let result = engine.check_permission(Feature::LocalLlm).await.unwrap();
    assert!(result.is_allowed(), "Balanced mode should allow local LLMs");

    // Test Paranoid mode
    engine
        .set_privacy_level(PrivacyLevel::Paranoid)
        .await
        .unwrap();

    let result = engine.check_permission(Feature::LocalLlm).await.unwrap();
    assert!(!result.is_allowed(), "Paranoid mode should reject all LLMs");

    let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
    assert!(!result.is_allowed(), "Paranoid mode should reject all LLMs");
}

#[tokio::test]
async fn test_llm_provider_preferences() {
    let pool = create_test_db().await;

    // Test default primary provider (None when not set)
    let primary = get_primary_provider(&pool).await.unwrap();
    assert_eq!(primary, None);

    // Set primary to Claude
    set_primary_provider(&pool, LlmProvider::Claude)
        .await
        .unwrap();
    let primary = get_primary_provider(&pool).await.unwrap();
    assert_eq!(primary, Some(LlmProvider::Claude));

    // Test task-specific override
    set_provider_preference(&pool, TaskType::EmailDraft, LlmProvider::OpenAi)
        .await
        .unwrap();

    let task_provider = get_provider_preference(&pool, TaskType::EmailDraft)
        .await
        .unwrap();
    assert_eq!(task_provider, Some(LlmProvider::OpenAi));

    // Test fallback to None for unset task
    let task_provider = get_provider_preference(&pool, TaskType::FormFill)
        .await
        .unwrap();
    assert_eq!(task_provider, None);

    // Test multiple task-specific providers
    set_provider_preference(&pool, TaskType::EmailDraft, LlmProvider::OpenAi)
        .await
        .unwrap();
    set_provider_preference(&pool, TaskType::FormFill, LlmProvider::Claude)
        .await
        .unwrap();

    assert_eq!(
        get_provider_preference(&pool, TaskType::EmailDraft)
            .await
            .unwrap(),
        Some(LlmProvider::OpenAi)
    );
    assert_eq!(
        get_provider_preference(&pool, TaskType::FormFill)
            .await
            .unwrap(),
        Some(LlmProvider::Claude)
    );

    // Test local provider preferences
    set_provider_preference(&pool, TaskType::EmailDraft, LlmProvider::Ollama)
        .await
        .unwrap();
    assert_eq!(
        get_provider_preference(&pool, TaskType::EmailDraft)
            .await
            .unwrap(),
        Some(LlmProvider::Ollama)
    );
    assert!(LlmProvider::Ollama.is_local());
}

#[tokio::test]
async fn test_router_permission_enforcement() {
    let pool = create_test_db().await;
    let engine = PrivacyEngine::new(pool.clone());

    // Test Balanced mode - allows cloud LLMs
    engine
        .set_privacy_level(PrivacyLevel::Balanced)
        .await
        .unwrap();

    let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
    assert!(result.is_allowed(), "Balanced mode should allow cloud LLMs");

    let result = engine.check_permission(Feature::LocalLlm).await.unwrap();
    assert!(result.is_allowed(), "Balanced mode should allow local LLMs");

    // Test LocalPrivacy mode - rejects cloud, allows local
    engine
        .set_privacy_level(PrivacyLevel::LocalPrivacy)
        .await
        .unwrap();

    let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
    assert!(
        !result.is_allowed(),
        "LocalPrivacy mode should reject cloud LLMs"
    );

    let result = engine.check_permission(Feature::LocalLlm).await.unwrap();
    assert!(
        result.is_allowed(),
        "LocalPrivacy mode should allow local LLMs"
    );

    // Test Paranoid mode - rejects all automation
    engine
        .set_privacy_level(PrivacyLevel::Paranoid)
        .await
        .unwrap();

    let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
    assert!(
        !result.is_allowed(),
        "Paranoid mode should reject cloud LLMs"
    );

    let result = engine.check_permission(Feature::LocalLlm).await.unwrap();
    assert!(
        !result.is_allowed(),
        "Paranoid mode should reject local LLMs"
    );

    let result = engine
        .check_permission(Feature::BrowserAutomation)
        .await
        .unwrap();
    assert!(
        !result.is_allowed(),
        "Paranoid mode should reject browser automation"
    );

    // Test other features in Balanced mode
    engine
        .set_privacy_level(PrivacyLevel::Balanced)
        .await
        .unwrap();

    let result = engine
        .check_permission(Feature::EmailSending)
        .await
        .unwrap();
    assert!(result.is_allowed(), "Balanced mode should allow email");

    let result = engine.check_permission(Feature::PiiScanning).await.unwrap();
    assert!(
        result.is_allowed(),
        "Balanced mode should allow PII scanning"
    );
}

#[tokio::test]
async fn test_api_key_storage() {
    let pool = create_test_db().await;

    // Test storing and retrieving API key
    set_api_key(&pool, LlmProvider::OpenAi, "sk-test-key-123")
        .await
        .unwrap();

    let key = get_api_key(&pool, LlmProvider::OpenAi).await.unwrap();
    assert_eq!(key, Some("sk-test-key-123".to_string()));

    // Test updating API key
    set_api_key(&pool, LlmProvider::OpenAi, "sk-test-key-456")
        .await
        .unwrap();

    let key = get_api_key(&pool, LlmProvider::OpenAi).await.unwrap();
    assert_eq!(key, Some("sk-test-key-456".to_string()));

    // Test deleting API key
    delete_api_key(&pool, LlmProvider::OpenAi).await.unwrap();

    let key = get_api_key(&pool, LlmProvider::OpenAi).await.unwrap();
    assert_eq!(key, None);

    // Test multiple providers
    set_api_key(&pool, LlmProvider::OpenAi, "sk-openai-key")
        .await
        .unwrap();
    set_api_key(&pool, LlmProvider::Claude, "sk-ant-key")
        .await
        .unwrap();
    set_api_key(&pool, LlmProvider::Gemini, "goog-key")
        .await
        .unwrap();

    let openai_key = get_api_key(&pool, LlmProvider::OpenAi).await.unwrap();
    let claude_key = get_api_key(&pool, LlmProvider::Claude).await.unwrap();
    let gemini_key = get_api_key(&pool, LlmProvider::Gemini).await.unwrap();

    assert_eq!(openai_key, Some("sk-openai-key".to_string()));
    assert_eq!(claude_key, Some("sk-ant-key".to_string()));
    assert_eq!(gemini_key, Some("goog-key".to_string()));

    // Test local providers don't require API keys
    let ollama_key = get_api_key(&pool, LlmProvider::Ollama).await.unwrap();
    assert_eq!(ollama_key, None);

    // Cleanup
    delete_api_key(&pool, LlmProvider::OpenAi).await.unwrap();
    delete_api_key(&pool, LlmProvider::Claude).await.unwrap();
    delete_api_key(&pool, LlmProvider::Gemini).await.unwrap();
}
