# Removal Follow-Up Reminder System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** When a removal email is sent, schedule a follow-up after 15 days. If LLM + SMTP are configured, auto-send the follow-up. Otherwise, surface a notification badge on the removals history page so the user knows to follow up manually.

**Architecture:** A new `removal_followups` DB table tracks pending follow-ups per removal attempt. A new `FollowUpReminders` daily scheduler job auto-sends via LLM+SMTP when available, leaving others pending for the UI. A `FollowUpNotifications` Svelte component renders above the removal history list, showing overdue/upcoming reminders with dismiss buttons.

**Tech Stack:** Rust/SQLite/sqlx (spectral-db, spectral-scheduler), Tauri commands (src-tauri), Svelte 5 runes (frontend), spectral-mail (SMTP), spectral-privacy (LLM router), Tailwind CSS

---

## Phase Tracking

| Phase | Title | Status |
|-------|-------|--------|
| Phase 1 | Data Layer — `removal_followups` table + `FollowUpReminders` job type | ✅ Complete |
| Phase 2 | Backend — DB queries, Tauri commands, scheduler job handler | ✅ Complete |
| Phase 3 | Email Send Wiring — schedule follow-ups at send time | ✅ Complete |
| Phase 4 | Frontend — notification component + removal history integration | ✅ Complete |

> **Instructions for Claude:** Update the Status column above as you complete phases. Use: ⬜ Not Started → 🔄 In Progress → ✅ Complete → ❌ Blocked (add note below the table)

---

## File Map

### New Files
| File | Purpose |
|------|---------|
| `crates/spectral-db/migrations/023_removal_followups.sql` | Schema for `removal_followups` table |
| `crates/spectral-db/migrations/024_seed_followup_job.sql` | Seed `FollowUpReminders` row in `scheduled_jobs` |
| `crates/spectral-db/src/removal_followups.rs` | DB query functions: schedule, get due, get pending, mark sent, dismiss |
| `src/lib/api/followups.ts` | TypeScript API wrappers for follow-up commands |
| `src/lib/components/removals/FollowUpNotifications.svelte` | Notification card component with overdue badge + dismiss |

### Modified Files
| File | What Changes |
|------|-------------|
| `crates/spectral-scheduler/src/jobs.rs` | Add `FollowUpReminders` variant to `JobType` enum |
| `crates/spectral-db/src/lib.rs` | Re-export `removal_followups` module and functions |
| `src-tauri/src/commands/scheduler.rs` | Add `FollowUpReminders` match arm + `run_followup_reminders_job` + `send_auto_followup` helpers |
| `src-tauri/src/commands/removal.rs` | Add `get_pending_followups` and `dismiss_followup` Tauri commands |
| `src-tauri/src/lib.rs` | Register two new Tauri commands |
| `src-tauri/src/removal_worker.rs` | After `submit_via_email` succeeds, call `schedule_followup` |
| `src-tauri/src/commands/scan.rs` | After `send_removal_email` INSERT, call `schedule_followup` |
| `src/routes/removals/+page.svelte` | Import and render `<FollowUpNotifications>` above the job list |

---

## Phase 1: Data Layer

**Goal:** Add the `removal_followups` DB table and the `FollowUpReminders` job type so the rest of the system has a foundation to build on.

**Phase Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting Task 1.1. Set to ✅ after Task 1.3 commits cleanly.

---

### Task 1.1 — DB Migration: `removal_followups` table

**Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting. Set to ✅ after the commit succeeds.

**Files:**
- Create: `crates/spectral-db/migrations/023_removal_followups.sql`

**Context:** The next migration number is 023. The `removal_attempts` table already exists with `id TEXT PRIMARY KEY`. The `ON DELETE CASCADE` on `attempt_id` ensures follow-up rows are cleaned up when a removal attempt is deleted.

- [x] **Step 1: Create the migration**

```sql
-- crates/spectral-db/migrations/023_removal_followups.sql
CREATE TABLE IF NOT EXISTS removal_followups (
    id           TEXT PRIMARY KEY NOT NULL,
    attempt_id   TEXT NOT NULL REFERENCES removal_attempts(id) ON DELETE CASCADE,
    broker_id    TEXT NOT NULL,
    recipient    TEXT NOT NULL,      -- broker email address to follow up with
    follow_up_at TEXT NOT NULL,      -- ISO-8601: submitted_at + 15 days
    sent_at      TEXT,               -- ISO-8601: null = not yet sent or dismissed
    dismissed_at TEXT,               -- ISO-8601: null = not dismissed by user
    method       TEXT                -- 'smtp_auto' | 'user_dismissed' | null when pending
);

CREATE INDEX idx_removal_followups_attempt ON removal_followups(attempt_id);
CREATE INDEX idx_removal_followups_due
    ON removal_followups(follow_up_at)
    WHERE sent_at IS NULL AND dismissed_at IS NULL;
```

