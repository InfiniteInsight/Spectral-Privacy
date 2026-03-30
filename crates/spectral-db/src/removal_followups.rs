//! Database operations for removal follow-up reminders.

use sqlx::SqlitePool;

/// A scheduled follow-up reminder for a removal attempt.
#[derive(Debug, Clone)]
pub struct RemovalFollowup {
    /// Unique follow-up ID.
    pub id: String,
    /// ID of the associated removal attempt.
    pub attempt_id: String,
    /// Broker identifier.
    pub broker_id: String,
    /// Broker email address to send the follow-up to.
    pub recipient: String,
    /// ISO-8601 timestamp: when to follow up (typically `submitted_at` + 15 days).
    pub follow_up_at: String,
    /// ISO-8601 timestamp when the follow-up was sent; `None` if not yet sent.
    pub sent_at: Option<String>,
    /// ISO-8601 timestamp when the user dismissed this reminder; `None` if not dismissed.
    pub dismissed_at: Option<String>,
    /// How the follow-up was resolved: `'smtp_auto'`, `'user_dismissed'`, or `None` when pending.
    pub method: Option<String>,
}

type FollowupRow = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn row_to_followup(r: FollowupRow) -> RemovalFollowup {
    RemovalFollowup {
        id: r.0,
        attempt_id: r.1,
        broker_id: r.2,
        recipient: r.3,
        follow_up_at: r.4,
        sent_at: r.5,
        dismissed_at: r.6,
        method: r.7,
    }
}

const SELECT_COLS: &str = "id, attempt_id, broker_id, recipient, follow_up_at, \
                            sent_at, dismissed_at, method";

/// Schedule a follow-up reminder for a removal attempt.
///
/// `follow_up_at` must be an RFC-3339 timestamp.
///
/// # Errors
/// Returns `sqlx::Error` if the insert fails.
pub async fn schedule_followup(
    pool: &SqlitePool,
    attempt_id: &str,
    broker_id: &str,
    recipient: &str,
    follow_up_at: &str,
) -> Result<String, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO removal_followups (id, attempt_id, broker_id, recipient, follow_up_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(attempt_id)
    .bind(broker_id)
    .bind(recipient)
    .bind(follow_up_at)
    .execute(pool)
    .await?;
    Ok(id)
}

/// Return follow-ups that are due (`follow_up_at <= now`) and not yet sent or dismissed.
///
/// # Errors
/// Returns `sqlx::Error` if the query fails.
pub async fn get_due_followups(pool: &SqlitePool) -> Result<Vec<RemovalFollowup>, sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    let rows = sqlx::query_as::<_, FollowupRow>(&format!(
        "SELECT {SELECT_COLS} FROM removal_followups
         WHERE follow_up_at <= ? AND sent_at IS NULL AND dismissed_at IS NULL
         ORDER BY follow_up_at ASC"
    ))
    .bind(&now)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(row_to_followup).collect())
}

/// Return all pending (unsent, undismissed) follow-ups regardless of due date.
///
/// # Errors
/// Returns `sqlx::Error` if the query fails.
pub async fn get_pending_followups(pool: &SqlitePool) -> Result<Vec<RemovalFollowup>, sqlx::Error> {
    let rows = sqlx::query_as::<_, FollowupRow>(&format!(
        "SELECT {SELECT_COLS} FROM removal_followups
         WHERE sent_at IS NULL AND dismissed_at IS NULL
         ORDER BY follow_up_at ASC"
    ))
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(row_to_followup).collect())
}

