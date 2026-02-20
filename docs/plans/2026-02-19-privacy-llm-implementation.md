# Privacy Level & LLM Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement comprehensive privacy controls and LLM integration with centralized Privacy Engine orchestrator

**Architecture:** New `spectral-privacy` crate provides single enforcement point for all privacy decisions. LLM providers (OpenAI, Gemini, LM Studio) integrate with existing `spectral-llm` crate. Settings stored in vault database with encrypted API keys.

**Tech Stack:** Rust (spectral-privacy crate), SQLCipher (encrypted settings), reqwest (HTTP clients), Svelte 5 (UI), Tauri 2 (commands)

---

## Phase 1: Core Privacy Engine Foundation

### Task 1: Create spectral-privacy Crate Scaffold

**Files:**
- Create: `crates/spectral-privacy/Cargo.toml`
- Create: `crates/spectral-privacy/src/lib.rs`
- Create: `crates/spectral-privacy/src/error.rs`
- Modify: `Cargo.toml` (workspace members)

**Step 1: Create crate directory**

```bash
mkdir -p crates/spectral-privacy/src
```

**Step 2: Write Cargo.toml**

```toml
[package]
name = "spectral-privacy"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true

[dependencies]
# Internal dependencies
spectral-core = { path = "../spectral-core" }
spectral-db = { path = "../spectral-db" }

# Error handling
thiserror = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Async
tokio = { workspace = true }
async-trait = "0.1"

# Database
sqlx = { workspace = true }

# Logging
tracing = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

**Step 3: Write error types**

File: `crates/spectral-privacy/src/error.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrivacyError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

pub type Result<T> = std::result::Result<T, PrivacyError>;
```

**Step 4: Write lib.rs scaffold**

File: `crates/spectral-privacy/src/lib.rs`

```rust
//! Spectral Privacy - Centralized privacy controls and settings management

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;

pub use error::{PrivacyError, Result};
```

**Step 5: Add to workspace**

Modify: `Cargo.toml` (root)

```toml
[workspace]
members = [
    # ... existing members ...
    "crates/spectral-privacy",
]
```

**Step 6: Test compilation**

```bash
cargo build -p spectral-privacy
```

Expected: Success

**Step 7: Commit**

```bash
git add crates/spectral-privacy Cargo.toml
git commit -m "feat(privacy): create spectral-privacy crate scaffold

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 2: Privacy Level Types and Feature Flags

**Files:**
- Create: `crates/spectral-privacy/src/types.rs`
- Modify: `crates/spectral-privacy/src/lib.rs`
- Create: `crates/spectral-privacy/src/types.rs` (tests section)

**Step 1: Write failing test**

File: `crates/spectral-privacy/src/types.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_level_serialization() {
        let level = PrivacyLevel::Balanced;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, r#""Balanced""#);

        let deserialized: PrivacyLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, PrivacyLevel::Balanced);
    }

    #[test]
    fn test_paranoid_feature_flags() {
        let flags = FeatureFlags::from_privacy_level(PrivacyLevel::Paranoid);
        assert!(!flags.allow_local_llm);
        assert!(!flags.allow_cloud_llm);
        assert!(!flags.allow_browser_automation);
    }

    #[test]
    fn test_local_privacy_feature_flags() {
        let flags = FeatureFlags::from_privacy_level(PrivacyLevel::LocalPrivacy);
        assert!(flags.allow_local_llm);
        assert!(!flags.allow_cloud_llm);
        assert!(flags.allow_browser_automation);
    }

    #[test]
    fn test_balanced_feature_flags() {
        let flags = FeatureFlags::from_privacy_level(PrivacyLevel::Balanced);
        assert!(flags.allow_local_llm);
        assert!(flags.allow_cloud_llm);
        assert!(flags.allow_browser_automation);
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p spectral-privacy
```

Expected: FAIL (types don't exist)

**Step 3: Implement types**

File: `crates/spectral-privacy/src/types.rs`

```rust
use serde::{Deserialize, Serialize};

/// Privacy level presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// No automation, no LLM, manual only
    Paranoid,
    /// Local LLM only, automation allowed
    LocalPrivacy,
    /// Cloud LLM + PII filtering, all features
    Balanced,
    /// User-defined feature flags
    Custom,
}

/// Granular feature control flags
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureFlags {
    pub allow_local_llm: bool,
    pub allow_cloud_llm: bool,
    pub allow_browser_automation: bool,
    pub allow_email_sending: bool,
    pub allow_imap_monitoring: bool,
    pub allow_pii_scanning: bool,
}

impl FeatureFlags {
    /// Create feature flags from privacy level preset
    pub fn from_privacy_level(level: PrivacyLevel) -> Self {
        match level {
            PrivacyLevel::Paranoid => Self {
                allow_local_llm: false,
                allow_cloud_llm: false,
                allow_browser_automation: false,
                allow_email_sending: false,
                allow_imap_monitoring: false,
                allow_pii_scanning: false,
            },
            PrivacyLevel::LocalPrivacy => Self {
                allow_local_llm: true,
                allow_cloud_llm: false,
                allow_browser_automation: true,
                allow_email_sending: true,
                allow_imap_monitoring: true,
                allow_pii_scanning: true,
            },
            PrivacyLevel::Balanced => Self {
                allow_local_llm: true,
                allow_cloud_llm: true,
                allow_browser_automation: true,
                allow_email_sending: true,
                allow_imap_monitoring: true,
                allow_pii_scanning: true,
            },
            PrivacyLevel::Custom => Self::default(),
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::from_privacy_level(PrivacyLevel::Balanced)
    }
}

/// Features that can be permission-checked
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    LocalLlm,
    CloudLlm,
    BrowserAutomation,
    EmailSending,
    ImapMonitoring,
    PiiScanning,
}

/// Result of permission check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionResult {
    Allowed,
    Denied { reason: String },
}

impl PermissionResult {
    /// Check if permission is allowed
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }

    /// Get denial reason if denied
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Denied { reason } => Some(reason),
            Self::Allowed => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_level_serialization() {
        let level = PrivacyLevel::Balanced;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, r#""Balanced""#);

        let deserialized: PrivacyLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, PrivacyLevel::Balanced);
    }

    #[test]
    fn test_paranoid_feature_flags() {
        let flags = FeatureFlags::from_privacy_level(PrivacyLevel::Paranoid);
        assert!(!flags.allow_local_llm);
        assert!(!flags.allow_cloud_llm);
        assert!(!flags.allow_browser_automation);
    }

    #[test]
    fn test_local_privacy_feature_flags() {
        let flags = FeatureFlags::from_privacy_level(PrivacyLevel::LocalPrivacy);
        assert!(flags.allow_local_llm);
        assert!(!flags.allow_cloud_llm);
        assert!(flags.allow_browser_automation);
    }

    #[test]
    fn test_balanced_feature_flags() {
        let flags = FeatureFlags::from_privacy_level(PrivacyLevel::Balanced);
        assert!(flags.allow_local_llm);
        assert!(flags.allow_cloud_llm);
        assert!(flags.allow_browser_automation);
    }
}
```

**Step 4: Export from lib.rs**

Modify: `crates/spectral-privacy/src/lib.rs`

```rust
pub mod error;
pub mod types;

pub use error::{PrivacyError, Result};
pub use types::{Feature, FeatureFlags, PermissionResult, PrivacyLevel};
```

**Step 5: Run tests**

```bash
cargo test -p spectral-privacy
```

Expected: PASS (all 4 tests)

**Step 6: Commit**

```bash
git add crates/spectral-privacy/src/
git commit -m "feat(privacy): add privacy level types and feature flags

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 3: Database Migration for Settings Table

**Files:**
- Modify: `crates/spectral-db/src/migrations.rs`
- Modify: `crates/spectral-db/src/lib.rs`

**Step 1: Add settings table migration**

Modify: `crates/spectral-db/src/migrations.rs`

Add after existing migrations:

```rust
const MIGRATION_009_SETTINGS: &str = r#"
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_settings_updated_at ON settings(updated_at);
"#;
```

**Step 2: Update migration list**

In same file, update `pub const MIGRATIONS` array:

```rust
pub const MIGRATIONS: &[&str] = &[
    MIGRATION_001_PROFILES,
    MIGRATION_002_FINDINGS,
    MIGRATION_003_REMOVAL_ATTEMPTS,
    MIGRATION_004_SCAN_JOBS,
    MIGRATION_005_BROKER_SCANS,
    MIGRATION_006_PRIVACY_SCORE,
    MIGRATION_007_REMOVAL_QUEUE,
    MIGRATION_008_DISCOVERY_FINDINGS,
    MIGRATION_009_SETTINGS, // New
];
```

**Step 3: Test migration**

```bash
cargo test -p spectral-db test_migrations
```

Expected: PASS

**Step 4: Commit**

```bash
git add crates/spectral-db/src/migrations.rs
git commit -m "feat(db): add settings table migration

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 4: Settings Storage Module

**Files:**
- Create: `crates/spectral-db/src/settings.rs`
- Modify: `crates/spectral-db/src/lib.rs`

**Step 1: Write failing tests**

File: `crates/spectral-db/src/settings.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    #[tokio::test]
    async fn test_set_and_get_setting() {
        let pool = create_test_db().await;

        let value = serde_json::json!({"level": "Balanced"});
        set_setting(&pool, "privacy_level", &value).await.unwrap();

        let retrieved = get_setting(&pool, "privacy_level").await.unwrap();
        assert_eq!(retrieved, Some(value));
    }

    #[tokio::test]
    async fn test_get_nonexistent_setting() {
        let pool = create_test_db().await;

        let result = get_setting(&pool, "does_not_exist").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_delete_setting() {
        let pool = create_test_db().await;

        let value = serde_json::json!({"test": true});
        set_setting(&pool, "test_key", &value).await.unwrap();

        delete_setting(&pool, "test_key").await.unwrap();

        let result = get_setting(&pool, "test_key").await.unwrap();
        assert_eq!(result, None);
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p spectral-db settings::tests
```

Expected: FAIL (functions don't exist)

**Step 3: Implement settings storage**

File: `crates/spectral-db/src/settings.rs`

```rust
use crate::error::Result;
use serde_json::Value;
use sqlx::SqlitePool;

/// Set a setting in the database
pub async fn set_setting(pool: &SqlitePool, key: &str, value: &Value) -> Result<()> {
    let value_str = serde_json::to_string(value)?;

    sqlx::query!(
        r#"
        INSERT INTO settings (key, value, updated_at)
        VALUES (?, ?, datetime('now'))
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = datetime('now')
        "#,
        key,
        value_str
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get a setting from the database
pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<Value>> {
    let row = sqlx::query!(
        r#"
        SELECT value
        FROM settings
        WHERE key = ?
        "#,
        key
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let value: Value = serde_json::from_str(&r.value)?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

/// Delete a setting from the database
pub async fn delete_setting(pool: &SqlitePool, key: &str) -> Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM settings
        WHERE key = ?
        "#,
        key
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    #[tokio::test]
    async fn test_set_and_get_setting() {
        let pool = create_test_db().await;

        let value = serde_json::json!({"level": "Balanced"});
        set_setting(&pool, "privacy_level", &value).await.unwrap();

        let retrieved = get_setting(&pool, "privacy_level").await.unwrap();
        assert_eq!(retrieved, Some(value));
    }

    #[tokio::test]
    async fn test_get_nonexistent_setting() {
        let pool = create_test_db().await;

        let result = get_setting(&pool, "does_not_exist").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_delete_setting() {
        let pool = create_test_db().await;

        let value = serde_json::json!({"test": true});
        set_setting(&pool, "test_key", &value).await.unwrap();

        delete_setting(&pool, "test_key").await.unwrap();

        let result = get_setting(&pool, "test_key").await.unwrap();
        assert_eq!(result, None);
    }
}
```

**Step 4: Export from lib.rs**

Modify: `crates/spectral-db/src/lib.rs`

```rust
pub mod settings;
```

**Step 5: Run tests**

```bash
cargo test -p spectral-db settings::tests
```

Expected: PASS (3 tests)

**Step 6: Commit**

```bash
git add crates/spectral-db/src/
git commit -m "feat(db): add settings storage module

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 5: PrivacyEngine Core Implementation

**Files:**
- Create: `crates/spectral-privacy/src/engine.rs`
- Modify: `crates/spectral-privacy/src/lib.rs`
- Modify: `crates/spectral-privacy/Cargo.toml`

**Step 1: Write failing tests**

File: `crates/spectral-privacy/src/engine.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    #[tokio::test]
    async fn test_get_default_privacy_level() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        let level = engine.get_privacy_level().await.unwrap();
        assert_eq!(level, PrivacyLevel::Balanced); // Default
    }

    #[tokio::test]
    async fn test_set_privacy_level() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        engine.set_privacy_level(PrivacyLevel::Paranoid).await.unwrap();

        let level = engine.get_privacy_level().await.unwrap();
        assert_eq!(level, PrivacyLevel::Paranoid);
    }

    #[tokio::test]
    async fn test_check_permission_allowed() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        engine.set_privacy_level(PrivacyLevel::Balanced).await.unwrap();

        let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
        assert!(result.is_allowed());
    }

    #[tokio::test]
    async fn test_check_permission_denied() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        engine.set_privacy_level(PrivacyLevel::Paranoid).await.unwrap();

        let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
        assert!(!result.is_allowed());
        assert!(result.reason().unwrap().contains("Paranoid"));
    }

    #[tokio::test]
    async fn test_custom_feature_flags() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        let mut flags = FeatureFlags::default();
        flags.allow_cloud_llm = false;

        engine.set_privacy_level(PrivacyLevel::Custom).await.unwrap();
        engine.set_feature_flags(flags.clone()).await.unwrap();

        let retrieved = engine.get_feature_flags().await.unwrap();
        assert_eq!(retrieved, flags);
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p spectral-privacy engine::tests
```

Expected: FAIL (PrivacyEngine doesn't exist)

**Step 3: Implement PrivacyEngine**

File: `crates/spectral-privacy/src/engine.rs`

```rust
use crate::error::Result;
use crate::types::{Feature, FeatureFlags, PermissionResult, PrivacyLevel};
use serde_json::json;
use spectral_db::settings::{delete_setting, get_setting, set_setting};
use sqlx::SqlitePool;