- [x] **Step 2: Build check**

```bash
cargo build -p spectral-db 2>&1 | grep "^error"
```

Expected: no output (zero errors)

- [x] **Step 3: Commit**

```bash
git add crates/spectral-db/migrations/023_removal_followups.sql
git commit -m "chore(db): add migration for removal_followups table"
```

---

### Task 1.2 — DB Migration: Seed `FollowUpReminders` scheduled job

**Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting. Set to ✅ after the commit succeeds.

**Files:**
- Create: `crates/spectral-db/migrations/024_seed_followup_job.sql`

**Context:** The `scheduled_jobs` table is seeded in migrations (see migration 008). The `job_type` value must match the serde serialization of `JobType::FollowUpReminders` — with `#[serde(rename_all = "PascalCase")]` that is `"FollowUpReminders"`. `interval_days = 1` means the job runs daily.

- [x] **Step 1: Create the migration**

```sql
-- crates/spectral-db/migrations/024_seed_followup_job.sql
INSERT OR IGNORE INTO scheduled_jobs (id, job_type, interval_days, next_run_at, enabled)
VALUES ('default-followup-reminders', 'FollowUpReminders', 1, datetime('now'), 1);
```

- [x] **Step 2: Build check**

```bash
cargo build -p spectral-db 2>&1 | grep "^error"
```

Expected: no output

- [x] **Step 3: Commit**

```bash
git add crates/spectral-db/migrations/024_seed_followup_job.sql
git commit -m "chore(db): seed FollowUpReminders scheduled job (daily)"
```

---

### Task 1.3 — Add `FollowUpReminders` to `JobType` enum

**Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting. Set to ✅ after the commit succeeds.

**Files:**
- Modify: `crates/spectral-scheduler/src/jobs.rs`

**Context:** The current `JobType` enum has `ScanAll`, `VerifyRemovals`, `PollImap`. Adding `FollowUpReminders` here will cause a non-exhaustive match compile error in `src-tauri/src/commands/scheduler.rs` — that is expected and will be fixed in Phase 2, Task 2.2.

- [x] **Step 1: Add the variant**

Full file after edit:

```rust
// crates/spectral-scheduler/src/jobs.rs
//! Job type definitions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum JobType {
    ScanAll,
    VerifyRemovals,
    PollImap,
    FollowUpReminders,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub job_type: JobType,
    pub interval_days: u32,
    pub next_run_at: String,
    pub last_run_at: Option<String>,
    pub enabled: bool,
}
```

- [x] **Step 2: Build spectral-scheduler only (ignore app compile error for now)**

```bash
cargo build -p spectral-scheduler 2>&1 | grep "^error"
```

Expected: no output

- [x] **Step 3: Commit**

```bash
git add crates/spectral-scheduler/src/jobs.rs
git commit -m "feat(scheduler): add FollowUpReminders job type"
```

> **Handoff note:** After this commit `cargo build -p spectral-app` will fail with a non-exhaustive match error in `scheduler.rs`. This is expected — it is resolved in Task 2.2.

---

## Phase 2: Backend

**Goal:** Add DB query functions, Tauri commands for the UI, and the scheduler job handler that auto-sends follow-ups.

**Phase Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting Task 2.1. Set to ✅ after Task 2.4 commits and `cargo build -p spectral-app` passes cleanly.

---

### Task 2.1 — `spectral-db`: `removal_followups` query module

**Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting. Set to ✅ after tests pass and commit succeeds.

**Files:**
- Create: `crates/spectral-db/src/removal_followups.rs`
- Modify: `crates/spectral-db/src/lib.rs`

**Context:** Follows the same pattern as other modules in `spectral-db/src/`. The `uuid` crate is already a dependency of `spectral-db`. Use `chrono::Utc::now().to_rfc3339()` for all timestamps. The `sqlx::query_as!` macro requires a compile-time database connection — if the project does not support that, use `sqlx::query_as` (without `!`) with explicit column bindings instead. Check existing modules in `crates/spectral-db/src/` to see which form is used.

- [x] **Step 1: Check which `query_as` form existing modules use**