/// Mark a follow-up as sent by the auto-scheduler.
///
/// # Errors
/// Returns `sqlx::Error` if the update fails.
pub async fn mark_followup_sent(
    pool: &SqlitePool,
    followup_id: &str,
    method: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE removal_followups SET sent_at = ?, method = ? WHERE id = ?")
        .bind(&now)
        .bind(method)
        .bind(followup_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Mark a follow-up as dismissed by the user.
///
/// # Errors
/// Returns `sqlx::Error` if the update fails.
pub async fn dismiss_followup(pool: &SqlitePool, followup_id: &str) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE removal_followups SET dismissed_at = ?, method = 'user_dismissed' WHERE id = ?",
    )
    .bind(&now)
    .bind(followup_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_pool() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:")
            .await
            .expect("in-memory pool"); // nosemgrep: no-unwrap-in-production
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations"); // nosemgrep: no-unwrap-in-production
        pool
    }

    async fn insert_stub_attempt(pool: &SqlitePool, id: &str) {
        sqlx::query(
            "INSERT INTO removal_attempts (id, broker_id, status, created_at)
             VALUES (?, 'klaviyo', 'Submitted', '2026-03-29T00:00:00Z')",
        )
        .bind(id)
        .execute(pool)
        .await
        .expect("stub attempt"); // nosemgrep: no-unwrap-in-production
    }

    #[tokio::test]
    async fn test_schedule_and_get_pending() {
        let pool = make_pool().await;
        insert_stub_attempt(&pool, "att-1").await;

        let id = schedule_followup(
            &pool,
            "att-1",
            "klaviyo",
            "privacy@klaviyo.com",
            "2099-01-01T00:00:00Z",
        )
        .await
        .expect("schedule"); // nosemgrep: no-unwrap-in-production

        let pending = get_pending_followups(&pool).await.expect("pending"); // nosemgrep: no-unwrap-in-production
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
        assert_eq!(pending[0].recipient, "privacy@klaviyo.com");
        assert!(pending[0].sent_at.is_none());
    }

    #[tokio::test]
    async fn test_dismiss_removes_from_pending() {
        let pool = make_pool().await;
        insert_stub_attempt(&pool, "att-2").await;

        let id = schedule_followup(
            &pool,
            "att-2",
            "klaviyo",
            "privacy@klaviyo.com",
            "2099-01-01T00:00:00Z",
        )
        .await
        .expect("schedule"); // nosemgrep: no-unwrap-in-production

        dismiss_followup(&pool, &id).await.expect("dismiss"); // nosemgrep: no-unwrap-in-production

        let pending = get_pending_followups(&pool).await.expect("pending"); // nosemgrep: no-unwrap-in-production
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_due_followups_filters_by_date() {
        let pool = make_pool().await;
        insert_stub_attempt(&pool, "att-3").await;
        insert_stub_attempt(&pool, "att-4").await;

        // Past date — should appear in due list
        schedule_followup(
            &pool,
            "att-3",
            "klaviyo",
            "privacy@klaviyo.com",
            "2000-01-01T00:00:00Z",
        )
        .await
        .expect("past"); // nosemgrep: no-unwrap-in-production

        // Future date — should NOT appear in due list
        schedule_followup(
            &pool,
            "att-4",
            "klaviyo",
            "privacy@klaviyo.com",
            "2099-01-01T00:00:00Z",
        )
        .await
        .expect("future"); // nosemgrep: no-unwrap-in-production

        let due = get_due_followups(&pool).await.expect("due"); // nosemgrep: no-unwrap-in-production
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].attempt_id, "att-3");
    }

    #[tokio::test]
    async fn test_mark_sent_removes_from_due() {
        let pool = make_pool().await;
        insert_stub_attempt(&pool, "att-5").await;

        let id = schedule_followup(
            &pool,
            "att-5",
            "klaviyo",
            "privacy@klaviyo.com",
            "2000-01-01T00:00:00Z",
        )
        .await
        .expect("schedule"); // nosemgrep: no-unwrap-in-production

        mark_followup_sent(&pool, &id, "smtp_auto")
            .await
            .expect("mark sent"); // nosemgrep: no-unwrap-in-production

        let due = get_due_followups(&pool).await.expect("due"); // nosemgrep: no-unwrap-in-production
        assert!(due.is_empty());
    }
}