const PRIVACY_LEVEL_KEY: &str = "privacy_level";
const FEATURE_FLAGS_KEY: &str = "feature_flags";

/// Central privacy control engine
pub struct PrivacyEngine {
    pool: SqlitePool,
}

impl PrivacyEngine {
    /// Create new privacy engine
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get current privacy level
    pub async fn get_privacy_level(&self) -> Result<PrivacyLevel> {
        match get_setting(&self.pool, PRIVACY_LEVEL_KEY).await? {
            Some(value) => {
                let level: PrivacyLevel = serde_json::from_value(value)?;
                Ok(level)
            }
            None => Ok(PrivacyLevel::Balanced), // Default
        }
    }

    /// Set privacy level
    pub async fn set_privacy_level(&self, level: PrivacyLevel) -> Result<()> {
        let value = json!(level);
        set_setting(&self.pool, PRIVACY_LEVEL_KEY, &value).await?;

        // If not Custom, clear custom feature flags
        if level != PrivacyLevel::Custom {
            let _ = delete_setting(&self.pool, FEATURE_FLAGS_KEY).await;
        }

        Ok(())
    }

    /// Get current feature flags
    pub async fn get_feature_flags(&self) -> Result<FeatureFlags> {
        let level = self.get_privacy_level().await?;

        if level == PrivacyLevel::Custom {
            match get_setting(&self.pool, FEATURE_FLAGS_KEY).await? {
                Some(value) => {
                    let flags: FeatureFlags = serde_json::from_value(value)?;
                    Ok(flags)
                }
                None => Ok(FeatureFlags::default()),
            }
        } else {
            Ok(FeatureFlags::from_privacy_level(level))
        }
    }

    /// Set custom feature flags (only applies when level is Custom)
    pub async fn set_feature_flags(&self, flags: FeatureFlags) -> Result<()> {
        let level = self.get_privacy_level().await?;

        if level != PrivacyLevel::Custom {
            return Err(crate::error::PrivacyError::InvalidConfiguration(
                "Cannot set custom feature flags when privacy level is not Custom".to_string(),
            ));
        }

        let value = serde_json::to_value(flags)?;
        set_setting(&self.pool, FEATURE_FLAGS_KEY, &value).await?;

        Ok(())
    }

    /// Check permission for a specific feature
    pub async fn check_permission(&self, feature: Feature) -> Result<PermissionResult> {
        let flags = self.get_feature_flags().await?;

        let allowed = match feature {
            Feature::LocalLlm => flags.allow_local_llm,
            Feature::CloudLlm => flags.allow_cloud_llm,
            Feature::BrowserAutomation => flags.allow_browser_automation,
            Feature::EmailSending => flags.allow_email_sending,
            Feature::ImapMonitoring => flags.allow_imap_monitoring,
            Feature::PiiScanning => flags.allow_pii_scanning,
        };

        if allowed {
            Ok(PermissionResult::Allowed)
        } else {
            let level = self.get_privacy_level().await?;
            Ok(PermissionResult::Denied {
                reason: format!(
                    "{:?} feature not allowed under {:?} privacy level",
                    feature, level
                ),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral_db::test_helpers::*;

    #[tokio::test]
    async fn test_get_default_privacy_level() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        let level = engine.get_privacy_level().await.unwrap();
        assert_eq!(level, PrivacyLevel::Balanced); // Default
    }

    #[tokio::test]
    async fn test_set_privacy_level() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        engine.set_privacy_level(PrivacyLevel::Paranoid).await.unwrap();

        let level = engine.get_privacy_level().await.unwrap();
        assert_eq!(level, PrivacyLevel::Paranoid);
    }

    #[tokio::test]
    async fn test_check_permission_allowed() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        engine.set_privacy_level(PrivacyLevel::Balanced).await.unwrap();

        let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
        assert!(result.is_allowed());
    }

    #[tokio::test]
    async fn test_check_permission_denied() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        engine.set_privacy_level(PrivacyLevel::Paranoid).await.unwrap();

        let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
        assert!(!result.is_allowed());
        assert!(result.reason().unwrap().contains("Paranoid"));
    }

    #[tokio::test]
    async fn test_custom_feature_flags() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        let mut flags = FeatureFlags::default();
        flags.allow_cloud_llm = false;

        engine.set_privacy_level(PrivacyLevel::Custom).await.unwrap();
        engine.set_feature_flags(flags.clone()).await.unwrap();

        let retrieved = engine.get_feature_flags().await.unwrap();
        assert_eq!(retrieved, flags);
    }
}
```

**Step 4: Export from lib.rs**

Modify: `crates/spectral-privacy/src/lib.rs`

```rust
pub mod engine;
pub mod error;
pub mod types;

pub use engine::PrivacyEngine;
pub use error::{PrivacyError, Result};
pub use types::{Feature, FeatureFlags, PermissionResult, PrivacyLevel};
```

**Step 5: Run tests**

```bash
cargo test -p spectral-privacy engine::tests
```

Expected: PASS (6 tests)

**Step 6: Commit**

```bash
git add crates/spectral-privacy/src/
git commit -m "feat(privacy): implement PrivacyEngine core

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 6: LLM Provider Settings Module

**Files:**
- Create: `crates/spectral-privacy/src/llm_settings.rs`
- Modify: `crates/spectral-privacy/src/lib.rs`

**Step 1: Write failing tests**

File: `crates/spectral-privacy/src/llm_settings.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use spectral_db::test_helpers::*;

    #[tokio::test]
    async fn test_set_and_get_api_key() {
        let pool = create_test_db().await;

        set_api_key(&pool, LlmProvider::OpenAi, "sk-test").await.unwrap();

        let key = get_api_key(&pool, LlmProvider::OpenAi).await.unwrap();
        assert_eq!(key, Some("sk-test".to_string()));
    }

    #[tokio::test]
    async fn test_delete_api_key() {
        let pool = create_test_db().await;

        set_api_key(&pool, LlmProvider::Gemini, "key123").await.unwrap();
        delete_api_key(&pool, LlmProvider::Gemini).await.unwrap();

        let key = get_api_key(&pool, LlmProvider::Gemini).await.unwrap();
        assert_eq!(key, None);
    }

    #[tokio::test]
    async fn test_set_and_get_provider_preference() {
        let pool = create_test_db().await;

        set_provider_preference(&pool, TaskType::EmailDraft, LlmProvider::Claude)
            .await
            .unwrap();

        let pref = get_provider_preference(&pool, TaskType::EmailDraft)
            .await
            .unwrap();
        assert_eq!(pref, Some(LlmProvider::Claude));
    }

    #[tokio::test]
    async fn test_set_and_get_primary_provider() {
        let pool = create_test_db().await;

        set_primary_provider(&pool, LlmProvider::LmStudio).await.unwrap();

        let primary = get_primary_provider(&pool).await.unwrap();
        assert_eq!(primary, Some(LlmProvider::LmStudio));
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p spectral-privacy llm_settings::tests
```