```bash
grep -n "query_as!" crates/spectral-db/src/*.rs | head -5
grep -n "query_as(" crates/spectral-db/src/*.rs | head -5
```

Use whichever form the existing code uses. If `query_as!` (macro form) is used, the struct fields must exactly match column names. If `query_as` (function form) is used, map rows manually.

- [x] **Step 2: Write the module**

```rust
// crates/spectral-db/src/removal_followups.rs
//! Database operations for removal follow-up reminders.

use sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub struct RemovalFollowup {
    pub id: String,
    pub attempt_id: String,
    pub broker_id: String,
    pub recipient: String,
    pub follow_up_at: String,
    pub sent_at: Option<String>,
    pub dismissed_at: Option<String>,
    pub method: Option<String>,
}

/// Schedule a follow-up reminder for a removal attempt.
///
/// `follow_up_at` must be an RFC-3339 timestamp (e.g. `(Utc::now() + Duration::days(15)).to_rfc3339()`).
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
    let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>, Option<String>, Option<String>)>(
        r"SELECT id, attempt_id, broker_id, recipient, follow_up_at, sent_at, dismissed_at, method
          FROM removal_followups
          WHERE follow_up_at <= ? AND sent_at IS NULL AND dismissed_at IS NULL
          ORDER BY follow_up_at ASC",
    )
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
    let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>, Option<String>, Option<String>)>(
        r"SELECT id, attempt_id, broker_id, recipient, follow_up_at, sent_at, dismissed_at, method
          FROM removal_followups
          WHERE sent_at IS NULL AND dismissed_at IS NULL
          ORDER BY follow_up_at ASC",
    )
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

fn row_to_followup(
    r: (String, String, String, String, String, Option<String>, Option<String>, Option<String>),
) -> RemovalFollowup {
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

        let pending = get_pending_followups(&pool)
            .await
            .expect("pending"); // nosemgrep: no-unwrap-in-production
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

        let pending = get_pending_followups(&pool)
            .await
            .expect("pending"); // nosemgrep: no-unwrap-in-production
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_due_followups_filters_by_date() {
        let pool = make_pool().await;
        insert_stub_attempt(&pool, "att-3").await;
        insert_stub_attempt(&pool, "att-4").await;

        // Past date — should appear in due list
        schedule_followup(&pool, "att-3", "klaviyo", "privacy@klaviyo.com", "2000-01-01T00:00:00Z")
            .await
            .expect("past"); // nosemgrep: no-unwrap-in-production

        // Future date — should NOT appear in due list
        schedule_followup(&pool, "att-4", "klaviyo", "privacy@klaviyo.com", "2099-01-01T00:00:00Z")
            .await
            .expect("future"); // nosemgrep: no-unwrap-in-production

        let due = get_due_followups(&pool).await.expect("due"); // nosemgrep: no-unwrap-in-production
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].attempt_id, "att-3");
    }

    #[tokio::test]
    async fn test_mark_sent_removes_from_pending() {
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
```

- [x] **Step 3: Export from `spectral-db/src/lib.rs`**

Add after the last existing `pub mod` line in `crates/spectral-db/src/lib.rs`:

```rust
pub mod removal_followups;
pub use removal_followups::{
    dismiss_followup as dismiss_removal_followup, get_due_followups,
    get_pending_followups as get_pending_removal_followups, mark_followup_sent,
    schedule_followup, RemovalFollowup,
};
```

- [x] **Step 4: Run tests**

```bash
cargo test -p spectral-db removal_followups 2>&1 | tail -15
```

Expected:
```
test removal_followups::tests::test_dismiss_removes_from_pending ... ok
test removal_followups::tests::test_due_followups_filters_by_date ... ok
test removal_followups::tests::test_mark_sent_removes_from_pending ... ok
test removal_followups::tests::test_schedule_and_get_pending ... ok

test result: ok. 4 passed; 0 failed
```

- [x] **Step 5: Commit**

```bash
git add crates/spectral-db/src/removal_followups.rs crates/spectral-db/src/lib.rs
git commit -m "feat(db): add removal_followups query module with tests"
```

---

### Task 2.2 — Scheduler job handler: `FollowUpReminders`

**Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting. Set to ✅ after `cargo build -p spectral-app` passes and the commit succeeds.

**Files:**
- Modify: `src-tauri/src/commands/scheduler.rs`

