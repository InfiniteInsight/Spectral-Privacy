//! Database migration management.
//!
//! Embeds SQL migrations and provides functions to apply them automatically.
//! Uses `SQLx`'s built-in migration support with compile-time embedding.

use crate::error::{DatabaseError, Result};
use sqlx::{Pool, Sqlite};

/// Run all pending database migrations.
///
/// This function applies all migrations in the `migrations/` directory that
/// haven't been applied yet. It uses `SQLx`'s built-in migration system which
/// tracks applied migrations in a `_sqlx_migrations` table.
///
/// # Errors
/// Returns `DatabaseError::Migration` if any migration fails to execute.
pub async fn run_migrations(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::info!("Running database migrations");

    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| DatabaseError::Migration(format!("migration execution failed: {e}")))?;

    tracing::info!("Database migrations completed successfully");
    Ok(())
}

/// Get the current schema version.
///
/// Returns the number of applied migrations. Returns 0 if no migrations
/// have been applied yet or if the migrations table doesn't exist.
///
/// # Errors
/// Returns `DatabaseError` if the migrations table cannot be queried.
pub async fn get_schema_version(pool: &Pool<Sqlite>) -> Result<i64> {
    // Check if the migrations table exists
    let table_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
    )
    .fetch_one(pool)
    .await?
        > 0;

    if !table_exists {
        return Ok(0);
    }

    let version =
        sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
            .fetch_optional(pool)
            .await?
            .unwrap_or(0);

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::EncryptedPool;

    #[tokio::test]
    async fn test_run_migrations() {
        let key = vec![0u8; 32];
        let pool = EncryptedPool::new(":memory:", key)
            .await
            .expect("create encrypted pool");

        run_migrations(pool.pool()).await.expect("run migrations");

        // Verify tables were created
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations' ORDER BY name"
        )
        .fetch_all(pool.pool())
        .await
        .expect("query tables");

        assert_eq!(
            tables,
            vec![
                "audit_log",
                "broker_results",
                "broker_scans",
                "discovery_findings",
                "email_removals",
                "findings",
                "profiles",
                "removal_attempts",
                "removal_evidence",
                "scan_jobs",
                "scheduled_jobs"
            ]
        );
    }

    #[tokio::test]
    async fn test_get_schema_version() {
        let key = vec![0u8; 32];
        let pool = EncryptedPool::new(":memory:", key)
            .await
            .expect("create encrypted pool");

        // Before migrations
        let version = get_schema_version(pool.pool()).await.expect("get version");
        assert_eq!(version, 0);

        // After migrations
        run_migrations(pool.pool()).await.expect("run migrations");

        let version = get_schema_version(pool.pool()).await.expect("get version");
        assert_eq!(version, 9); // Nine migrations applied
    }

    #[tokio::test]
    async fn test_migrations_idempotent() {
        let key = vec![0u8; 32];
        let pool = EncryptedPool::new(":memory:", key)
            .await
            .expect("create encrypted pool");

        // Run migrations twice
        run_migrations(pool.pool())
            .await
            .expect("first migration run");

        run_migrations(pool.pool())
            .await
            .expect("second migration run should be idempotent");

        let version = get_schema_version(pool.pool()).await.expect("get version");
        assert_eq!(version, 9);
    }
}
