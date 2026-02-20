use crate::error::Result;
use crate::types::{Feature, FeatureFlags, PermissionResult, PrivacyLevel};
use sqlx::SqlitePool;

/// Central orchestrator for all privacy-related decisions.
///
/// The `PrivacyEngine` is the single source of truth for privacy settings,
/// managing privacy levels, feature flags, and permission checks.
#[derive(Debug, Clone)]
pub struct PrivacyEngine {
    pool: SqlitePool,
}

impl PrivacyEngine {
    /// Create a new privacy engine with the given database pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get the current privacy level.
    ///
    /// Returns `PrivacyLevel::Balanced` as the default if no level is set.
    ///
    /// # Errors
    /// Returns an error if the database query fails or the stored value is invalid.
    pub async fn get_privacy_level(&self) -> Result<PrivacyLevel> {
        let value = spectral_db::settings::get_setting(&self.pool, "privacy_level").await?;

        match value {
            Some(v) => {
                let level: PrivacyLevel = serde_json::from_value(v)?;
                Ok(level)
            }
            None => Ok(PrivacyLevel::Balanced), // Default
        }
    }

    /// Set the privacy level.
    ///
    /// # Errors
    /// Returns an error if the database update fails.
    pub async fn set_privacy_level(&self, level: PrivacyLevel) -> Result<()> {
        let value = serde_json::to_value(level)?;
        spectral_db::settings::set_setting(&self.pool, "privacy_level", &value).await?;
        Ok(())
    }

    /// Check if a feature is allowed under the current privacy settings.
    ///
    /// # Errors
    /// Returns an error if the database query fails.
    pub async fn check_permission(&self, feature: Feature) -> Result<PermissionResult> {
        let level = self.get_privacy_level().await?;

        // For Custom level, check feature flags
        if level == PrivacyLevel::Custom {
            let flags = self.get_feature_flags().await?;
            return Ok(flags.check_feature(feature));
        }

        // For predefined levels, use the level's feature flags
        let flags = level.to_feature_flags();
        let result = flags.check_feature(feature);

        // Add privacy level to denial reason for predefined levels
        if let PermissionResult::Denied { reason: _ } = result {
            return Ok(PermissionResult::Denied {
                reason: format!("Privacy level {level:?} does not allow {feature:?}"),
            });
        }

        Ok(result)
    }

    /// Get the current feature flags.
    ///
    /// Returns default flags if none are set.
    ///
    /// # Errors
    /// Returns an error if the database query fails or the stored value is invalid.
    pub async fn get_feature_flags(&self) -> Result<FeatureFlags> {
        let value = spectral_db::settings::get_setting(&self.pool, "feature_flags").await?;

        match value {
            Some(v) => {
                let flags: FeatureFlags = serde_json::from_value(v)?;
                Ok(flags)
            }
            None => Ok(FeatureFlags::default()),
        }
    }

    /// Set custom feature flags.
    ///
    /// Note: This only takes effect when the privacy level is set to Custom.
    ///
    /// # Errors
    /// Returns an error if the database update fails.
    pub async fn set_feature_flags(&self, flags: FeatureFlags) -> Result<()> {
        let value = serde_json::to_value(flags)?;
        spectral_db::settings::set_setting(&self.pool, "feature_flags", &value).await?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::types::PrivacyLevel;
    use spectral_db::Database;

    async fn create_test_db() -> SqlitePool {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create test database");
        db.run_migrations().await.expect("run migrations");
        db.pool().clone()
    }

    #[tokio::test]
    async fn test_get_default_privacy_level() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        // nosemgrep: no-unwrap-in-production
        let level = engine.get_privacy_level().await.unwrap();
        assert_eq!(level, PrivacyLevel::Balanced); // Default
    }

    #[tokio::test]
    async fn test_set_privacy_level() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        // nosemgrep: no-unwrap-in-production
        engine
            .set_privacy_level(PrivacyLevel::Paranoid)
            .await
            .unwrap();

        // nosemgrep: no-unwrap-in-production
        let level = engine.get_privacy_level().await.unwrap();
        assert_eq!(level, PrivacyLevel::Paranoid);
    }

    #[tokio::test]
    async fn test_check_permission_allowed() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        // nosemgrep: no-unwrap-in-production
        engine
            .set_privacy_level(PrivacyLevel::Balanced)
            .await
            .unwrap();

        // nosemgrep: no-unwrap-in-production
        let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
        assert!(result.is_allowed());
    }

    #[tokio::test]
    async fn test_check_permission_denied() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        // nosemgrep: no-unwrap-in-production
        engine
            .set_privacy_level(PrivacyLevel::Paranoid)
            .await
            .unwrap();

        // nosemgrep: no-unwrap-in-production
        let result = engine.check_permission(Feature::CloudLlm).await.unwrap();
        assert!(!result.is_allowed());
        // nosemgrep: no-unwrap-in-production
        assert!(result.reason().unwrap().contains("Paranoid"));
    }

    #[tokio::test]
    async fn test_custom_feature_flags() {
        let pool = create_test_db().await;
        let engine = PrivacyEngine::new(pool);

        let flags = FeatureFlags {
            allow_cloud_llm: false,
            ..Default::default()
        };

        // nosemgrep: no-unwrap-in-production
        engine
            .set_privacy_level(PrivacyLevel::Custom)
            .await
            .unwrap();
        // nosemgrep: no-unwrap-in-production
        engine.set_feature_flags(flags.clone()).await.unwrap();

        // nosemgrep: no-unwrap-in-production
        let retrieved = engine.get_feature_flags().await.unwrap();
        assert_eq!(retrieved, flags);
    }
}