**Context:** Task 1.3 left `spectral-app` with a non-exhaustive match error. This task fixes it by adding the `FollowUpReminders` arm and two new helper functions. The helpers follow the same pattern as `run_poll_imap_job` and `handle_llm_reply` already in this file. `SmtpConfig`, `CompletionRequest`, `TaskType`, `PrivacyAwareLlmRouter`, and `SqlitePool` are already imported.

- [x] **Step 1: Add the match arm to `run_job_now`**

In the `match job_type { ... }` block, add after the `PollImap` arm:

```rust
JobType::FollowUpReminders => {
    let db = vault.database().map_err(|e| {
        CommandError::new("DATABASE_ERROR", format!("Failed to get vault database: {e}"))
    })?;
    run_followup_reminders_job(db.pool()).await
}
```

- [x] **Step 2: Add `run_followup_reminders_job` helper**

Add this function after `run_poll_imap_job` and before `handle_llm_reply`:

```rust
async fn run_followup_reminders_job(pool: &SqlitePool) -> Result<(), CommandError> {
    info!("Executing FollowUpReminders job");

    let due = spectral_db::get_due_followups(pool).await.map_err(|e| {
        CommandError::new(
            "DATABASE_ERROR",
            format!("Failed to fetch due follow-ups: {e}"),
        )
    })?;

    if due.is_empty() {
        info!("No follow-ups due");
        return Ok(());
    }

    info!("{} follow-up(s) due", due.len());

    let smtp_config = spectral_mail::settings::get_smtp_config(pool)
        .await
        .ok()
        .flatten();
    let llm_available = spectral_privacy::get_primary_provider(pool)
        .await
        .ok()
        .flatten()
        .is_some();

    for followup in &due {
        if let (Some(ref smtp), true) = (&smtp_config, llm_available) {
            match send_auto_followup(followup, smtp, pool).await {
                Ok(()) => {
                    if let Err(e) =
                        spectral_db::mark_followup_sent(pool, &followup.id, "smtp_auto").await
                    {
                        tracing::warn!(
                            "Failed to mark follow-up {} as sent: {}",
                            followup.id,
                            e
                        );
                    } else {
                        info!(
                            "Auto-sent follow-up for attempt {} to {}",
                            followup.attempt_id, followup.recipient
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to auto-send follow-up for attempt {}: {}",
                        followup.attempt_id,
                        e
                    );
                    // Leave unsent — retried next daily run, surfaced in UI
                }
            }
        } else {
            info!(
                "Follow-up for attempt {} due but LLM/SMTP not configured — surfacing in UI",
                followup.attempt_id
            );
        }
    }

    Ok(())
}

/// Draft and send an automated follow-up email using the LLM, then log it.
async fn send_auto_followup(
    followup: &spectral_db::RemovalFollowup,
    smtp: &SmtpConfig,
    pool: &SqlitePool,
) -> Result<(), String> {
    let router = PrivacyAwareLlmRouter::new(pool.clone());

    let prompt = format!(
        "Draft a short, professional follow-up email (3-5 sentences) for a data deletion \
        request sent 15 days ago to {} that has not yet been confirmed. Reference the original \
        CCPA/GDPR data deletion request. Ask for a status update and confirmation of deletion. \
        Provide only the email body — no subject line.",
        followup.recipient
    );

    let request = CompletionRequest::new(prompt).with_max_tokens(256);
    let response = router
        .route(TaskType::EmailDraft, request)
        .await
        .map_err(|e| format!("LLM request failed: {e}"))?;

    let body = response.content.trim().to_string();

    let template = spectral_mail::templates::EmailTemplate {
        to: followup.recipient.clone(),
        subject: "Follow-Up: Data Deletion Request".to_string(),
        body: body.clone(),
    };

    spectral_mail::sender::send_smtp(&template, &smtp.username, smtp, None)
        .await
        .map_err(|e| format!("SMTP send failed: {e}"))?;

    let now = chrono::Utc::now().to_rfc3339();
    let body_hash = spectral_mail::sender::body_hash(&body);
    sqlx::query(
        "INSERT INTO email_removals \
         (id, attempt_id, broker_id, recipient, method, subject, body_hash, sent_at) \
         VALUES (lower(hex(randomblob(16))), ?, ?, ?, 'smtp_followup', \
                 'Follow-Up: Data Deletion Request', ?, ?)",
    )
    .bind(&followup.attempt_id)
    .bind(&followup.broker_id)
    .bind(&followup.recipient)
    .bind(&body_hash)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to log follow-up to email_removals: {e}"))?;

    Ok(())
}
```

