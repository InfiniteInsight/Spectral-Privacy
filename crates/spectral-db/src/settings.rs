//! Settings storage for application configuration.
//!
//! Provides key-value storage for application settings using the settings table.
//! Settings values are stored as JSON, enabling flexible schema-less configuration.

use crate::error::{DatabaseError, Result};
use serde_json::Value;
use sqlx::SqlitePool;

/// Set a setting in the database
pub async fn set_setting(pool: &SqlitePool, key: &str, value: &Value) -> Result<()> {
    let value_str = serde_json::to_string(value)
        .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

    sqlx::query(
        r"
        INSERT INTO settings (key, value, updated_at)
        VALUES (?, ?, datetime('now'))
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = datetime('now')
        ",
    )
    .bind(key)
    .bind(value_str)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get a setting from the database
pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<Value>> {
    let row: Option<(String,)> = sqlx::query_as(
        r"
        SELECT value
        FROM settings
        WHERE key = ?
        ",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;

    match row {
        Some((value_str,)) => {
            let value: Value = serde_json::from_str(&value_str)
                .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

/// Delete a setting from the database
pub async fn delete_setting(pool: &SqlitePool, key: &str) -> Result<()> {
    sqlx::query(
        r"
        DELETE FROM settings
        WHERE key = ?
        ",
    )
    .bind(key)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::Database;

    async fn create_test_db() -> Database {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key)
            .await
            .expect("create test database");
        db.run_migrations().await.expect("run migrations");
        db
    }

    #[tokio::test]
    async fn test_set_and_get_setting() {
        let db = create_test_db().await;
        let pool = db.pool();

        let value = serde_json::json!({"level": "Balanced"});
        // nosemgrep: no-unwrap-in-production
        set_setting(pool, "privacy_level", &value).await.unwrap();

        // nosemgrep: no-unwrap-in-production
        let retrieved = get_setting(pool, "privacy_level").await.unwrap();
        assert_eq!(retrieved, Some(value));
    }

    #[tokio::test]
    async fn test_get_nonexistent_setting() {
        let db = create_test_db().await;
        let pool = db.pool();

        // nosemgrep: no-unwrap-in-production
        let result = get_setting(pool, "does_not_exist").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_delete_setting() {
        let db = create_test_db().await;
        let pool = db.pool();

        let value = serde_json::json!({"test": true});
        // nosemgrep: no-unwrap-in-production
        set_setting(pool, "test_key", &value).await.unwrap();

        // nosemgrep: no-unwrap-in-production
        delete_setting(pool, "test_key").await.unwrap();

        // nosemgrep: no-unwrap-in-production
        let result = get_setting(pool, "test_key").await.unwrap();
        assert_eq!(result, None);
    }
}