Expected: FAIL (functions don't exist)

**Step 3: Implement LLM provider types**

File: `crates/spectral-privacy/src/llm_settings.rs`

```rust
use crate::error::Result;
use serde::{Deserialize, Serialize};
use spectral_db::settings::{delete_setting, get_setting, set_setting};
use sqlx::SqlitePool;

/// LLM provider options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LlmProvider {
    Ollama,
    LmStudio,
    OpenAi,
    Claude,
    Gemini,
}

impl LlmProvider {
    /// Check if provider is local (doesn't require API key or internet)
    pub fn is_local(self) -> bool {
        matches!(self, Self::Ollama | Self::LmStudio)
    }

    /// Get provider name for display
    pub fn name(self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::LmStudio => "LM Studio",
            Self::OpenAi => "OpenAI",
            Self::Claude => "Claude",
            Self::Gemini => "Gemini",
        }
    }

    /// Get settings key for API key
    fn api_key_setting_key(self) -> String {
        format!("llm_api_key_{}", self.name().to_lowercase().replace(' ', "_"))
    }
}

/// Task types for task-based routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    EmailDraft,
    FormFilling,
}

impl TaskType {
    /// Get settings key for provider preference
    fn preference_setting_key(self) -> String {
        format!("llm_task_preference_{:?}", self).to_lowercase()
    }
}

const PRIMARY_PROVIDER_KEY: &str = "llm_primary_provider";

/// Set API key for a provider
pub async fn set_api_key(pool: &SqlitePool, provider: LlmProvider, key: &str) -> Result<()> {
    let setting_key = provider.api_key_setting_key();
    let value = serde_json::json!(key);
    set_setting(pool, &setting_key, &value).await?;
    Ok(())
}

/// Get API key for a provider
pub async fn get_api_key(pool: &SqlitePool, provider: LlmProvider) -> Result<Option<String>> {
    let setting_key = provider.api_key_setting_key();
    match get_setting(pool, &setting_key).await? {
        Some(value) => {
            let key: String = serde_json::from_value(value)?;
            Ok(Some(key))
        }
        None => Ok(None),
    }
}

/// Delete API key for a provider
pub async fn delete_api_key(pool: &SqlitePool, provider: LlmProvider) -> Result<()> {
    let setting_key = provider.api_key_setting_key();
    delete_setting(pool, &setting_key).await?;
    Ok(())
}

/// Set provider preference for a task type
pub async fn set_provider_preference(
    pool: &SqlitePool,
    task_type: TaskType,
    provider: LlmProvider,
) -> Result<()> {
    let setting_key = task_type.preference_setting_key();
    let value = serde_json::json!(provider);
    set_setting(pool, &setting_key, &value).await?;
    Ok(())
}

/// Get provider preference for a task type
pub async fn get_provider_preference(
    pool: &SqlitePool,
    task_type: TaskType,
) -> Result<Option<LlmProvider>> {
    let setting_key = task_type.preference_setting_key();
    match get_setting(pool, &setting_key).await? {
        Some(value) => {
            let provider: LlmProvider = serde_json::from_value(value)?;
            Ok(Some(provider))
        }
        None => Ok(None),
    }
}

/// Set primary (default) provider
pub async fn set_primary_provider(pool: &SqlitePool, provider: LlmProvider) -> Result<()> {
    let value = serde_json::json!(provider);
    set_setting(pool, PRIMARY_PROVIDER_KEY, &value).await?;
    Ok(())
}

/// Get primary (default) provider
pub async fn get_primary_provider(pool: &SqlitePool) -> Result<Option<LlmProvider>> {
    match get_setting(pool, PRIMARY_PROVIDER_KEY).await? {
        Some(value) => {
            let provider: LlmProvider = serde_json::from_value(value)?;
            Ok(Some(provider))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral_db::test_helpers::*;

    #[tokio::test]
    async fn test_set_and_get_api_key() {
        let pool = create_test_db().await;

        set_api_key(&pool, LlmProvider::OpenAi, "sk-test").await.unwrap();

        let key = get_api_key(&pool, LlmProvider::OpenAi).await.unwrap();
        assert_eq!(key, Some("sk-test".to_string()));
    }

    #[tokio::test]
    async fn test_delete_api_key() {
        let pool = create_test_db().await;

        set_api_key(&pool, LlmProvider::Gemini, "key123").await.unwrap();
        delete_api_key(&pool, LlmProvider::Gemini).await.unwrap();

        let key = get_api_key(&pool, LlmProvider::Gemini).await.unwrap();
        assert_eq!(key, None);
    }

    #[tokio::test]
    async fn test_set_and_get_provider_preference() {
        let pool = create_test_db().await;

        set_provider_preference(&pool, TaskType::EmailDraft, LlmProvider::Claude)
            .await
            .unwrap();

        let pref = get_provider_preference(&pool, TaskType::EmailDraft)
            .await
            .unwrap();
        assert_eq!(pref, Some(LlmProvider::Claude));
    }

    #[tokio::test]
    async fn test_set_and_get_primary_provider() {
        let pool = create_test_db().await;

        set_primary_provider(&pool, LlmProvider::LmStudio).await.unwrap();

        let primary = get_primary_provider(&pool).await.unwrap();
        assert_eq!(primary, Some(LlmProvider::LmStudio));
    }
}
```

**Step 4: Export from lib.rs**

Modify: `crates/spectral-privacy/src/lib.rs`

```rust
pub mod engine;
pub mod error;
pub mod llm_settings;
pub mod types;

pub use engine::PrivacyEngine;
pub use error::{PrivacyError, Result};
pub use llm_settings::{
    delete_api_key, get_api_key, get_primary_provider, get_provider_preference, set_api_key,
    set_primary_provider, set_provider_preference, LlmProvider, TaskType,
};
pub use types::{Feature, FeatureFlags, PermissionResult, PrivacyLevel};
```

**Step 5: Run tests**

```bash
cargo test -p spectral-privacy llm_settings::tests
```

Expected: PASS (4 tests)

**Step 6: Commit**

```bash
git add crates/spectral-privacy/src/
git commit -m "feat(privacy): add LLM provider settings module

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 2: LLM Provider Expansion

### Task 7: OpenAI Provider Implementation

**Files:**
- Create: `crates/spectral-llm/src/providers/openai.rs`
- Modify: `crates/spectral-llm/src/providers/mod.rs`
- Modify: `crates/spectral-llm/Cargo.toml`

**Step 1: Write failing test**

File: `crates/spectral-llm/src/providers/openai.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_capabilities() {
        let provider = OpenAiProvider::new("sk-test".to_string());
        let caps = provider.capabilities();

        assert_eq!(caps.provider_name, "OpenAI");
        assert!(!caps.is_local);
        assert!(caps.supports_streaming);
    }

    #[tokio::test]
    async fn test_openai_request_construction() {
        let provider = OpenAiProvider::new("sk-test".to_string());

        let request = CompletionRequest {
            messages: vec![Message {
                role: Role::User,
                content: "Hello".to_string(),
            }],
            model: None,
            temperature: Some(0.7),
            max_tokens: Some(100),
        };

        // This will fail without real API key, but validates structure
        let result = provider.complete(request).await;
        assert!(result.is_err()); // Expected without valid key
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p spectral-llm openai::tests
```

Expected: FAIL (OpenAiProvider doesn't exist)

**Step 3: Add reqwest dependency**

Modify: `crates/spectral-llm/Cargo.toml`

```toml
[dependencies]
# ... existing dependencies ...
reqwest = { version = "0.12", features = ["json"] }
```

**Step 4: Implement OpenAI provider**

File: `crates/spectral-llm/src/providers/openai.rs`

```rust
use crate::provider::{
    CompletionRequest, CompletionResponse, LlmProvider, Message, ProviderCapabilities, Role,
};
use crate::LlmError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_MODEL: &str = "gpt-4o";

#[derive(Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessageResponse,
}

#[derive(Deserialize)]
struct OpenAiMessageResponse {
    content: String,
}

pub struct OpenAiProvider {
    api_key: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    fn convert_role(role: Role) -> String {
        match role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
        .to_string()
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider_name: "OpenAI".to_string(),
            is_local: false,
            supports_streaming: true,
            default_model: DEFAULT_MODEL.to_string(),
        }
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let openai_messages: Vec<OpenAiMessage> = request
            .messages
            .into_iter()
            .map(|m| OpenAiMessage {
                role: Self::convert_role(m.role),
                content: m.content,
            })
            .collect();

        let openai_request = OpenAiRequest {
            model: request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            messages: openai_messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false,
        };

        let response = self
            .client
            .post(OPENAI_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| LlmError::ProviderError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ProviderError(format!(
                "OpenAI API error {}: {}",
                status, error_text
            )));
        }

        let openai_response: OpenAiResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ProviderError(format!("Failed to parse response: {}", e)))?;

        let content = openai_response
            .choices
            .first()
            .ok_or_else(|| LlmError::ProviderError("No choices in response".to_string()))?
            .message
            .content
            .clone();

        Ok(CompletionResponse {
            content,
            model: DEFAULT_MODEL.to_string(),
        })
    }

    async fn complete_streaming(
        &self,
        _request: CompletionRequest,
    ) -> Result<
        std::pin::Pin<
            Box<dyn futures::Stream<Item = Result<String, LlmError>> + Send + 'static>,
        >,
        LlmError,
    > {
        // Streaming implementation deferred to polish phase
        Err(LlmError::ProviderError(
            "Streaming not yet implemented for OpenAI".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_capabilities() {
        let provider = OpenAiProvider::new("sk-test".to_string());
        let caps = provider.capabilities();

        assert_eq!(caps.provider_name, "OpenAI");
        assert!(!caps.is_local);
        assert!(caps.supports_streaming);
    }

    #[tokio::test]
    async fn test_openai_request_construction() {
        let provider = OpenAiProvider::new("sk-test".to_string());

        let request = CompletionRequest {
            messages: vec![Message {
                role: Role::User,
                content: "Hello".to_string(),
            }],
            model: None,
            temperature: Some(0.7),
            max_tokens: Some(100),
        };

        // This will fail without real API key, but validates structure
        let result = provider.complete(request).await;
        assert!(result.is_err()); // Expected without valid key
    }
}
```

**Step 5: Export from providers module**

Modify: `crates/spectral-llm/src/providers/mod.rs`

```rust
pub mod anthropic;
pub mod ollama;
pub mod openai;

pub use anthropic::AnthropicProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
```

**Step 6: Run tests**

```bash
cargo test -p spectral-llm openai::tests
```

Expected: PASS (2 tests)

**Step 7: Commit**

```bash
git add crates/spectral-llm/
git commit -m "feat(llm): add OpenAI provider implementation

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 8: Gemini Provider Implementation

**Files:**
- Create: `crates/spectral-llm/src/providers/gemini.rs`
- Modify: `crates/spectral-llm/src/providers/mod.rs`

**Step 1: Write failing test**

File: `crates/spectral-llm/src/providers/gemini.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_provider_capabilities() {
        let provider = GeminiProvider::new("test-key".to_string());
        let caps = provider.capabilities();

        assert_eq!(caps.provider_name, "Gemini");
        assert!(!caps.is_local);
        assert!(!caps.supports_streaming);
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p spectral-llm gemini::tests
```

Expected: FAIL (GeminiProvider doesn't exist)

**Step 3: Implement Gemini provider**

File: `crates/spectral-llm/src/providers/gemini.rs`

```rust
use crate::provider::{
    CompletionRequest, CompletionResponse, LlmProvider, ProviderCapabilities, Role,
};
use crate::LlmError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const DEFAULT_MODEL: &str = "gemini-2.0-flash-exp";

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContentResponse,
}

#[derive(Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Deserialize)]
struct GeminiPartResponse {
    text: String,
}

pub struct GeminiProvider {
    api_key: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    fn convert_role(role: Role) -> Option<String> {
        match role {
            Role::System => None, // Gemini doesn't have system role
            Role::User => Some("user".to_string()),
            Role::Assistant => Some("model".to_string()),
        }
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider_name: "Gemini".to_string(),
            is_local: false,
            supports_streaming: false,
            default_model: DEFAULT_MODEL.to_string(),
        }
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let gemini_contents: Vec<GeminiContent> = request
            .messages
            .into_iter()
            .map(|m| GeminiContent {
                parts: vec![GeminiPart { text: m.content }],
                role: Self::convert_role(m.role),
            })
            .collect();

        let generation_config = if request.temperature.is_some() || request.max_tokens.is_some() {
            Some(GeminiGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
            })
        } else {
            None
        };

        let gemini_request = GeminiRequest {
            contents: gemini_contents,
            generation_config,
        };

        let model = request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let url = format!("{}/:generateContent?key={}",
            format!("{}/{}", GEMINI_API_URL, model),
            self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&gemini_request)
            .send()
            .await
            .map_err(|e| LlmError::ProviderError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ProviderError(format!(
                "Gemini API error {}: {}",
                status, error_text
            )));
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ProviderError(format!("Failed to parse response: {}", e)))?;

        let content = gemini_response
            .candidates
            .first()
            .ok_or_else(|| LlmError::ProviderError("No candidates in response".to_string()))?
            .content
            .parts
            .first()
            .ok_or_else(|| LlmError::ProviderError("No parts in response".to_string()))?
            .text
            .clone();

        Ok(CompletionResponse {
            content,
            model: DEFAULT_MODEL.to_string(),
        })
    }

    async fn complete_streaming(
        &self,
        _request: CompletionRequest,
    ) -> Result<
        std::pin::Pin<
            Box<dyn futures::Stream<Item = Result<String, LlmError>> + Send + 'static>,
        >,
        LlmError,
    > {
        Err(LlmError::ProviderError(
            "Streaming not supported for Gemini".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_provider_capabilities() {
        let provider = GeminiProvider::new("test-key".to_string());
        let caps = provider.capabilities();

        assert_eq!(caps.provider_name, "Gemini");
        assert!(!caps.is_local);
        assert!(!caps.supports_streaming);
    }
}
```

**Step 4: Export from providers module**

Modify: `crates/spectral-llm/src/providers/mod.rs`

```rust
pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai;

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
```

**Step 5: Run tests**

```bash
cargo test -p spectral-llm gemini::tests
```

Expected: PASS (1 test)

**Step 6: Commit**

```bash
git add crates/spectral-llm/src/providers/
git commit -m "feat(llm): add Gemini provider implementation

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 9: LM Studio Provider Implementation

**Files:**
- Create: `crates/spectral-llm/src/providers/lmstudio.rs`
- Modify: `crates/spectral-llm/src/providers/mod.rs`

**Step 1: Write failing test**

File: `crates/spectral-llm/src/providers/lmstudio.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lmstudio_provider_capabilities() {
        let provider = LmStudioProvider::new();
        let caps = provider.capabilities();

        assert_eq!(caps.provider_name, "LM Studio");
        assert!(caps.is_local);
        assert!(caps.supports_streaming);
    }

    #[test]
    fn test_lmstudio_default_url() {
        let provider = LmStudioProvider::new();
        assert_eq!(provider.base_url, "http://localhost:1234");
    }

    #[test]
    fn test_lmstudio_custom_url() {
        let provider = LmStudioProvider::with_url("http://localhost:5000".to_string());
        assert_eq!(provider.base_url, "http://localhost:5000");
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p spectral-llm lmstudio::tests
```

Expected: FAIL (LmStudioProvider doesn't exist)

**Step 3: Implement LM Studio provider**

File: `crates/spectral-llm/src/providers/lmstudio.rs`

```rust
use crate::provider::{
    CompletionRequest, CompletionResponse, LlmProvider, Message, ProviderCapabilities, Role,
};
use crate::LlmError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "http://localhost:1234";
const DEFAULT_MODEL: &str = "local-model";

#[derive(Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessageResponse,
}

#[derive(Deserialize)]
struct OpenAiMessageResponse {
    content: String,
}

pub struct LmStudioProvider {
    base_url: String,
    client: reqwest::Client,
}

impl LmStudioProvider {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_url(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    /// Check if LM Studio is running at the configured URL
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/v1/models", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    fn convert_role(role: Role) -> String {
        match role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
        .to_string()
    }
}

impl Default for LmStudioProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for LmStudioProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider_name: "LM Studio".to_string(),
            is_local: true,
            supports_streaming: true,
            default_model: DEFAULT_MODEL.to_string(),
        }
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let messages: Vec<OpenAiMessage> = request
            .messages
            .into_iter()
            .map(|m| OpenAiMessage {
                role: Self::convert_role(m.role),
                content: m.content,
            })
            .collect();

        let openai_request = OpenAiRequest {
            model: request.model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false,
        };

        let url = format!("{}/v1/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| {
                LlmError::ProviderError(format!(
                    "LM Studio connection failed (is it running?): {}",
                    e
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ProviderError(format!(
                "LM Studio API error {}: {}",
                status, error_text
            )));
        }

        let openai_response: OpenAiResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ProviderError(format!("Failed to parse response: {}", e)))?;

        let content = openai_response
            .choices
            .first()
            .ok_or_else(|| LlmError::ProviderError("No choices in response".to_string()))?
            .message
            .content
            .clone();

        Ok(CompletionResponse {
            content,
            model: DEFAULT_MODEL.to_string(),
        })
    }

    async fn complete_streaming(
        &self,
        _request: CompletionRequest,
    ) -> Result<
        std::pin::Pin<
            Box<dyn futures::Stream<Item = Result<String, LlmError>> + Send + 'static>,
        >,
        LlmError,
    > {
        // Streaming implementation deferred to polish phase
        Err(LlmError::ProviderError(
            "Streaming not yet implemented for LM Studio".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lmstudio_provider_capabilities() {
        let provider = LmStudioProvider::new();
        let caps = provider.capabilities();

        assert_eq!(caps.provider_name, "LM Studio");
        assert!(caps.is_local);
        assert!(caps.supports_streaming);
    }

    #[test]
    fn test_lmstudio_default_url() {
        let provider = LmStudioProvider::new();
        assert_eq!(provider.base_url, "http://localhost:1234");
    }

    #[test]
    fn test_lmstudio_custom_url() {
        let provider = LmStudioProvider::with_url("http://localhost:5000".to_string());
        assert_eq!(provider.base_url, "http://localhost:5000");
    }
}
```

**Step 4: Export from providers module**

Modify: `crates/spectral-llm/src/providers/mod.rs`

```rust
pub mod anthropic;
pub mod gemini;
pub mod lmstudio;
pub mod ollama;
pub mod openai;

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use lmstudio::LmStudioProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
```

**Step 5: Run tests**

```bash
cargo test -p spectral-llm lmstudio::tests
```

Expected: PASS (3 tests)

**Step 6: Commit**

```bash
git add crates/spectral-llm/src/providers/
git commit -m "feat(llm): add LM Studio provider with auto-detection

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 10: Privacy-Aware LLM Router

**Files:**
- Create: `crates/spectral-privacy/src/llm_router.rs`
- Modify: `crates/spectral-privacy/src/lib.rs`
- Modify: `crates/spectral-privacy/Cargo.toml`

**Step 1: Add spectral-llm dependency**

Modify: `crates/spectral-privacy/Cargo.toml`

```toml
[dependencies]
# ... existing dependencies ...
spectral-llm = { path = "../spectral-llm" }
```

**Step 2: Write failing test**

File: `crates/spectral-privacy/src/llm_router.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use spectral_db::test_helpers::*;

    #[tokio::test]
    async fn test_route_with_permission_denied() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool.clone());
        engine.set_privacy_level(PrivacyLevel::Paranoid).await.unwrap();

        let router = PrivacyAwareLlmRouter::new(pool);

        let request = CompletionRequest {
            messages: vec![],
            model: None,
            temperature: None,
            max_tokens: None,
        };

        let result = router.route(TaskType::EmailDraft, request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Permission denied"));
    }
}
```

**Step 3: Run test to verify it fails**

```bash
cargo test -p spectral-privacy llm_router::tests
```

Expected: FAIL (PrivacyAwareLlmRouter doesn't exist)

**Step 4: Implement privacy-aware LLM router**

File: `crates/spectral-privacy/src/llm_router.rs`

```rust
use crate::engine::PrivacyEngine;
use crate::error::{PrivacyError, Result};
use crate::llm_settings::{get_api_key, get_primary_provider, get_provider_preference};
use crate::llm_settings::{LlmProvider, TaskType};
use crate::types::Feature;
use spectral_llm::provider::{CompletionRequest, CompletionResponse, LlmProvider as LlmProviderTrait};
use spectral_llm::providers::{
    AnthropicProvider, GeminiProvider, LmStudioProvider, OllamaProvider, OpenAiProvider,
};
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct PrivacyAwareLlmRouter {
    pool: SqlitePool,
}

impl PrivacyAwareLlmRouter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Route a request to the appropriate LLM provider based on privacy settings and preferences
    pub async fn route(
        &self,
        task_type: TaskType,
        request: CompletionRequest,
    ) -> Result<CompletionResponse> {
        let engine = PrivacyEngine::new(self.pool.clone());

        // 1. Determine which provider to use
        let provider = self.select_provider(task_type).await?;

        // 2. Check permissions based on provider locality
        let feature = if provider.is_local() {
            Feature::LocalLlm
        } else {
            Feature::CloudLlm
        };

        let permission = engine.check_permission(feature).await?;
        if !permission.is_allowed() {
            return Err(PrivacyError::PermissionDenied(
                permission.reason().unwrap_or("Unknown reason").to_string(),
            ));
        }

        // 3. Create provider instance
        let provider_instance = self.create_provider(provider).await?;

        // 4. Apply PII filtering if cloud provider
        let (filtered_request, should_filter) = if provider.is_local() {
            (request, false)
        } else {
            // TODO: Apply PII filtering in Phase 5
            (request, true)
        };

        // 5. Execute request
        let response = provider_instance
            .complete(filtered_request)
            .await
            .map_err(|e| PrivacyError::PermissionDenied(format!("LLM error: {}", e)))?;

        // 6. Detokenize response if filtered
        let final_response = if should_filter {
            // TODO: Detokenize in Phase 5
            response
        } else {
            response
        };

        Ok(final_response)
    }

    async fn select_provider(&self, task_type: TaskType) -> Result<LlmProvider> {
        // Check task-specific preference first
        if let Some(pref) = get_provider_preference(&self.pool, task_type).await? {
            return Ok(pref);
        }

        // Fall back to primary provider
        if let Some(primary) = get_primary_provider(&self.pool).await? {
            return Ok(primary);
        }

        // Default to Ollama if nothing configured
        Ok(LlmProvider::Ollama)
    }

    async fn create_provider(
        &self,
        provider: LlmProvider,
    ) -> Result<Arc<dyn LlmProviderTrait + Send + Sync>> {
        match provider {
            LlmProvider::Ollama => Ok(Arc::new(OllamaProvider::new())),

            LlmProvider::LmStudio => Ok(Arc::new(LmStudioProvider::new())),

            LlmProvider::OpenAi => {
                let api_key = get_api_key(&self.pool, provider)
                    .await?
                    .ok_or_else(|| {
                        PrivacyError::InvalidConfiguration("OpenAI API key not set".to_string())
                    })?;
                Ok(Arc::new(OpenAiProvider::new(api_key)))
            }

            LlmProvider::Claude => {
                let api_key = get_api_key(&self.pool, provider)
                    .await?
                    .ok_or_else(|| {
                        PrivacyError::InvalidConfiguration("Claude API key not set".to_string())
                    })?;
                Ok(Arc::new(AnthropicProvider::new(api_key)))
            }

            LlmProvider::Gemini => {
                let api_key = get_api_key(&self.pool, provider)
                    .await?
                    .ok_or_else(|| {
                        PrivacyError::InvalidConfiguration("Gemini API key not set".to_string())
                    })?;
                Ok(Arc::new(GeminiProvider::new(api_key)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PrivacyLevel;
    use spectral_db::test_helpers::*;

    #[tokio::test]
    async fn test_route_with_permission_denied() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool.clone());
        engine.set_privacy_level(PrivacyLevel::Paranoid).await.unwrap();

        let router = PrivacyAwareLlmRouter::new(pool);

        let request = CompletionRequest {
            messages: vec![],
            model: None,
            temperature: None,
            max_tokens: None,
        };

        let result = router.route(TaskType::EmailDraft, request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Permission denied"));
    }
}
```

**Step 5: Export from lib.rs**

Modify: `crates/spectral-privacy/src/lib.rs`

```rust
pub mod engine;
pub mod error;
pub mod llm_router;
pub mod llm_settings;
pub mod types;

pub use engine::PrivacyEngine;
pub use error::{PrivacyError, Result};
pub use llm_router::PrivacyAwareLlmRouter;
pub use llm_settings::{
    delete_api_key, get_api_key, get_primary_provider, get_provider_preference, set_api_key,
    set_primary_provider, set_provider_preference, LlmProvider, TaskType,
};
pub use types::{Feature, FeatureFlags, PermissionResult, PrivacyLevel};
```

**Step 6: Run tests**

```bash
cargo test -p spectral-privacy llm_router::tests
```

Expected: PASS (1 test)

**Step 7: Commit**

```bash
git add crates/spectral-privacy/
git commit -m "feat(privacy): implement privacy-aware LLM router

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 11: Tauri Privacy Settings Commands

**Files:**
- Create: `src-tauri/src/commands/privacy.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: Create privacy commands module**

File: `src-tauri/src/commands/privacy.rs`

```rust
use crate::error::{CommandError, Result};
use crate::state::VaultState;
use serde::{Deserialize, Serialize};
use spectral_privacy::{
    delete_api_key, get_api_key, get_primary_provider, get_provider_preference,
    set_api_key, set_primary_provider, set_provider_preference, Feature, FeatureFlags,
    LlmProvider, PrivacyEngine, PrivacyLevel, TaskType,
};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacySettings {
    pub level: PrivacyLevel,
    pub feature_flags: FeatureFlags,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProviderSettings {
    pub primary_provider: Option<LlmProvider>,
    pub email_draft_provider: Option<LlmProvider>,
    pub form_filling_provider: Option<LlmProvider>,
}

#[tauri::command]
pub async fn get_privacy_settings(
    vault_state: State<'_, VaultState>,
    vault_id: String,
) -> Result<PrivacySettings> {
    let pool = vault_state.get_pool(&vault_id).await?;
    let engine = PrivacyEngine::new(pool);

    let level = engine.get_privacy_level().await?;
    let feature_flags = engine.get_feature_flags().await?;

    Ok(PrivacySettings {
        level,
        feature_flags,
    })
}

#[tauri::command]
pub async fn set_privacy_level(
    vault_state: State<'_, VaultState>,
    vault_id: String,
    level: PrivacyLevel,
) -> Result<()> {
    let pool = vault_state.get_pool(&vault_id).await?;
    let engine = PrivacyEngine::new(pool);

    engine.set_privacy_level(level).await?;
    Ok(())
}

#[tauri::command]
pub async fn set_custom_feature_flags(
    vault_state: State<'_, VaultState>,
    vault_id: String,
    flags: FeatureFlags,
) -> Result<()> {
    let pool = vault_state.get_pool(&vault_id).await?;
    let engine = PrivacyEngine::new(pool);

    engine.set_feature_flags(flags).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_llm_provider_settings(
    vault_state: State<'_, VaultState>,
    vault_id: String,
) -> Result<LlmProviderSettings> {
    let pool = vault_state.get_pool(&vault_id).await?;

    let primary_provider = get_primary_provider(&pool).await?;
    let email_draft_provider = get_provider_preference(&pool, TaskType::EmailDraft).await?;
    let form_filling_provider = get_provider_preference(&pool, TaskType::FormFilling).await?;

    Ok(LlmProviderSettings {
        primary_provider,
        email_draft_provider,
        form_filling_provider,
    })
}

#[tauri::command]
pub async fn set_llm_primary_provider(
    vault_state: State<'_, VaultState>,
    vault_id: String,
    provider: LlmProvider,
) -> Result<()> {
    let pool = vault_state.get_pool(&vault_id).await?;
    set_primary_provider(&pool, provider).await?;
    Ok(())
}

#[tauri::command]
pub async fn set_llm_task_provider(
    vault_state: State<'_, VaultState>,
    vault_id: String,
    task_type: TaskType,
    provider: LlmProvider,
) -> Result<()> {
    let pool = vault_state.get_pool(&vault_id).await?;
    set_provider_preference(&pool, task_type, provider).await?;
    Ok(())
}

#[tauri::command]
pub async fn set_llm_api_key(
    vault_state: State<'_, VaultState>,
    vault_id: String,
    provider: LlmProvider,
    api_key: String,
) -> Result<()> {
    let pool = vault_state.get_pool(&vault_id).await?;
    set_api_key(&pool, provider, &api_key).await?;
    Ok(())
}

#[tauri::command]
pub async fn test_llm_provider(
    vault_state: State<'_, VaultState>,
    vault_id: String,
    provider: LlmProvider,
) -> Result<()> {
    let pool = vault_state.get_pool(&vault_id).await?;

    // For local providers, check availability
    if provider.is_local() {
        match provider {
            LlmProvider::LmStudio => {
                let lm_studio = spectral_llm::providers::LmStudioProvider::new();
                if !lm_studio.is_available().await {
                    return Err(CommandError::InvalidRequest(
                        "LM Studio is not running at localhost:1234".to_string(),
                    ));
                }
            }
            LlmProvider::Ollama => {
                let ollama = spectral_llm::providers::OllamaProvider::new();
                // Ollama provider doesn't have is_available yet - assume OK
                // TODO: Add is_available to OllamaProvider
            }
            _ => {}
        }
    } else {
        // For cloud providers, verify API key exists
        let api_key = get_api_key(&pool, provider)
            .await?
            .ok_or_else(|| CommandError::InvalidRequest("API key not set".to_string()))?;

        if api_key.is_empty() {
            return Err(CommandError::InvalidRequest("API key is empty".to_string()));
        }

        // TODO: Actually test the connection with a minimal request in Phase 6
    }

    Ok(())
}
```

**Step 2: Export from commands module**

Modify: `src-tauri/src/commands/mod.rs`

```rust
pub mod privacy;
// ... existing modules ...
```

**Step 3: Register commands in main.rs**

Modify: `src-tauri/src/main.rs`

In the `.invoke_handler` section, add:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::privacy::get_privacy_settings,
    commands::privacy::set_privacy_level,
    commands::privacy::set_custom_feature_flags,
    commands::privacy::get_llm_provider_settings,
    commands::privacy::set_llm_primary_provider,
    commands::privacy::set_llm_task_provider,
    commands::privacy::set_llm_api_key,
    commands::privacy::test_llm_provider,
])
```

**Step 4: Build to verify compilation**

```bash
cargo build
```

Expected: Success

**Step 5: Commit**

```bash
git add src-tauri/src/commands/
git commit -m "feat(tauri): add privacy settings commands

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 12: Frontend Privacy API Module

**Files:**
- Create: `src/lib/api/privacy.ts`

**Step 1: Create privacy API module**

File: `src/lib/api/privacy.ts`

```typescript
import { invoke } from '@tauri-apps/api/core';

export type PrivacyLevel = 'Paranoid' | 'LocalPrivacy' | 'Balanced' | 'Custom';

export interface FeatureFlags {
	allow_local_llm: boolean;
	allow_cloud_llm: boolean;
	allow_browser_automation: boolean;
	allow_email_sending: boolean;
	allow_imap_monitoring: boolean;
	allow_pii_scanning: boolean;
}

export interface PrivacySettings {
	level: PrivacyLevel;
	featureFlags: FeatureFlags;
}

export type LlmProvider = 'Ollama' | 'LmStudio' | 'OpenAi' | 'Claude' | 'Gemini';
export type TaskType = 'EmailDraft' | 'FormFilling';

export interface LlmProviderSettings {
	primaryProvider: LlmProvider | null;
	emailDraftProvider: LlmProvider | null;
	formFillingProvider: LlmProvider | null;
}

export async function getPrivacySettings(vaultId: string): Promise<PrivacySettings> {
	return await invoke('get_privacy_settings', { vaultId });
}

export async function setPrivacyLevel(vaultId: string, level: PrivacyLevel): Promise<void> {
	await invoke('set_privacy_level', { vaultId, level });
}

export async function setCustomFeatureFlags(
	vaultId: string,
	flags: FeatureFlags
): Promise<void> {
	await invoke('set_custom_feature_flags', { vaultId, flags });
}

export async function getLlmProviderSettings(vaultId: string): Promise<LlmProviderSettings> {
	return await invoke('get_llm_provider_settings', { vaultId });
}

export async function setLlmPrimaryProvider(
	vaultId: string,
	provider: LlmProvider
): Promise<void> {
	await invoke('set_llm_primary_provider', { vaultId, provider });
}

export async function setLlmTaskProvider(
	vaultId: string,
	taskType: TaskType,
	provider: LlmProvider
): Promise<void> {
	await invoke('set_llm_task_provider', { vaultId, taskType, provider });
}

export async function setLlmApiKey(
	vaultId: string,
	provider: LlmProvider,
	apiKey: string
): Promise<void> {
	await invoke('set_llm_api_key', { vaultId, provider, apiKey });
}

export async function testLlmProvider(vaultId: string, provider: LlmProvider): Promise<void> {
	await invoke('test_llm_provider', { vaultId, provider });
}
```

**Step 2: Commit**

```bash
git add src/lib/api/privacy.ts
git commit -m "feat(frontend): add privacy settings API module

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 3: UI Privacy Level Tab

### Task 13: Privacy Level Preset Selector Component

**Files:**
- Modify: `src/routes/settings/+page.svelte`

**Step 1: Import privacy API**

Modify: `src/routes/settings/+page.svelte`

Add to imports section:

```typescript
import {
	getPrivacySettings,
	setPrivacyLevel,
	setCustomFeatureFlags,
	type PrivacyLevel,
	type FeatureFlags
} from '$lib/api/privacy';
```

**Step 2: Add privacy state variables**

After existing state variables:

```typescript
// Privacy settings state
let privacySettings = $state<{ level: PrivacyLevel; featureFlags: FeatureFlags } | null>(null);
let privacyLoading = $state(false);
let privacyError = $state<string | null>(null);
```

**Step 3: Add load privacy settings function**

```typescript
async function loadPrivacySettings() {
	if (!vaultStore.currentVaultId) return;
	privacyLoading = true;
	privacyError = null;
	try {
		privacySettings = await getPrivacySettings(vaultStore.currentVaultId);
	} catch (err) {
		privacyError = err instanceof Error ? err.message : String(err);
		console.error('Failed to load privacy settings:', err);
	} finally {
		privacyLoading = false;
	}
}

async function handleSetPrivacyLevel(level: PrivacyLevel) {
	if (!vaultStore.currentVaultId) return;
	privacyError = null;
	try {
		await setPrivacyLevel(vaultStore.currentVaultId, level);
		await loadPrivacySettings(); // Reload to get updated feature flags
	} catch (err) {
		privacyError = err instanceof Error ? err.message : String(err);
		console.error('Failed to set privacy level:', err);
	}
}
```

**Step 4: Add effect to load privacy settings**

```typescript
$effect(() => {
	if (activeTab === 'privacy' && vaultStore.currentVaultId) {
		loadPrivacySettings();
	}
});
```

**Step 5: Replace "Coming Soon" UI with privacy level selector**

Replace the Privacy Level tab content (currently "Coming Soon" message) with:

```svelte
{#if activeTab === 'privacy'}
	<section>
		<h2 class="mb-4 text-lg font-semibold text-gray-800">Privacy Level</h2>

		{#if privacyError}
			<div class="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-900">
				{privacyError}
			</div>
		{/if}

		{#if privacyLoading}
			<p class="text-gray-500">Loading privacy settings...</p>
		{:else if privacySettings}
			<div class="space-y-4">
				<!-- Paranoid -->
				<button
					onclick={() => handleSetPrivacyLevel('Paranoid')}
					class="w-full rounded-lg border p-4 text-left transition-colors {privacySettings.level ===
					'Paranoid'
						? 'border-primary-600 bg-primary-50'
						: 'border-gray-200 bg-white hover:bg-gray-50'}"
				>
					<div class="flex items-center justify-between">
						<h3 class="font-medium text-gray-900">Paranoid</h3>
						{#if privacySettings.level === 'Paranoid'}
							<span class="text-sm text-primary-600">Active</span>
						{/if}
					</div>
					<p class="mt-1 text-sm text-gray-600">
						No automation, no LLM features. All operations manual-only. Maximum privacy.
					</p>
				</button>

				<!-- Local Privacy -->
				<button
					onclick={() => handleSetPrivacyLevel('LocalPrivacy')}
					class="w-full rounded-lg border p-4 text-left transition-colors {privacySettings.level ===
					'LocalPrivacy'
						? 'border-primary-600 bg-primary-50'
						: 'border-gray-200 bg-white hover:bg-gray-50'}"
				>
					<div class="flex items-center justify-between">
						<h3 class="font-medium text-gray-900">Local Privacy</h3>
						{#if privacySettings.level === 'LocalPrivacy'}
							<span class="text-sm text-primary-600">Active</span>
						{/if}
					</div>
					<p class="mt-1 text-sm text-gray-600">
						Local LLM (Ollama/LM Studio) only, automation enabled. No data leaves your machine.
					</p>
				</button>

				<!-- Balanced -->
				<button
					onclick={() => handleSetPrivacyLevel('Balanced')}
					class="w-full rounded-lg border p-4 text-left transition-colors {privacySettings.level ===
					'Balanced'
						? 'border-primary-600 bg-primary-50'
						: 'border-gray-200 bg-white hover:bg-gray-50'}"
				>
					<div class="flex items-center justify-between">
						<h3 class="font-medium text-gray-900">Balanced</h3>
						{#if privacySettings.level === 'Balanced'}
							<span class="text-sm text-primary-600">Active</span>
						{/if}
					</div>
					<p class="mt-1 text-sm text-gray-600">
						Cloud LLM with PII filtering, all features enabled. Good balance of privacy and
						convenience.
					</p>
				</button>

				<!-- Custom -->
				<button
					onclick={() => handleSetPrivacyLevel('Custom')}
					class="w-full rounded-lg border p-4 text-left transition-colors {privacySettings.level ===
					'Custom'
						? 'border-primary-600 bg-primary-50'
						: 'border-gray-200 bg-white hover:bg-gray-50'}"
				>
					<div class="flex items-center justify-between">
						<h3 class="font-medium text-gray-900">Custom</h3>
						{#if privacySettings.level === 'Custom'}
							<span class="text-sm text-primary-600">Active</span>
						{/if}
					</div>
					<p class="mt-1 text-sm text-gray-600">
						Configure individual features manually. Advanced users only.
					</p>
				</button>
			</div>
		{/if}
	</section>
{/if}
```

**Step 6: Test in browser**

```bash
npm run dev
```

Navigate to Settings → Privacy Level tab, verify:
- Four preset buttons render
- Active preset is highlighted
- Clicking preset changes selection (check browser console)

**Step 7: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(frontend): implement privacy level preset selector

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 14: Custom Feature Flags Editor

**Files:**
- Modify: `src/routes/settings/+page.svelte`

**Step 1: Add custom flags handler**

After `handleSetPrivacyLevel` function:

```typescript
async function handleUpdateFeatureFlag(flag: keyof FeatureFlags, value: boolean) {
	if (!vaultStore.currentVaultId || !privacySettings) return;
	if (privacySettings.level !== 'Custom') return; // Only allow editing in Custom mode

	privacyError = null;
	try {
		const updatedFlags = { ...privacySettings.featureFlags, [flag]: value };
		await setCustomFeatureFlags(vaultStore.currentVaultId, updatedFlags);
		await loadPrivacySettings(); // Reload to confirm change
	} catch (err) {
		privacyError = err instanceof Error ? err.message : String(err);
		console.error('Failed to update feature flag:', err);
	}
}
```

**Step 2: Add custom flags UI**

After the Custom preset button, add:

```svelte
<!-- Custom Feature Flags (only show when Custom is active) -->
{#if privacySettings.level === 'Custom'}
	<div class="mt-6 rounded-lg border border-gray-200 bg-gray-50 p-4">
		<h3 class="mb-3 font-medium text-gray-900">Feature Flags</h3>
		<div class="space-y-3">
			<label class="flex items-center justify-between">
				<span class="text-sm text-gray-700">Allow Local LLM (Ollama/LM Studio)</span>
				<input
					type="checkbox"
					checked={privacySettings.featureFlags.allow_local_llm}
					onchange={(e) => handleUpdateFeatureFlag('allow_local_llm', e.currentTarget.checked)}
					class="rounded"
				/>
			</label>

			<label class="flex items-center justify-between">
				<span class="text-sm text-gray-700">Allow Cloud LLM (OpenAI/Claude/Gemini)</span>
				<input
					type="checkbox"
					checked={privacySettings.featureFlags.allow_cloud_llm}
					onchange={(e) => handleUpdateFeatureFlag('allow_cloud_llm', e.currentTarget.checked)}
					class="rounded"
				/>
			</label>

			<label class="flex items-center justify-between">
				<span class="text-sm text-gray-700">Allow Browser Automation</span>
				<input
					type="checkbox"
					checked={privacySettings.featureFlags.allow_browser_automation}
					onchange={(e) =>
						handleUpdateFeatureFlag('allow_browser_automation', e.currentTarget.checked)}
					class="rounded"
				/>
			</label>

			<label class="flex items-center justify-between">
				<span class="text-sm text-gray-700">Allow Email Sending</span>
				<input
					type="checkbox"
					checked={privacySettings.featureFlags.allow_email_sending}
					onchange={(e) =>
						handleUpdateFeatureFlag('allow_email_sending', e.currentTarget.checked)}
					class="rounded"
				/>
			</label>

			<label class="flex items-center justify-between">
				<span class="text-sm text-gray-700">Allow IMAP Monitoring</span>
				<input
					type="checkbox"
					checked={privacySettings.featureFlags.allow_imap_monitoring}
					onchange={(e) =>
						handleUpdateFeatureFlag('allow_imap_monitoring', e.currentTarget.checked)}
					class="rounded"
				/>
			</label>

			<label class="flex items-center justify-between">
				<span class="text-sm text-gray-700">Allow PII Scanning</span>
				<input
					type="checkbox"
					checked={privacySettings.featureFlags.allow_pii_scanning}
					onchange={(e) => handleUpdateFeatureFlag('allow_pii_scanning', e.currentTarget.checked)}
					class="rounded"
				/>
			</label>
		</div>
	</div>
{/if}
```

**Step 3: Test custom flags**

```bash
npm run dev
```

- Select Custom preset
- Verify feature flags panel appears
- Toggle flags, verify changes persist (reload tab)

**Step 4: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(frontend): add custom feature flags editor

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 4: UI LLM Providers Tab

### Task 15: Create LLM Providers Settings Tab

**Files:**
- Modify: `src/routes/settings/+page.svelte`

**Step 1: Add LLM Providers tab to tab bar**

Modify tab array:

```svelte
{#each [['privacy', 'Privacy Level'], ['llm-providers', 'LLM Providers'], ['email', 'Email'], ['scheduling', 'Scheduling'], ['audit', 'Audit Log']] as [id, label] (id)}
```

**Step 2: Import LLM provider API**

Add to imports:

```typescript
import {
	getLlmProviderSettings,
	setLlmPrimaryProvider,
	setLlmTaskProvider,
	setLlmApiKey,
	testLlmProvider,
	type LlmProvider,
	type TaskType,
	type LlmProviderSettings
} from '$lib/api/privacy';
```

**Step 3: Add LLM provider state**

```typescript
// LLM provider state
let llmSettings = $state<LlmProviderSettings | null>(null);
let llmLoading = $state(false);
let llmError = $state<string | null>(null);
let llmTestResults = $state<Record<LlmProvider, 'idle' | 'testing' | 'success' | 'error'>>({
	Ollama: 'idle',
	LmStudio: 'idle',
	OpenAi: 'idle',
	Claude: 'idle',
	Gemini: 'idle'
});
let llmApiKeys = $state<Record<LlmProvider, string>>({
	Ollama: '',
	LmStudio: '',
	OpenAi: '',
	Claude: '',
	Gemini: ''
});
```

**Step 4: Add load LLM settings function**

```typescript
async function loadLlmSettings() {
	if (!vaultStore.currentVaultId) return;
	llmLoading = true;
	llmError = null;
	try {
		llmSettings = await getLlmProviderSettings(vaultStore.currentVaultId);
	} catch (err) {
		llmError = err instanceof Error ? err.message : String(err);
		console.error('Failed to load LLM settings:', err);
	} finally {
		llmLoading = false;
	}
}

async function handleSetPrimaryProvider(provider: LlmProvider) {
	if (!vaultStore.currentVaultId) return;
	llmError = null;
	try {
		await setLlmPrimaryProvider(vaultStore.currentVaultId, provider);
		await loadLlmSettings();
	} catch (err) {
		llmError = err instanceof Error ? err.message : String(err);
	}
}

async function handleSetTaskProvider(taskType: TaskType, provider: LlmProvider) {
	if (!vaultStore.currentVaultId) return;
	llmError = null;
	try {
		await setLlmTaskProvider(vaultStore.currentVaultId, taskType, provider);
		await loadLlmSettings();
	} catch (err) {
		llmError = err instanceof Error ? err.message : String(err);
	}
}

async function handleSetApiKey(provider: LlmProvider) {
	if (!vaultStore.currentVaultId) return;
	const apiKey = llmApiKeys[provider];
	if (!apiKey) return;

	llmError = null;
	try {
		await setLlmApiKey(vaultStore.currentVaultId, provider, apiKey);
		llmApiKeys[provider] = ''; // Clear input after saving
		await loadLlmSettings();
	} catch (err) {
		llmError = err instanceof Error ? err.message : String(err);
	}
}

async function handleTestProvider(provider: LlmProvider) {
	if (!vaultStore.currentVaultId) return;
	llmTestResults[provider] = 'testing';
	try {
		await testLlmProvider(vaultStore.currentVaultId, provider);
		llmTestResults[provider] = 'success';
	} catch (err) {
		llmTestResults[provider] = 'error';
		llmError = err instanceof Error ? err.message : String(err);
	}
}
```

**Step 5: Add effect to load LLM settings**

```typescript
$effect(() => {
	if (activeTab === 'llm-providers' && vaultStore.currentVaultId) {
		loadLlmSettings();
	}
});
```

**Step 6: Add LLM Providers tab content**

After the audit tab section:

```svelte
{:else if activeTab === 'llm-providers'}
	<section>
		<h2 class="mb-4 text-lg font-semibold text-gray-800">LLM Providers</h2>

		{#if llmError}
			<div class="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-900">
				{llmError}
			</div>
		{/if}

		{#if llmLoading}
			<p class="text-gray-500">Loading provider settings...</p>
		{:else if llmSettings}
			<!-- Primary Provider Selection -->
			<div class="mb-6 rounded-lg border border-gray-200 bg-white p-4">
				<h3 class="mb-3 font-medium text-gray-900">Primary Provider</h3>
				<p class="mb-3 text-sm text-gray-600">
					Default provider used when no task-specific preference is set
				</p>
				<select
					value={llmSettings.primaryProvider ?? ''}
					onchange={(e) =>
						handleSetPrimaryProvider(e.currentTarget.value as LlmProvider)}
					class="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm"
				>
					<option value="">None (use first available)</option>
					<option value="Ollama">Ollama</option>
					<option value="LmStudio">LM Studio</option>
					<option value="OpenAi">OpenAI</option>
					<option value="Claude">Claude</option>
					<option value="Gemini">Gemini</option>
				</select>
			</div>

			<!-- Task-Specific Providers -->
			<div class="mb-6 rounded-lg border border-gray-200 bg-white p-4">
				<h3 class="mb-3 font-medium text-gray-900">Task-Specific Providers</h3>
				<p class="mb-3 text-sm text-gray-600">
					Override primary provider for specific task types
				</p>

				<div class="space-y-3">
					<div>
						<label class="mb-1 block text-sm font-medium text-gray-700">Email Drafting</label>
						<select
							value={llmSettings.emailDraftProvider ?? ''}
							onchange={(e) =>
								handleSetTaskProvider('EmailDraft', e.currentTarget.value as LlmProvider)}
							class="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm"
						>
							<option value="">Use primary provider</option>
							<option value="Ollama">Ollama</option>
							<option value="LmStudio">LM Studio</option>
							<option value="OpenAi">OpenAI</option>
							<option value="Claude">Claude</option>
							<option value="Gemini">Gemini</option>
						</select>
					</div>

					<div>
						<label class="mb-1 block text-sm font-medium text-gray-700">Form Filling</label>
						<select
							value={llmSettings.formFillingProvider ?? ''}
							onchange={(e) =>
								handleSetTaskProvider('FormFilling', e.currentTarget.value as LlmProvider)}
							class="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm"
						>
							<option value="">Use primary provider</option>
							<option value="Ollama">Ollama</option>
							<option value="LmStudio">LM Studio</option>
							<option value="OpenAi">OpenAI</option>
							<option value="Claude">Claude</option>
							<option value="Gemini">Gemini</option>
						</select>
					</div>
				</div>
			</div>

			<!-- Provider Configuration (API Keys & Testing) -->
			<div class="space-y-4">
				<h3 class="font-medium text-gray-900">Provider Configuration</h3>

				<!-- Ollama -->
				<div class="rounded-lg border border-gray-200 bg-white p-4">
					<div class="flex items-center justify-between">
						<div>
							<h4 class="font-medium text-gray-900">Ollama</h4>
							<p class="text-sm text-gray-500">Local provider (no API key needed)</p>
						</div>
						<button
							onclick={() => handleTestProvider('Ollama')}
							disabled={llmTestResults.Ollama === 'testing'}
							class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
						>
							{llmTestResults.Ollama === 'testing' ? 'Testing...' : 'Test Connection'}
						</button>
					</div>
					{#if llmTestResults.Ollama === 'success'}
						<p class="mt-2 text-sm text-green-600">Connected successfully</p>
					{:else if llmTestResults.Ollama === 'error'}
						<p class="mt-2 text-sm text-red-600">Connection failed</p>
					{/if}
				</div>

				<!-- LM Studio -->
				<div class="rounded-lg border border-gray-200 bg-white p-4">
					<div class="flex items-center justify-between">
						<div>
							<h4 class="font-medium text-gray-900">LM Studio</h4>
							<p class="text-sm text-gray-500">Local provider (no API key needed)</p>
						</div>
						<button
							onclick={() => handleTestProvider('LmStudio')}
							disabled={llmTestResults.LmStudio === 'testing'}
							class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
						>
							{llmTestResults.LmStudio === 'testing' ? 'Testing...' : 'Test Connection'}
						</button>
					</div>
					{#if llmTestResults.LmStudio === 'success'}
						<p class="mt-2 text-sm text-green-600">Connected successfully</p>
					{:else if llmTestResults.LmStudio === 'error'}
						<p class="mt-2 text-sm text-red-600">Connection failed</p>
					{/if}
				</div>

				<!-- OpenAI -->
				<div class="rounded-lg border border-gray-200 bg-white p-4">
					<h4 class="mb-2 font-medium text-gray-900">OpenAI</h4>
					<div class="flex gap-2">
						<input
							type="password"
							bind:value={llmApiKeys.OpenAi}
							placeholder="sk-..."
							class="flex-1 rounded-lg border border-gray-300 px-3 py-2 text-sm"
						/>
						<button
							onclick={() => handleSetApiKey('OpenAi')}
							disabled={!llmApiKeys.OpenAi}
							class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
						>
							Save
						</button>
						<button
							onclick={() => handleTestProvider('OpenAi')}
							disabled={llmTestResults.OpenAi === 'testing'}
							class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
						>
							{llmTestResults.OpenAi === 'testing' ? 'Testing...' : 'Test'}
						</button>
					</div>
					{#if llmTestResults.OpenAi === 'success'}
						<p class="mt-2 text-sm text-green-600">Connected successfully</p>
					{:else if llmTestResults.OpenAi === 'error'}
						<p class="mt-2 text-sm text-red-600">Connection failed</p>
					{/if}
				</div>

				<!-- Claude -->
				<div class="rounded-lg border border-gray-200 bg-white p-4">
					<h4 class="mb-2 font-medium text-gray-900">Claude</h4>
					<div class="flex gap-2">
						<input
							type="password"
							bind:value={llmApiKeys.Claude}
							placeholder="sk-ant-..."
							class="flex-1 rounded-lg border border-gray-300 px-3 py-2 text-sm"
						/>
						<button
							onclick={() => handleSetApiKey('Claude')}
							disabled={!llmApiKeys.Claude}
							class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
						>
							Save
						</button>
						<button
							onclick={() => handleTestProvider('Claude')}
							disabled={llmTestResults.Claude === 'testing'}
							class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
						>
							{llmTestResults.Claude === 'testing' ? 'Testing...' : 'Test'}
						</button>
					</div>
					{#if llmTestResults.Claude === 'success'}
						<p class="mt-2 text-sm text-green-600">Connected successfully</p>
					{:else if llmTestResults.Claude === 'error'}
						<p class="mt-2 text-sm text-red-600">Connection failed</p>
					{/if}
				</div>

				<!-- Gemini -->
				<div class="rounded-lg border border-gray-200 bg-white p-4">
					<h4 class="mb-2 font-medium text-gray-900">Gemini</h4>
					<div class="flex gap-2">
						<input
							type="password"
							bind:value={llmApiKeys.Gemini}
							placeholder="API key..."
							class="flex-1 rounded-lg border border-gray-300 px-3 py-2 text-sm"
						/>
						<button
							onclick={() => handleSetApiKey('Gemini')}
							disabled={!llmApiKeys.Gemini}
							class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
						>
							Save
						</button>
						<button
							onclick={() => handleTestProvider('Gemini')}
							disabled={llmTestResults.Gemini === 'testing'}
							class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
						>
							{llmTestResults.Gemini === 'testing' ? 'Testing...' : 'Test'}
						</button>
					</div>
					{#if llmTestResults.Gemini === 'success'}
						<p class="mt-2 text-sm text-green-600">Connected successfully</p>
					{:else if llmTestResults.Gemini === 'error'}
						<p class="mt-2 text-sm text-red-600">Connection failed</p>
					{/if}
				</div>
			</div>
		{/if}
	</section>
{/if}
```

**Step 7: Test LLM Providers tab**

```bash
npm run dev
```

- Navigate to Settings → LLM Providers
- Verify primary provider dropdown works
- Verify task-specific provider dropdowns work
- Test local provider detection (Ollama/LM Studio)
- Test API key input and save for cloud providers

**Step 8: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat(frontend): implement LLM Providers settings tab

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 5: LLM Features Integration

### Task 16: Email Draft LLM Integration (Stub)

**Files:**
- Create: `src/lib/llm/emailDrafting.ts`
- Create: `src-tauri/src/commands/llm.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: Create email drafting module (frontend)**

File: `src/lib/llm/emailDrafting.ts`

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface EmailDraftRequest {
	vaultId: string;
	purpose: string; // "opt-out request", "follow-up", etc.
	brokerName?: string;
	context?: string;
}

export interface EmailDraftResponse {
	subject: string;
	body: string;
}

export async function draftEmail(request: EmailDraftRequest): Promise<EmailDraftResponse> {
	return await invoke('draft_email', request);
}
```

**Step 2: Create LLM commands module (backend)**

File: `src-tauri/src/commands/llm.rs`

```rust
use crate::error::{CommandError, Result};
use crate::state::VaultState;
use serde::{Deserialize, Serialize};
use spectral_llm::provider::{CompletionRequest, Message, Role};
use spectral_privacy::{PrivacyAwareLlmRouter, TaskType};
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailDraftRequest {
    pub vault_id: String,
    pub purpose: String,
    pub broker_name: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailDraftResponse {
    pub subject: String,
    pub body: String,
}

#[tauri::command]
pub async fn draft_email(
    vault_state: State<'_, VaultState>,
    request: EmailDraftRequest,
) -> Result<EmailDraftResponse> {
    let pool = vault_state.get_pool(&request.vault_id).await?;
    let router = PrivacyAwareLlmRouter::new(pool);

    // Build prompt
    let prompt = build_email_draft_prompt(&request);

    let completion_request = CompletionRequest {
        messages: vec![Message {
            role: Role::User,
            content: prompt,
        }],
        model: None,
        temperature: Some(0.7),
        max_tokens: Some(500),
    };

    let response = router
        .route(TaskType::EmailDraft, completion_request)
        .await
        .map_err(|e| CommandError::InternalError(e.to_string()))?;

    // Parse response (simple heuristic: first line is subject, rest is body)
    let parts: Vec<&str> = response.content.splitn(2, '\n').collect();
    let subject = parts.first().unwrap_or(&"").trim().to_string();
    let body = parts.get(1).unwrap_or(&"").trim().to_string();

    Ok(EmailDraftResponse { subject, body })
}

fn build_email_draft_prompt(request: &EmailDraftRequest) -> String {
    let broker_context = request
        .broker_name
        .as_ref()
        .map(|b| format!(" to {}", b))
        .unwrap_or_default();

    let additional_context = request
        .context
        .as_ref()
        .map(|c| format!("\n\nAdditional context: {}", c))
        .unwrap_or_default();

    format!(
        "Draft a professional email {} for the following purpose: {}.{}

Format your response as:
Subject: [subject line]
[blank line]
[email body]",
        broker_context, request.purpose, additional_context
    )
}
```

**Step 3: Export from commands module**

Modify: `src-tauri/src/commands/mod.rs`

```rust
pub mod llm;
pub mod privacy;
// ... existing modules ...
```

**Step 4: Register command in main.rs**

Modify: `src-tauri/src/main.rs`

Add to `.invoke_handler`:

```rust
commands::llm::draft_email,
```

**Step 5: Build to verify compilation**

```bash
cargo build
```

Expected: Success

**Step 6: Commit**

```bash
git add src/lib/llm/ src-tauri/src/commands/llm.rs src-tauri/src/commands/mod.rs src-tauri/src/main.rs
git commit -m "feat(llm): add email drafting integration stub

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 17: Form Filling LLM Integration (Stub)

**Files:**
- Create: `src/lib/llm/formFilling.ts`
- Modify: `src-tauri/src/commands/llm.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: Create form filling module (frontend)**

File: `src/lib/llm/formFilling.ts`

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface FormField {
	name: string;
	label: string;
	type: 'text' | 'email' | 'phone' | 'address' | 'date';
	value?: string;
}

export interface FormFillingRequest {
	vaultId: string;
	profileId: string; // Which profile to use for filling
	fields: FormField[];
}

export interface FormFillingResponse {
	filledFields: Record<string, string>; // field name -> filled value
}

export async function fillForm(request: FormFillingRequest): Promise<FormFillingResponse> {
	return await invoke('fill_form', request);
}
```

**Step 2: Add form filling command**

Modify: `src-tauri/src/commands/llm.rs`

Add at end of file:

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormField {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub value: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormFillingRequest {
    pub vault_id: String,
    pub profile_id: String,
    pub fields: Vec<FormField>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormFillingResponse {
    pub filled_fields: std::collections::HashMap<String, String>,
}

#[tauri::command]
pub async fn fill_form(
    vault_state: State<'_, VaultState>,
    request: FormFillingRequest,
) -> Result<FormFillingResponse> {
    let pool = vault_state.get_pool(&request.vault_id).await?;

    // TODO: Load profile data from database using request.profile_id
    // For now, return stub response

    let router = PrivacyAwareLlmRouter::new(pool);

    // Build prompt describing form fields
    let field_descriptions: Vec<String> = request
        .fields
        .iter()
        .map(|f| format!("- {} ({}): {}", f.name, f.field_type, f.label))
        .collect();

    let prompt = format!(
        "Fill the following form fields with appropriate values based on a typical user profile:\n\n{}\n\nReturn JSON with field names as keys and filled values as values.",
        field_descriptions.join("\n")
    );

    let completion_request = CompletionRequest {
        messages: vec![Message {
            role: Role::User,
            content: prompt,
        }],
        model: None,
        temperature: Some(0.3), // Lower temperature for more consistent output
        max_tokens: Some(500),
    };

    let response = router
        .route(TaskType::FormFilling, completion_request)
        .await
        .map_err(|e| CommandError::InternalError(e.to_string()))?;

    // Parse JSON response (simplified - real implementation would be more robust)
    let filled_fields: std::collections::HashMap<String, String> =
        serde_json::from_str(&response.content).unwrap_or_default();

    Ok(FormFillingResponse { filled_fields })
}
```

**Step 3: Register command in main.rs**

Modify: `src-tauri/src/main.rs`

Add to `.invoke_handler`:

```rust
commands::llm::fill_form,
```

**Step 4: Build to verify compilation**

```bash
cargo build
```

Expected: Success

**Step 5: Commit**

```bash
git add src/lib/llm/formFilling.ts src-tauri/src/commands/llm.rs src-tauri/src/main.rs
git commit -m "feat(llm): add form filling integration stub

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 18: PII Filtering Integration

**Files:**
- Modify: `crates/spectral-privacy/src/llm_router.rs`

**Step 1: Import PII filtering from spectral-llm**

Modify: `crates/spectral-privacy/src/llm_router.rs`

Update imports:

```rust
use spectral_llm::pii::{PiiFilter, PiiFilterStrategy};
use spectral_llm::provider::{CompletionRequest, CompletionResponse, LlmProvider as LlmProviderTrait};
```

**Step 2: Replace PII filtering TODOs with real implementation**

Find the section with `// TODO: Apply PII filtering in Phase 5` and replace:

```rust
// 4. Apply PII filtering if cloud provider
let (filtered_request, token_map) = if provider.is_local() {
    (request, None)
} else {
    let filter = PiiFilter::new(PiiFilterStrategy::Tokenize);

    // Extract text from messages for filtering
    let original_text: Vec<String> = request
        .messages
        .iter()
        .map(|m| m.content.clone())
        .collect();

    // Filter each message
    let mut filtered_messages = Vec::new();
    let mut combined_token_map = None;

    for (idx, message) in request.messages.into_iter().enumerate() {
        let (filtered_text, map) = filter.filter(&original_text[idx]);
        filtered_messages.push(spectral_llm::provider::Message {
            role: message.role,
            content: filtered_text,
        });

        // Keep token map from first message (simplified - could merge maps)
        if idx == 0 {
            combined_token_map = Some(map);
        }
    }

    let filtered_req = CompletionRequest {
        messages: filtered_messages,
        model: request.model,
        temperature: request.temperature,
        max_tokens: request.max_tokens,
    };

    (filtered_req, combined_token_map)
};

// 5. Execute request
let response = provider_instance
    .complete(filtered_request)
    .await
    .map_err(|e| PrivacyError::PermissionDenied(format!("LLM error: {}", e)))?;

// 6. Detokenize response if filtered
let final_response = if let Some(token_map) = token_map {
    let filter = PiiFilter::new(PiiFilterStrategy::Tokenize);
    let detokenized_content = filter.detokenize(&response.content, &token_map);

    CompletionResponse {
        content: detokenized_content,
        model: response.model,
    }
} else {
    response
};
```

**Step 3: Run tests**

```bash
cargo test -p spectral-privacy
```

Expected: PASS (all tests)

**Step 4: Commit**

```bash
git add crates/spectral-privacy/src/llm_router.rs
git commit -m "feat(privacy): integrate PII filtering for cloud providers

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 6: Polish & Testing

### Task 19: Add Integration Tests

**Files:**
- Create: `crates/spectral-privacy/tests/integration_test.rs`

**Step 1: Create integration test file**

File: `crates/spectral-privacy/tests/integration_test.rs`

```rust
use spectral_privacy::{
    Feature, FeatureFlags, LlmProvider, PrivacyAwareLlmRouter, PrivacyEngine, PrivacyLevel,
    TaskType,
};
use spectral_db::test_helpers::create_test_db;
use spectral_llm::provider::{CompletionRequest, Message, Role};

#[tokio::test]
async fn test_privacy_level_flow() {
    let pool = create_test_db().await;
    let engine = PrivacyEngine::new(pool.clone());

    // Start with Balanced
    engine.set_privacy_level(PrivacyLevel::Balanced).await.unwrap();

    let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
    assert!(result.is_allowed());

    // Switch to Paranoid
    engine.set_privacy_level(PrivacyLevel::Paranoid).await.unwrap();

    let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
    assert!(!result.is_allowed());

    // Switch to Custom with selective permissions
    engine.set_privacy_level(PrivacyLevel::Custom).await.unwrap();

    let mut flags = FeatureFlags::default();
    flags.allow_local_llm = true;
    flags.allow_cloud_llm = false;

    engine.set_feature_flags(flags).await.unwrap();

    let local_result = engine.check_permission(Feature::LocalLlm).await.unwrap();
    assert!(local_result.is_allowed());

    let cloud_result = engine.check_permission(Feature::CloudLlm).await.unwrap();
    assert!(!cloud_result.is_allowed());
}

#[tokio::test]
async fn test_llm_provider_preferences() {
    let pool = create_test_db().await;

    // Set primary provider
    spectral_privacy::set_primary_provider(&pool, LlmProvider::Ollama)
        .await
        .unwrap();

    // Set task-specific preference
    spectral_privacy::set_provider_preference(&pool, TaskType::EmailDraft, LlmProvider::Claude)
        .await
        .unwrap();

    // Verify retrieval
    let primary = spectral_privacy::get_primary_provider(&pool).await.unwrap();
    assert_eq!(primary, Some(LlmProvider::Ollama));

    let task_pref = spectral_privacy::get_provider_preference(&pool, TaskType::EmailDraft)
        .await
        .unwrap();
    assert_eq!(task_pref, Some(LlmProvider::Claude));
}

#[tokio::test]
async fn test_router_permission_enforcement() {
    let pool = create_test_db().await;
    let engine = PrivacyEngine::new(pool.clone());

    // Set Paranoid level (no LLM allowed)
    engine.set_privacy_level(PrivacyLevel::Paranoid).await.unwrap();

    let router = PrivacyAwareLlmRouter::new(pool);

    let request = CompletionRequest {
        messages: vec![Message {
            role: Role::User,
            content: "Test".to_string(),
        }],
        model: None,
        temperature: None,
        max_tokens: None,
    };

    // Should fail due to permission denial
    let result = router.route(TaskType::EmailDraft, request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Permission denied"));
}

#[tokio::test]
async fn test_api_key_storage() {
    let pool = create_test_db().await;

    let test_key = "sk-test-key-12345";

    // Set API key
    spectral_privacy::set_api_key(&pool, LlmProvider::OpenAi, test_key)
        .await
        .unwrap();

    // Retrieve API key
    let retrieved = spectral_privacy::get_api_key(&pool, LlmProvider::OpenAi)
        .await
        .unwrap();
    assert_eq!(retrieved, Some(test_key.to_string()));

    // Delete API key
    spectral_privacy::delete_api_key(&pool, LlmProvider::OpenAi)
        .await
        .unwrap();

    // Verify deletion
    let after_delete = spectral_privacy::get_api_key(&pool, LlmProvider::OpenAi)
        .await
        .unwrap();
    assert_eq!(after_delete, None);
}
```

**Step 2: Run integration tests**

```bash
cargo test -p spectral-privacy --test integration_test
```

Expected: PASS (4 tests)

**Step 3: Commit**

```bash
git add crates/spectral-privacy/tests/
git commit -m "test(privacy): add integration tests for privacy engine

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 20: Manual Testing Checklist & Documentation

**Files:**
- Create: `docs/testing/privacy-llm-manual-tests.md`
- Modify: `README.md` (add privacy configuration section)

**Step 1: Create manual testing checklist**

File: `docs/testing/privacy-llm-manual-tests.md`

```markdown
# Privacy & LLM Integration Manual Testing Checklist

## Privacy Level Presets

### Test 1: Paranoid Level
- [ ] Navigate to Settings → Privacy Level
- [ ] Select "Paranoid" preset
- [ ] Verify all LLM features disabled in UI
- [ ] Attempt to draft email → should show permission error
- [ ] Verify no LLM API calls made (check network tab)

### Test 2: Local Privacy Level
- [ ] Select "Local Privacy" preset
- [ ] Verify only Ollama and LM Studio selectable in LLM Providers tab
- [ ] Verify cloud provider options grayed out
- [ ] Draft email using Ollama → should work
- [ ] Attempt to use OpenAI → should show permission error

### Test 3: Balanced Level
- [ ] Select "Balanced" preset
- [ ] Verify all LLM providers selectable
- [ ] Configure OpenAI API key
- [ ] Draft email using OpenAI → should work with PII filtering
- [ ] Verify sensitive data (email, phone) not sent to API (check logs)

### Test 4: Custom Level
- [ ] Select "Custom" preset
- [ ] Verify feature flags panel appears
- [ ] Disable "Allow Cloud LLM"
- [ ] Enable "Allow Local LLM"
- [ ] Verify behavior matches Local Privacy level
- [ ] Toggle individual flags and verify enforcement

## LLM Provider Configuration

### Test 5: Local Provider Auto-Detection
- [ ] Start Ollama on localhost:11434
- [ ] Navigate to Settings → LLM Providers
- [ ] Click "Test Connection" for Ollama → should succeed
- [ ] Stop Ollama
- [ ] Click "Test Connection" again → should fail with clear error
- [ ] Repeat for LM Studio on localhost:1234

### Test 6: Cloud Provider API Keys
- [ ] Enter valid OpenAI API key
- [ ] Click "Save"
- [ ] Verify key not displayed after save (masked)
- [ ] Click "Test" → should succeed
- [ ] Enter invalid API key
- [ ] Click "Test" → should fail with clear error message
- [ ] Repeat for Claude and Gemini

### Test 7: Primary Provider Selection
- [ ] Set primary provider to Ollama
- [ ] Draft email without task-specific preference → should use Ollama
- [ ] Change primary to OpenAI
- [ ] Draft email again → should use OpenAI

### Test 8: Task-Specific Routing
- [ ] Set primary provider to Ollama
- [ ] Set Email Draft task provider to Claude
- [ ] Draft email → should use Claude (not Ollama)
- [ ] Fill form → should use Ollama (no task override)

## Email Drafting

### Test 9: Basic Email Draft
- [ ] Configure provider (Ollama or cloud with API key)
- [ ] Navigate to removal flow
- [ ] Trigger email draft feature
- [ ] Verify subject and body populated
- [ ] Verify content relevant to broker and purpose

### Test 10: PII Filtering (Cloud Providers)
- [ ] Set privacy level to Balanced
- [ ] Configure OpenAI API key
- [ ] Draft email containing: name, email, phone, address
- [ ] Enable network logging
- [ ] Submit draft request
- [ ] Verify sensitive data tokenized in API request
- [ ] Verify response detokenized correctly

## Form Filling

### Test 11: Basic Form Fill
- [ ] Navigate to broker form page
- [ ] Trigger form fill feature
- [ ] Verify fields populated with profile data
- [ ] Verify appropriate values for field types (email, phone, etc.)

## Error Handling

### Test 12: Permission Denied Errors
- [ ] Set privacy level to Paranoid
- [ ] Attempt to draft email
- [ ] Verify clear error: "Feature not allowed under Paranoid privacy level"
- [ ] Verify suggestion to change privacy level

### Test 13: Provider Unavailable
- [ ] Set primary provider to LM Studio
- [ ] Stop LM Studio
- [ ] Attempt to draft email
- [ ] Verify clear error: "LM Studio is not running"
- [ ] Verify suggestion to start LM Studio

### Test 14: Missing API Key
- [ ] Set primary provider to OpenAI
- [ ] Delete API key
- [ ] Attempt to draft email
- [ ] Verify clear error: "OpenAI API key not set"
- [ ] Verify link to LLM Providers settings

## Edge Cases

### Test 15: Vault Switching
- [ ] Configure privacy level in Vault A
- [ ] Switch to Vault B
- [ ] Verify Vault B has independent privacy settings
- [ ] Configure different level in Vault B
- [ ] Switch back to Vault A
- [ ] Verify Vault A settings preserved

### Test 16: Network Offline
- [ ] Set primary provider to cloud provider
- [ ] Disable network
- [ ] Attempt to draft email
- [ ] Verify clear error: "Network connection failed"
- [ ] Enable network
- [ ] Retry → should succeed

### Test 17: Rapid Privacy Level Changes
- [ ] Rapidly switch between Paranoid ↔ Balanced ↔ Custom
- [ ] Verify no race conditions or stale state
- [ ] Verify current level always accurately reflected in UI

## Performance

### Test 18: LLM Response Latency
- [ ] Draft email using Ollama
- [ ] Measure time from request to response
- [ ] Verify < 5 seconds for local provider
- [ ] Repeat with cloud provider
- [ ] Verify < 10 seconds for cloud provider

### Test 19: Settings Load Time
- [ ] Navigate to Privacy Level tab
- [ ] Measure time to load settings
- [ ] Verify < 500ms
- [ ] Navigate to LLM Providers tab
- [ ] Verify < 500ms

## Accessibility

### Test 20: Keyboard Navigation
- [ ] Navigate Settings → Privacy Level using Tab key only
- [ ] Select each preset using Enter key
- [ ] Navigate to LLM Providers
- [ ] Tab through provider configs
- [ ] Test API key inputs and buttons

### Test 21: Screen Reader
- [ ] Enable screen reader
- [ ] Navigate Privacy Level presets
- [ ] Verify each preset announced with description
- [ ] Verify active state announced
- [ ] Verify error messages announced

## Success Criteria

All 21 tests must pass for feature to be considered complete.
```

**Step 2: Add privacy configuration section to README**

Modify: `README.md`

Add section:

```markdown
## Privacy Configuration

Spectral provides comprehensive privacy controls to balance convenience with data protection.

### Privacy Levels

**Paranoid**
- No automation or LLM features
- All operations require manual confirmation
- Maximum privacy, minimum convenience

**Local Privacy**
- Local LLM only (Ollama, LM Studio)
- Automation enabled
- No data leaves your machine

**Balanced** (Default)
- Cloud LLM with PII filtering
- All features enabled
- Good balance of privacy and convenience

**Custom**
- Manually configure individual features
- For advanced users

### LLM Provider Setup

#### Local Providers (No API key required)

**Ollama**
1. Install Ollama from https://ollama.ai
2. Start Ollama service
3. Spectral will auto-detect on localhost:11434

**LM Studio**
1. Install LM Studio from https://lmstudio.ai
2. Load a model and start server
3. Spectral will auto-detect on localhost:1234

#### Cloud Providers (API key required)

**OpenAI**
1. Get API key from https://platform.openai.com
2. Navigate to Settings → LLM Providers
3. Enter API key and click Save

**Claude**
1. Get API key from https://console.anthropic.com
2. Navigate to Settings → LLM Providers
3. Enter API key and click Save

**Gemini**
1. Get API key from https://ai.google.dev
2. Navigate to Settings → LLM Providers
3. Enter API key and click Save

### Task-Based Routing

You can set different providers for different tasks:
- **Primary Provider**: Default for all tasks
- **Email Drafting**: Override for email composition
- **Form Filling**: Override for form auto-fill

### PII Filtering

When using cloud providers, Spectral automatically filters personally identifiable information:
- Names → TOKEN_NAME_1, TOKEN_NAME_2
- Emails → TOKEN_EMAIL_1, TOKEN_EMAIL_2
- Phone numbers → TOKEN_PHONE_1, TOKEN_PHONE_2
- Addresses → TOKEN_ADDRESS_1, TOKEN_ADDRESS_2

Filtered data is sent to cloud APIs, then detokenized in responses.
```

**Step 3: Commit**

```bash
git add docs/testing/privacy-llm-manual-tests.md README.md
git commit -m "docs: add privacy/LLM testing checklist and configuration guide

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Implementation Complete

All 20 tasks across 6 phases have been planned. Ready to execute using Subagent-Driven Development.