- [x] **Step 3: Build check — app must now compile cleanly**

```bash
cargo build -p spectral-app 2>&1 | grep "^error"
```

Expected: no output

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/commands/scheduler.rs
git commit -m "feat(scheduler): implement FollowUpReminders job with LLM auto-send"
```

---

### Task 2.3 — Tauri commands: `get_pending_followups` + `dismiss_followup`

**Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting. Set to ✅ after `cargo build -p spectral-app` passes and the commit succeeds.

**Files:**
- Modify: `src-tauri/src/commands/removal.rs`
- Modify: `src-tauri/src/lib.rs`

**Context:** `removal.rs` already has `submit_removal` and `mark_attempt_verified`. Add the two new commands after those. The `FollowupDto` is needed because `RemovalFollowup` is a crate-internal type; the DTO exposes only what the frontend needs (no `sent_at`, `dismissed_at`, `method` — those are backend-only).

- [x] **Step 1: Add imports and structs to `removal.rs`**

At the top of `src-tauri/src/commands/removal.rs`, ensure these imports are present (add any missing):

```rust
use crate::error::CommandError;
use crate::state::AppState;
use spectral_db::RemovalFollowup;
```

- [x] **Step 2: Add DTO and commands to `removal.rs`**

Append after the existing commands:

```rust
/// Follow-up reminder data returned to the frontend.
#[derive(serde::Serialize)]
pub struct FollowupDto {
    pub id: String,
    pub attempt_id: String,
    pub broker_id: String,
    pub recipient: String,
    /// ISO-8601 timestamp: when the follow-up is due.
    pub follow_up_at: String,
}

impl From<RemovalFollowup> for FollowupDto {
    fn from(f: RemovalFollowup) -> Self {
        Self {
            id: f.id,
            attempt_id: f.attempt_id,
            broker_id: f.broker_id,
            recipient: f.recipient,
            follow_up_at: f.follow_up_at,
        }
    }
}

#[tauri::command]
pub async fn get_pending_followups(
    vault_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FollowupDto>, CommandError> {
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new("VAULT_NOT_UNLOCKED", format!("Vault {vault_id} not unlocked"))
    })?;
    let db = vault.database().map_err(|e| {
        CommandError::new("DATABASE_ERROR", format!("Failed to access database: {e}"))
    })?;

    spectral_db::get_pending_removal_followups(db.pool())
        .await
        .map(|rows| rows.into_iter().map(FollowupDto::from).collect())
        .map_err(|e| {
            CommandError::new("DATABASE_ERROR", format!("Failed to get follow-ups: {e}"))
        })
}

