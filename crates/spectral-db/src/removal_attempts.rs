//! Removal attempt database operations.

use crate::error::{DatabaseError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use spectral_broker::removal::RemovalOutcome;
use sqlx::{Pool, Row, Sqlite};

/// A removal attempt record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovalAttempt {
    /// Unique identifier for this removal attempt
    pub id: String,
    /// Reference to the broker result being removed
    pub broker_result_id: String,
    /// Identifier of the broker being contacted
    pub broker_id: String,
    /// Timestamp when the removal was attempted
    pub attempted_at: DateTime<Utc>,
    /// Outcome of the removal attempt
    pub outcome: RemovalOutcome,
    /// Email address used for verification (if applicable)
    pub verification_email: Option<String>,
    /// Optional notes about this attempt
    pub notes: Option<String>,
}

impl RemovalAttempt {
    /// Create a new removal attempt.
    #[must_use]
    pub fn new(broker_result_id: String, broker_id: String, outcome: RemovalOutcome) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            broker_result_id,
            broker_id,
            attempted_at: Utc::now(),
            outcome,
            verification_email: None,
            notes: None,
        }
    }

    /// Save the removal attempt to the database.
    pub async fn save(&self, pool: &Pool<Sqlite>) -> Result<()> {
        let outcome_json = serde_json::to_string(&self.outcome)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        let outcome_type = match &self.outcome {
            RemovalOutcome::Submitted => "Submitted",
            RemovalOutcome::RequiresEmailVerification { .. } => "RequiresEmailVerification",
            RemovalOutcome::RequiresCaptcha { .. } => "RequiresCaptcha",
            RemovalOutcome::RequiresAccountCreation => "RequiresAccountCreation",
            RemovalOutcome::Failed { .. } => "Failed",
        };

        sqlx::query(
            "INSERT INTO removal_attempts (
                id, broker_result_id, broker_id, attempted_at,
                outcome_type, outcome_data, verification_email, notes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&self.id)
        .bind(&self.broker_result_id)
        .bind(&self.broker_id)
        .bind(self.attempted_at.to_rfc3339())
        .bind(outcome_type)
        .bind(outcome_json)
        .bind(&self.verification_email)
        .bind(&self.notes)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Load a removal attempt by ID.
    pub async fn load(pool: &Pool<Sqlite>, id: &str) -> Result<Self> {
        let row = sqlx::query(
            "SELECT id, broker_result_id, broker_id, attempted_at,
             outcome_data, verification_email, notes
             FROM removal_attempts WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        let outcome_data: String = row.try_get("outcome_data")?;
        let outcome: RemovalOutcome = serde_json::from_str(&outcome_data)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        let attempted_at_str: String = row.try_get("attempted_at")?;
        let attempted_at = DateTime::parse_from_rfc3339(&attempted_at_str)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?
            .with_timezone(&Utc);

        Ok(Self {
            id: row.try_get("id")?,
            broker_result_id: row.try_get("broker_result_id")?,
            broker_id: row.try_get("broker_id")?,
            attempted_at,
            outcome,
            verification_email: row.try_get("verification_email")?,
            notes: row.try_get("notes")?,
        })
    }

    /// Load all removal attempts for a broker result.
    pub async fn load_for_result(pool: &Pool<Sqlite>, broker_result_id: &str) -> Result<Vec<Self>> {
        let rows = sqlx::query(
            "SELECT id, broker_result_id, broker_id, attempted_at,
             outcome_data, verification_email, notes
             FROM removal_attempts
             WHERE broker_result_id = ?1
             ORDER BY attempted_at DESC",
        )
        .bind(broker_result_id)
        .fetch_all(pool)
        .await?;

        let mut attempts = Vec::new();
        for row in rows {
            let outcome_data: String = row.try_get("outcome_data")?;
            let outcome: RemovalOutcome = serde_json::from_str(&outcome_data)
                .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

            let attempted_at_str: String = row.try_get("attempted_at")?;
            let attempted_at = DateTime::parse_from_rfc3339(&attempted_at_str)
                .map_err(|e| DatabaseError::SerializationError(e.to_string()))?
                .with_timezone(&Utc);

            attempts.push(Self {
                id: row.try_get("id")?,
                broker_result_id: row.try_get("broker_result_id")?,
                broker_id: row.try_get("broker_id")?,
                attempted_at,
                outcome,
                verification_email: row.try_get("verification_email")?,
                notes: row.try_get("notes")?,
            });
        }

        Ok(attempts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use chrono::Utc;

    async fn setup_test_db() -> Database {
        let key = vec![0u8; 32];
        let db = Database::new(":memory:", key).await.unwrap();
        db.run_migrations().await.unwrap();
        db
    }

    async fn create_test_broker_result(pool: &Pool<Sqlite>, id: &str) {
        // Create a profile first (required by foreign key)
        let dummy_data = [0u8; 32];
        let dummy_nonce = [0u8; 12];

        sqlx::query(
            "INSERT INTO profiles (id, data, nonce, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind("profile123")
        .bind(&dummy_data[..]) // dummy encrypted data
        .bind(&dummy_nonce[..]) // dummy nonce
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(pool)
        .await
        .unwrap();

        // Create broker_result (referenced by removal_attempts)
        sqlx::query(
            "INSERT INTO broker_results (
                id, profile_id, broker_id, status,
                first_seen, last_checked
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(id)
        .bind("profile123")
        .bind("spokeo")
        .bind("found")
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_save_and_load_removal_attempt() {
        let db = setup_test_db().await;
        create_test_broker_result(db.pool(), "result123").await;

        let attempt = RemovalAttempt::new(
            "result123".to_string(),
            "spokeo".to_string(),
            RemovalOutcome::Submitted,
        );

        attempt.save(db.pool()).await.unwrap();

        let loaded = RemovalAttempt::load(db.pool(), &attempt.id).await.unwrap();
        assert_eq!(loaded.id, attempt.id);
        assert_eq!(loaded.broker_id, "spokeo");
    }

    #[tokio::test]
    async fn test_load_for_result() {
        let db = setup_test_db().await;
        create_test_broker_result(db.pool(), "result123").await;

        let first_attempt = RemovalAttempt::new(
            "result123".to_string(),
            "spokeo".to_string(),
            RemovalOutcome::Submitted,
        );
        let second_attempt = RemovalAttempt::new(
            "result123".to_string(),
            "spokeo".to_string(),
            RemovalOutcome::Failed {
                reason: "Timeout".to_string(),
                error_details: None,
            },
        );

        first_attempt.save(db.pool()).await.unwrap();
        second_attempt.save(db.pool()).await.unwrap();

        let all_attempts = RemovalAttempt::load_for_result(db.pool(), "result123")
            .await
            .unwrap();
        assert_eq!(all_attempts.len(), 2);
    }
}