#[tauri::command]
pub async fn dismiss_followup(
    vault_id: String,
    followup_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault = state.get_vault(&vault_id).ok_or_else(|| {
        CommandError::new("VAULT_NOT_UNLOCKED", format!("Vault {vault_id} not unlocked"))
    })?;
    let db = vault.database().map_err(|e| {
        CommandError::new("DATABASE_ERROR", format!("Failed to access database: {e}"))
    })?;

    spectral_db::dismiss_removal_followup(db.pool(), &followup_id)
        .await
        .map_err(|e| {
            CommandError::new("DATABASE_ERROR", format!("Failed to dismiss follow-up: {e}"))
        })
}
```

- [x] **Step 3: Register commands in `src-tauri/src/lib.rs`**

In the `tauri::generate_handler![` block, add:

```rust
commands::removal::get_pending_followups,
commands::removal::dismiss_followup,
```

- [x] **Step 4: Build check**

```bash
cargo build -p spectral-app 2>&1 | grep "^error"
```

Expected: no output

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/commands/removal.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add get_pending_followups and dismiss_followup Tauri commands"
```

---

### Task 2.4 — Phase 2 verification

**Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting. Set to ✅ after all checks pass and the push succeeds.

- [x] **Step 1: Full build**

```bash
cargo build -p spectral-app 2>&1 | grep "^error"
```

Expected: no output

- [x] **Step 2: Run DB tests**

```bash
cargo test -p spectral-db 2>&1 | tail -15
```

Expected: all tests pass including the 4 new `removal_followups` tests

- [x] **Step 3: Push**

```bash
git push
```

> **Handoff note for next session:** Phase 2 is complete. Phase 3 (wiring follow-up creation into email send paths) can begin. Read `src-tauri/src/removal_worker.rs` (the `submit_via_email` function) and `src-tauri/src/commands/scan.rs` (the `send_removal_email` command, search for `INSERT INTO email_removals`) to find the exact insertion points.

---

## Phase 3: Email Send Wiring

**Goal:** Every time a removal email is sent (either via the batch worker or the single-send command), schedule a follow-up 15 days out in `removal_followups`.

**Phase Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting Task 3.1. Set to ✅ after Task 3.2 commits and `cargo build -p spectral-app` passes.

---

### Task 3.1 — Wire follow-up into `submit_via_email` (batch worker)

**Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting. Set to ✅ after the commit succeeds.

**Files:**
- Modify: `src-tauri/src/removal_worker.rs`

**Context:** `submit_via_email` in `removal_worker.rs` sends the removal email and then inserts into `email_removals`. Find that INSERT block — it is the right insertion point. The function has access to `attempt_id`, `broker_def.broker.id`, and the broker email (`to_email` from the `RemovalMethod::Email` destructure). `spectral_db::schedule_followup` is available after Phase 2. The follow-up is non-fatal — log a warning and continue if it fails.

- [x] **Step 1: Find the exact insertion point**

```bash
grep -n "INSERT INTO email_removals\|email_removal_id\|body_hash" src-tauri/src/removal_worker.rs | head -10
```

Note the line number of the `.execute(db.pool()).await` call that follows the `email_removals` INSERT.

- [x] **Step 2: Add follow-up scheduling after the INSERT**

Immediately after the `.execute(db.pool()).await?;` line of the `email_removals` INSERT, add:

```rust
// Schedule a 15-day follow-up reminder
let follow_up_at = (chrono::Utc::now() + chrono::Duration::days(15)).to_rfc3339();
if let Err(e) = spectral_db::schedule_followup(
    db.pool(),
    attempt_id,
    &broker_def.broker.id.to_string(),
    to_email,
    &follow_up_at,
)
.await
{
    tracing::warn!(
        "Failed to schedule follow-up for attempt {}: {}",
        attempt_id,
        e
    );
}
```

- [x] **Step 3: Build check**

```bash
cargo build -p spectral-app 2>&1 | grep "^error"
```

Expected: no output

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/removal_worker.rs
git commit -m "feat(worker): schedule 15-day follow-up after removal email is sent"
```

---

### Task 3.2 — Wire follow-up into `send_removal_email` (single-send command)

**Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting. Set to ✅ after `cargo build -p spectral-app` passes and the commit succeeds.

**Files:**
- Modify: `src-tauri/src/commands/scan.rs`

**Context:** `send_removal_email` is a Tauri command in `scan.rs` starting at line ~1417. It loads a `RemovalEmailContext` (which has `broker_id: String` and `broker_email: String` fields) and inserts into `email_removals`. Find that INSERT and add the follow-up scheduling call directly after it. `attempt_id` is a parameter of the command. `db.pool()` is available.

- [x] **Step 1: Find the exact insertion point**

```bash
grep -n "INSERT INTO email_removals\|broker_email\|follow_up" src-tauri/src/commands/scan.rs | head -15
```

Note the line of the `email_removals` INSERT execute call.

- [x] **Step 2: Add follow-up scheduling after the INSERT**

Immediately after the `.execute(db.pool()).await?;` line of the `email_removals` INSERT:

```rust
// Schedule a 15-day follow-up reminder
let follow_up_at = (chrono::Utc::now() + chrono::Duration::days(15)).to_rfc3339();
if let Err(e) = spectral_db::schedule_followup(
    db.pool(),
    &attempt_id,
    &context.broker_id,
    &context.broker_email,
    &follow_up_at,
)
.await
{
    tracing::warn!(
        "Failed to schedule follow-up for attempt {}: {}",
        attempt_id,
        e
    );
}
```

- [x] **Step 3: Build check**

```bash
cargo build -p spectral-app 2>&1 | grep "^error"
```

Expected: no output

- [x] **Step 4: Commit + push**

```bash
git add src-tauri/src/commands/scan.rs
git commit -m "feat(commands): schedule 15-day follow-up after manual removal email send"
git push
```

> **Handoff note for next session:** Phase 3 is complete. Phase 4 (frontend notification component) can begin. Read `src/routes/removals/+page.svelte` to understand the page structure and find the `vaultId` variable name and where the job list loop (`{#each jobs`) begins.

---

## Phase 4: Frontend

**Goal:** Show follow-up notifications on the removal history page — overdue items get an amber badge, upcoming ones get a blue info card, and users can dismiss each one.

**Phase Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting Task 4.1. Set to ✅ after Task 4.3 commits, `npm run check` passes, and the push succeeds.

---

### Task 4.1 — TypeScript API wrappers

**Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting. Set to ✅ after the commit succeeds.

**Files:**
- Create: `src/lib/api/followups.ts`

**Context:** Follows the same pattern as `src/lib/api/settings.ts` — thin wrappers around `invoke`. The `PendingFollowup` interface must match the `FollowupDto` Rust struct fields exactly: `id`, `attempt_id`, `broker_id`, `recipient`, `follow_up_at`.

- [x] **Step 1: Write the module**

```typescript
// src/lib/api/followups.ts
import { invoke } from '@tauri-apps/api/core';

export interface PendingFollowup {
    id: string;
    attempt_id: string;
    broker_id: string;
    /** Email address the follow-up will be sent to. */
    recipient: string;
    /** ISO-8601 datetime when the follow-up is due. */
    follow_up_at: string;
}

/** Return all pending (unsent, undismissed) follow-ups for the vault. */
export async function getPendingFollowups(vaultId: string): Promise<PendingFollowup[]> {
    return invoke('get_pending_followups', { vaultId });
}

/** Mark a follow-up as dismissed (user handled it manually). */
export async function dismissFollowup(vaultId: string, followupId: string): Promise<void> {
    return invoke('dismiss_followup', { vaultId, followupId });
}
```

- [x] **Step 2: Type check**

```bash
npm run check 2>&1 | grep -E "^Error|error TS" | head -10
```

Expected: no errors

- [x] **Step 3: Commit**

```bash
git add src/lib/api/followups.ts
git commit -m "feat(api): add getPendingFollowups and dismissFollowup TypeScript wrappers"
```

---

### Task 4.2 — `FollowUpNotifications` component

**Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting. Set to ✅ after the commit succeeds.

**Files:**
- Create: `src/lib/components/removals/FollowUpNotifications.svelte`

**Context:** Uses Svelte 5 runes (`$state`, `$effect`, `$props`). Follows badge/color patterns used throughout the removal dashboard: amber for overdue, blue for upcoming. Each card shows: broker name, recipient email, due date, overdue hint (if past due), and a Dismiss button. When all are dismissed, the section disappears entirely (`{#if followups.length > 0}`).

- [x] **Step 1: Write the component**

```svelte
<!-- src/lib/components/removals/FollowUpNotifications.svelte -->
<script lang="ts">
    import { getPendingFollowups, dismissFollowup, type PendingFollowup } from '$lib/api/followups';

    interface Props {
        vaultId: string;
    }

    let { vaultId }: Props = $props();

    let followups = $state<PendingFollowup[]>([]);
    let loading = $state(true);
    let dismissing = $state<Set<string>>(new Set());

    $effect(() => {
        load();
    });

    async function load() {
        try {
            followups = await getPendingFollowups(vaultId);
        } catch (e) {
            console.error('Failed to load follow-ups:', e);
        } finally {
            loading = false;
        }
    }

    async function handleDismiss(id: string) {
        dismissing = new Set([...dismissing, id]);
        try {
            await dismissFollowup(vaultId, id);
            followups = followups.filter((f) => f.id !== id);
        } catch (e) {
            console.error('Failed to dismiss follow-up:', e);
        } finally {
            dismissing = new Set([...dismissing].filter((x) => x !== id));
        }
    }

    function isOverdue(followUpAt: string): boolean {
        return new Date(followUpAt) <= new Date();
    }

    function formatDate(iso: string): string {
        return new Date(iso).toLocaleDateString(undefined, {
            month: 'short',
            day: 'numeric',
            year: 'numeric'
        });
    }

    function brokerLabel(brokerId: string): string {
        return brokerId.charAt(0).toUpperCase() + brokerId.slice(1).replace(/-/g, ' ');
    }
</script>

{#if !loading && followups.length > 0}
    <div class="mb-6 space-y-3">
        <h2 class="text-sm font-semibold uppercase tracking-wide text-gray-700">
            Follow-Up Reminders
            <span
                class="ml-2 inline-flex items-center justify-center rounded-full bg-amber-500 px-2 py-0.5 text-xs font-bold text-white"
            >
                {followups.length}
            </span>
        </h2>

        {#each followups as followup (followup.id)}
            {@const overdue = isOverdue(followup.follow_up_at)}
            <div
                class="flex items-start justify-between rounded-lg border p-4 {overdue
                    ? 'border-amber-300 bg-amber-50'
                    : 'border-blue-200 bg-blue-50'}"
            >
                <div class="min-w-0 flex-1">
                    <div class="mb-1 flex items-center gap-2">
                        <span
                            class="inline-block rounded-full px-2 py-0.5 text-xs font-medium {overdue
                                ? 'bg-amber-200 text-amber-900'
                                : 'bg-blue-200 text-blue-900'}"
                        >
                            {overdue ? 'Follow-up overdue' : 'Follow-up scheduled'}
                        </span>
                        <span class="text-xs text-gray-500">{brokerLabel(followup.broker_id)}</span>
                    </div>

                    <p class="text-sm text-gray-800">
                        {overdue
                            ? `Your removal request to ${followup.recipient} needs a follow-up — due ${formatDate(followup.follow_up_at)}.`
                            : `Scheduled to follow up with ${followup.recipient} on ${formatDate(followup.follow_up_at)}.`}
                    </p>

                    {#if overdue}
                        <p class="mt-1 text-xs text-amber-800">
                            Connect SMTP + an LLM provider in Settings → Email to have Spectral
                            send this follow-up automatically.
                        </p>
                    {/if}
                </div>

                <button
                    onclick={() => handleDismiss(followup.id)}
                    disabled={dismissing.has(followup.id)}
                    class="ml-4 shrink-0 rounded px-3 py-1.5 text-xs font-medium transition-colors disabled:opacity-50
                           {overdue
                               ? 'bg-amber-200 text-amber-900 hover:bg-amber-300'
                               : 'bg-blue-200 text-blue-900 hover:bg-blue-300'}"
                >
                    {dismissing.has(followup.id) ? 'Dismissing…' : 'Dismiss'}
                </button>
            </div>
        {/each}
    </div>
{/if}
```

- [x] **Step 2: Type check**

```bash
npm run check 2>&1 | grep -E "^Error|error TS" | head -10
```

Expected: no errors

- [x] **Step 3: Commit**

```bash
git add src/lib/components/removals/FollowUpNotifications.svelte
git commit -m "feat(ui): add FollowUpNotifications component with overdue badge and dismiss"
```

---

### Task 4.3 — Wire component into removal history page

**Status:** ✅ Complete
> **Instructions for Claude:** Set to 🔄 when starting. Set to ✅ after `npm run check` passes, the commit succeeds, and the push succeeds.

**Files:**
- Modify: `src/routes/removals/+page.svelte`

**Context:** Read the full page first. Find: (1) the `vaultId` variable name — it may come from `$page.params`, a store, or a prop; (2) the `{#each jobs` loop — the component goes immediately above it, inside the same container div.

- [x] **Step 1: Read the page to find the insertion point**

```bash
head -60 src/routes/removals/+page.svelte
grep -n "vaultId\|vault_id\|{#each jobs\|{#each" src/routes/removals/+page.svelte | head -15
```

Note: the exact variable name holding the vault ID, and the line number of the `{#each` loop.

- [x] **Step 2: Add the import to `<script>`**

In the `<script>` block, add:

```typescript
import FollowUpNotifications from '$lib/components/removals/FollowUpNotifications.svelte';
```

- [x] **Step 3: Add the component above the job list**

Immediately before the `{#each jobs` line, add:

```svelte
<FollowUpNotifications vaultId={YOUR_VAULT_ID_VAR} />
```

Replace `YOUR_VAULT_ID_VAR` with the actual variable name found in Step 1.

- [x] **Step 4: Type check**

```bash
npm run check 2>&1 | grep -E "^Error|error TS" | head -10
```

Expected: no errors

- [x] **Step 5: Full build**

```bash
cargo build -p spectral-app 2>&1 | grep "^error"
cargo test -p spectral-db 2>&1 | tail -10
```

Expected: zero errors, all tests pass

- [x] **Step 6: Commit + push**

```bash
git add src/routes/removals/+page.svelte
git commit -m "feat(ui): show follow-up reminders on removal history page"
git push
```

> **Final state:** All 4 phases complete. The follow-up reminder system is fully operational. Run a SonarQube scan to verify 0 issues before merging.

---

## Status Legend

| Symbol | Meaning |
|--------|---------|
| ⬜ | Not Started |
| 🔄 | In Progress |
| ✅ | Complete |
| ❌ | Blocked — see note |

> **Instructions for Claude (global):** After completing any task, immediately update its **Task Status** line and its parent **Phase Status** line. After completing a phase, update the **Phase Tracking** table at the top of this document. This file is the single source of truth for progress across context windows.
