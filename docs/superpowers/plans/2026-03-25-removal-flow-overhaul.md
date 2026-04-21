# Removal Flow Overhaul Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Enable every broker/adtech entry to have an actionable removal button — scan for those with URL templates, send email or submit form for those without — plus a "Remove All" bulk action that works across all broker types.

**Architecture:** The core blocker is that `removal_attempts.finding_id` is `NOT NULL`, meaning removals can only be initiated after a scan produces findings. We break this coupling via a DB migration (nullable `finding_id` + new `profile_id` column), then add backend commands that create and process removal attempts directly from a broker + profile pair without a prior scan. The frontend gains a smart action button that inspects `search_method_type` to show the right action (Scan / Submit Form / Send Email / View Instructions), and a "Remove All" button on the list page.

**Tech Stack:** Rust/SQLite/sqlx (spectral-db), Tauri commands (src-tauri), Svelte 5 runes (frontend), Tailwind CSS

---

## Phase Tracking

| Phase | Title | Status |
|-------|-------|--------|
| Phase 1 | Data Layer — Decouple Removals from Findings | ✅ Complete |
| Phase 2 | Backend Commands — Broker Action API | ✅ Complete |
| Phase 3 | Smart Action UI — Per-Broker Action Buttons | ✅ Complete |
| Phase 4 | Bulk Removal — Remove All | ✅ Complete |

> **Instructions for Claude:** Update the Status column above as you complete phases. Use: ⬜ Not Started → 🔄 In Progress → ✅ Complete → ❌ Blocked (add note)

---

## File Map

### New Files
| File | Purpose |
|------|---------|
| `crates/spectral-db/migrations/022_standalone_removal_attempts.sql` | Make `finding_id` nullable, add `profile_id` column |
| `src/lib/components/removals/RemovalActionButton.svelte` | Smart action button component — renders correct action based on broker type |

### Modified Files
| File | What Changes |
|------|-------------|
| `crates/spectral-db/src/removal_attempts.rs` | `finding_id: Option<String>`, new `profile_id: Option<String>` field, new `create_standalone_removal_attempt()` fn |
| `src-tauri/src/removal_worker.rs` | Handle `None` finding — load profile from `removal_attempt.profile_id` when no finding exists |
| `src-tauri/src/commands/brokers.rs` | Add `SearchMethodType` enum, `search_method_type` field on `BrokerDetail`, add `removal_action_url` field |
| `src-tauri/src/commands/scan.rs` | Add `initiate_direct_removal` command, add `initiate_bulk_removal` command |
| `src-tauri/src/lib.rs` | Register two new Tauri commands |
| `src/lib/api/brokers.ts` | Add `search_method_type` and `removal_action_url` to `BrokerDetail` interface |
| `src/lib/api/removal.ts` | Add `initiateDirect()` and `initiateBulk()` methods |
| `src/routes/adtech/[adtechId]/+page.svelte` | Replace static "Scan This Company" button with `<RemovalActionButton>` |
| `src/routes/adtech/+page.svelte` | Add "Remove All" button and bulk progress display |

---

## Phase 1: Data Layer — Decouple Removals from Findings

**Goal:** Allow `removal_attempts` rows to exist without a corresponding finding — required for email, form, and manual brokers that can never produce scan findings.

**Phase Status:** ✅ Complete

### Task 1.1 — DB Migration: Nullable `finding_id` + `profile_id` Column

**Files:**
- Create: `crates/spectral-db/migrations/022_standalone_removal_attempts.sql`

**Context:** `removal_attempts` currently has `finding_id TEXT NOT NULL` with a FK to `findings`. SQLite cannot `ALTER COLUMN` to drop `NOT NULL` — you must recreate the table. The migration recreates it with `finding_id` nullable and adds a `profile_id` column so standalone removals know which profile they belong to.

**Task Status:** ✅ Complete

- [x] **Step 1: Create the migration file**

```sql
-- crates/spectral-db/migrations/022_standalone_removal_attempts.sql
-- Recreate removal_attempts with nullable finding_id and new profile_id column.
-- Required so removal attempts can be created without a scan finding (e.g. email-method brokers).

-- Step 1: Create new table with desired schema
CREATE TABLE removal_attempts_new (
    id TEXT PRIMARY KEY,
    finding_id TEXT,
    profile_id TEXT,
    broker_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('Pending', 'Submitted', 'Completed', 'Failed')),
    created_at TEXT NOT NULL,
    submitted_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    FOREIGN KEY (finding_id) REFERENCES findings(id) ON DELETE CASCADE
);

-- Step 2: Migrate existing rows — populate profile_id from the linked finding
INSERT INTO removal_attempts_new
    (id, finding_id, profile_id, broker_id, status, created_at, submitted_at, completed_at, error_message)
SELECT
    ra.id,
    ra.finding_id,
    f.profile_id,
    ra.broker_id,
    ra.status,
    ra.created_at,
    ra.submitted_at,
    ra.completed_at,
    ra.error_message
FROM removal_attempts ra
LEFT JOIN findings f ON f.id = ra.finding_id;

-- Step 3: Swap tables
DROP TABLE removal_attempts;
ALTER TABLE removal_attempts_new RENAME TO removal_attempts;

-- Step 4: Recreate indexes
CREATE INDEX idx_removal_attempts_finding ON removal_attempts(finding_id);
CREATE INDEX idx_removal_attempts_status ON removal_attempts(status);
CREATE INDEX idx_removal_attempts_created_at ON removal_attempts(created_at DESC);
CREATE INDEX idx_removal_attempts_profile ON removal_attempts(profile_id);
```

- [x] **Step 2: Verify migration compiles by running existing tests**

```bash
cd /home/evan/projects/spectral
cargo test -p spectral-db 2>&1 | tail -20
```

Expected: All tests pass (migration is run automatically in test DB setup via `db.run_migrations()`).

- [x] **Step 3: Commit**

```bash
git add crates/spectral-db/migrations/022_standalone_removal_attempts.sql
git commit -m "chore(db): add migration for nullable finding_id and profile_id on removal_attempts"
```

---

### Task 1.2 — Update `removal_attempts.rs` DB Module

**Files:**
- Modify: `crates/spectral-db/src/removal_attempts.rs`

**Context:** The `RemovalAttempt` struct has `finding_id: String` (non-optional). Change it to `Option<String>`, add `profile_id: Option<String>`, update all query row-mapping accordingly, and add a new `create_standalone_removal_attempt` function.

**Task Status:** ✅ Complete

- [x] **Step 1: Write a failing test for `create_standalone_removal_attempt`**

Add this test at the bottom of the `#[cfg(test)] mod tests` block in `crates/spectral-db/src/removal_attempts.rs`:

```rust
#[tokio::test]
async fn test_create_standalone_removal_attempt() {
    let db = setup_test_db().await;

    let attempt = create_standalone_removal_attempt(
        db.pool(),
        "test-broker",
        "profile-456",
    )
    .await
    .expect("should create standalone attempt");

    assert_eq!(attempt.broker_id, "test-broker");
    assert!(attempt.finding_id.is_none(), "standalone attempt should have no finding_id");
    assert_eq!(attempt.profile_id.as_deref(), Some("profile-456"));
    assert_eq!(attempt.status, RemovalStatus::Pending);
}
```

- [x] **Step 2: Run test to verify it fails**

```bash
cargo test -p spectral-db test_create_standalone_removal_attempt 2>&1 | tail -10
```

Expected: compile error — `create_standalone_removal_attempt` does not exist yet.

- [x] **Step 3: Update `RemovalAttempt` struct and all row-mappers**

In `crates/spectral-db/src/removal_attempts.rs`, change the struct:

```rust
pub struct RemovalAttempt {
    pub id: String,
    pub finding_id: Option<String>,  // was String, now Option
    pub profile_id: Option<String>,  // new field
    pub broker_id: String,
    pub status: RemovalStatus,
    pub created_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}
```

Update every SQL query that selects `removal_attempts` columns to include `profile_id`. The SELECT column list used in `get_by_id`, `get_by_finding_id`, `get_captcha_queue`, `get_failed_queue`, `get_by_scan_job_id` should become:

```sql
SELECT id, finding_id, profile_id, broker_id, status, created_at, submitted_at, completed_at, error_message
FROM removal_attempts
...
```

Update every row-to-struct mapping to read both new nullable columns:

```rust
RemovalAttempt {
    id: row.try_get("id")?,
    finding_id: row.try_get("finding_id")?,       // Option<String>
    profile_id: row.try_get("profile_id")?,       // Option<String> — new
    broker_id: row.try_get("broker_id")?,
    status: /* existing parsing */,
    created_at: /* existing */,
    submitted_at: /* existing */,
    completed_at: /* existing */,
    error_message: row.try_get("error_message")?,
}
```

Also update `create_removal_attempt` (the existing function) INSERT to include `profile_id`:

```rust
pub async fn create_removal_attempt(
    pool: &Pool<Sqlite>,
    finding_id: &str,
    broker_id: &str,
    profile_id: &str,
) -> Result<RemovalAttempt, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO removal_attempts (id, finding_id, profile_id, broker_id, status, created_at) VALUES (?, ?, ?, ?, 'Pending', ?)"
    )
    .bind(&id)
    .bind(finding_id)
    .bind(profile_id)
    .bind(broker_id)
    .bind(&created_at)
    .execute(pool)
    .await?;

    Ok(RemovalAttempt {
        id,
        finding_id: Some(finding_id.to_string()),
        profile_id: Some(profile_id.to_string()),
        broker_id: broker_id.to_string(),
        status: RemovalStatus::Pending,
        created_at: Utc::now(),
        submitted_at: None,
        completed_at: None,
        error_message: None,
    })
}
```

Add the new standalone function:

```rust
/// Create a removal attempt without an associated finding.
/// Used for brokers that cannot be auto-scanned (email, web-form, manual methods).
pub async fn create_standalone_removal_attempt(
    pool: &Pool<Sqlite>,
    broker_id: &str,
    profile_id: &str,
) -> Result<RemovalAttempt, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO removal_attempts (id, finding_id, profile_id, broker_id, status, created_at) VALUES (?, NULL, ?, ?, 'Pending', ?)"
    )
    .bind(&id)
    .bind(profile_id)
    .bind(broker_id)
    .bind(&created_at)
    .execute(pool)
    .await?;

    Ok(RemovalAttempt {
        id,
        finding_id: None,
        profile_id: Some(profile_id.to_string()),
        broker_id: broker_id.to_string(),
        status: RemovalStatus::Pending,
        created_at: Utc::now(),
        submitted_at: None,
        completed_at: None,
        error_message: None,
    })
}
```

- [x] **Step 4: Check all callers of `create_removal_attempt` in the codebase**

```bash
grep -rn "create_removal_attempt" /home/evan/projects/spectral/src-tauri/
```

Find each call site in `scan.rs` (the `submit_removals_for_confirmed` function). Each call must now pass `profile_id`. The scan has the finding, which has `finding.profile_id`. Update each call:

```rust
// Before:
spectral_db::removal_attempts::create_removal_attempt(
    db.pool(), &finding.id, &finding.broker_id
).await?;

// After:
spectral_db::removal_attempts::create_removal_attempt(
    db.pool(), &finding.id, &finding.broker_id, &finding.profile_id
).await?;
```

- [x] **Step 5: Run tests**

```bash
cargo test -p spectral-db 2>&1 | tail -20
cargo test -p spectral-scanner 2>&1 | tail -20
```

Expected: All pass including the new `test_create_standalone_removal_attempt`.

- [x] **Step 6: Commit**

```bash
git add crates/spectral-db/src/removal_attempts.rs
git commit -m "feat(db): support standalone removal attempts without scan findings"
```

---

### Task 1.3 — Update `removal_worker.rs` to Handle `None` Finding

**Files:**
- Modify: `src-tauri/src/removal_worker.rs`

**Context:** `submit_removal_task` currently does:
```rust
let finding = spectral_db::findings::get_by_id(db.pool(), &removal_attempt.finding_id)...
let profile_id = ... finding.profile_id ...
let listing_url = finding.listing_url;
```
When `finding_id` is `None`, this must fall back to `removal_attempt.profile_id` and use an empty `listing_url`.

**Task Status:** ✅ Complete

- [x] **Step 1: Update the finding + profile loading section in `submit_removal_task`**

Locate the section in `src-tauri/src/removal_worker.rs` that starts around line 484:
```rust
// Load associated finding
let finding = spectral_db::findings::get_by_id(db.pool(), &removal_attempt.finding_id)
```

Replace from that line through the `map_fields_for_submission` call (approximately lines 484–505) with:

```rust
// Load associated finding — may be None for standalone (email/form/manual) removals
let finding = match &removal_attempt.finding_id {
    Some(finding_id) => {
        Some(
            spectral_db::findings::get_by_id(db.pool(), finding_id)
                .await
                .map_err(|e| format!("Failed to load finding: {e}"))?
                .ok_or_else(|| format!("Finding not found: {finding_id}"))?,
        )
    }
    None => None,
};

// Resolve profile ID — from finding if present, otherwise from the attempt itself
let profile_id_str = match &finding {
    Some(f) => f.profile_id.clone(),
    None => removal_attempt
        .profile_id
        .clone()
        .ok_or_else(|| "Removal attempt has no finding_id and no profile_id".to_string())?,
};

let profile_id = spectral_core::types::ProfileId::new(&profile_id_str)
    .map_err(|e| format!("Invalid profile ID: {e}"))?;

let profile = vault
    .load_profile(&profile_id)
    .await
    .map_err(|e| format!("Failed to load profile: {e}"))?;

let key = vault
    .encryption_key()
    .map_err(|e| format!("Failed to get encryption key: {e}"))?;

// listing_url is empty for standalone removals (no scan finding)
let listing_url = finding.as_ref().map_or("", |f| f.listing_url.as_str());
let field_values = map_fields_for_submission(&profile, listing_url, key)?;
```

- [x] **Step 2: Build the project to check for compile errors**

```bash
cargo build -p spectral 2>&1 | grep -E "^error" | head -20
```

Expected: No errors. Fix any type mismatches if present (e.g., `removal_attempt.finding_id` is now `Option<String>` — any `&removal_attempt.finding_id` usage used as `&str` will need `.as_deref().unwrap_or("")`).

- [x] **Step 3: Run all tests**

```bash
cargo test 2>&1 | grep -E "FAILED|error\[" | head -20
```

Expected: No failures.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/removal_worker.rs
git commit -m "fix(worker): handle standalone removal attempts that have no associated finding"
```

---

### Task 1.4 — Phase 1 SonarQube Scan

**Task Status:** ✅ Complete

- [x] **Step 1: Run SonarQube scan**

```bash
cd /home/evan/projects/spectral
sonar-scanner \
  -Dsonar.projectKey=spectral \
  -Dsonar.sources=. \
  -Dsonar.host.url=http://localhost:9000 \
  -Dsonar.login=$SONAR_TOKEN 2>&1 | tail -30
```

If `sonar-scanner` is not on PATH, ask the user for the correct command.

- [x] **Step 2: Review results**

Open the SonarQube dashboard. Address any **Blocker** or **Critical** issues in the files touched by Phase 1:
- `crates/spectral-db/migrations/022_standalone_removal_attempts.sql`
- `crates/spectral-db/src/removal_attempts.rs`
- `src-tauri/src/removal_worker.rs`

- [x] **Step 3: Update Phase Tracking table**

Change Phase 1 status to ✅ Complete in the Phase Tracking table at the top of this document.

---

## Phase 2: Backend Commands — Broker Action API

**Goal:** Expose `search_method_type` on `BrokerDetail` so the frontend knows what action button to show, and add two new Tauri commands: `initiate_direct_removal` (one broker at a time) and `initiate_bulk_removal` (all non-scannable brokers at once).

**Phase Status:** ✅ Complete

### Task 2.1 — Add `SearchMethodType` and Enhance `BrokerDetail`

**Files:**
- Modify: `src-tauri/src/commands/brokers.rs`

**Context:** `BrokerDetail.removal_method` is currently a debug-formatted string (e.g., `"Email { email: \"...\", ... }"`). We need a clean `search_method_type` string enum and `removal_action_url` field so the frontend can decide what button to show and where to link.

**Task Status:** ✅ Complete

- [x] **Step 1: Add `SearchMethodType` enum and update `BrokerDetail`**

In `src-tauri/src/commands/brokers.rs`, add after the imports:

```rust
/// Indicates what kind of automated action is possible for a broker's search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMethodType {
    /// URL template — can auto-scan this broker
    Scannable,
    /// Web form — automation possible but requires form interaction
    WebForm,
    /// Manual — no automated search; user must act directly
    Manual,
}
```

Update `BrokerDetail` to add two new fields:

```rust
pub struct BrokerDetail {
    #[serde(flatten)]
    pub summary: BrokerSummary,
    pub removal_method: String,
    pub url: String,
    pub recheck_interval_days: u32,
    pub last_verified: String,
    pub scan_status: Option<String>,
    pub finding_count: Option<i64>,
    pub email_template: Option<EmailTemplate>,
    pub email_fallback: Option<EmailFallbackResponse>,
    /// Whether this broker can be auto-scanned or requires manual action
    pub search_method_type: SearchMethodType,
    /// URL for the removal action (opt-out form URL, or privacy page for manual)
    pub removal_action_url: Option<String>,
}
```

- [x] **Step 2: Populate new fields in `get_broker_detail`**

In the `get_broker_detail` command, after all the existing field population, add:

```rust
use spectral_broker::definition::{SearchMethod, RemovalMethod};

let search_method_type = match &def.search {
    SearchMethod::UrlTemplate { .. } => SearchMethodType::Scannable,
    SearchMethod::WebForm { .. } => SearchMethodType::WebForm,
    SearchMethod::Manual { .. } => SearchMethodType::Manual,
};

let removal_action_url = match &def.removal {
    RemovalMethod::WebForm { url, .. } => Some(url.clone()),
    RemovalMethod::BrowserForm { url, .. } => Some(url.clone()),
    RemovalMethod::Manual { .. } => Some(def.broker.url.clone()),
    RemovalMethod::Email { .. } => None,
    RemovalMethod::Phone { .. } => None,
};
```

And include them in the returned `BrokerDetail`:

```rust
Ok(BrokerDetail {
    summary: BrokerSummary::from(def),
    removal_method: format!("{:?}", def.removal),
    url: def.broker.url.clone(),
    recheck_interval_days: def.broker.recheck_interval_days,
    last_verified: def.broker.last_verified.to_string(),
    scan_status,
    finding_count,
    email_template,
    email_fallback,
    search_method_type,
    removal_action_url,
})
```

- [x] **Step 3: Build to verify no compile errors**

```bash
cargo build -p spectral 2>&1 | grep "^error" | head -10
```

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/commands/brokers.rs
git commit -m "feat(broker-api): add search_method_type and removal_action_url to BrokerDetail"
```

---

### Task 2.2 — Add `initiate_direct_removal` Command

**Files:**
- Modify: `src-tauri/src/commands/scan.rs`

**Context:** This command creates a standalone removal attempt for one broker + profile pair, then immediately queues it for processing via the existing removal worker. Returns the removal attempt ID.

**Task Status:** ✅ Complete

- [x] **Step 1: Add the command to `scan.rs`**

Add this function to `src-tauri/src/commands/scan.rs` (near the other removal commands, after `submit_removals_for_confirmed`):

```rust
/// Initiate a removal attempt for a broker that cannot be auto-scanned.
///
/// Creates a standalone removal attempt (not linked to a scan finding),
/// then processes it immediately via the existing removal worker.
/// Use for brokers with email, web-form, or manual search methods.
///
/// Returns the removal attempt ID.
#[tauri::command]
pub async fn initiate_direct_removal<R: tauri::Runtime>(
    vault_id: String,
    broker_id: String,
    profile_id: String,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<String, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault {vault_id} not found or locked"))?;

    let db = state
        .db(&vault_id)
        .map_err(|e| format!("Failed to get database: {e}"))?;

    // Create standalone removal attempt (no finding_id)
    let attempt = spectral_db::removal_attempts::create_standalone_removal_attempt(
        db.pool(),
        &broker_id,
        &profile_id,
    )
    .await
    .map_err(|e| format!("Failed to create removal attempt: {e}"))?;

    let attempt_id = attempt.id.clone();

    // Process immediately using the existing batch machinery
    let broker_registry = state.broker_registry();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
    let browser_engine: Arc<tokio::sync::Mutex<Option<Arc<spectral_browser::BrowserEngine>>>> =
        Arc::new(tokio::sync::Mutex::new(None));

    let db_arc = Arc::new(db.as_ref().clone());
    let vault_arc = Arc::new(vault.as_ref().clone());
    let registry_arc = Arc::new(broker_registry.as_ref().clone());

    let attempt_id_clone = attempt_id.clone();
    let app_clone = app.clone();

    tokio::spawn(async move {
        let result = crate::removal_worker::submit_removal_task(
            db_arc,
            vault_arc,
            attempt_id_clone.clone(),
            registry_arc,
            semaphore,
            browser_engine,
        )
        .await;

        match result {
            Ok(worker_result) => {
                let _ = app_clone.emit("removal:success", serde_json::json!({
                    "attempt_id": worker_result.attempt_id,
                    "broker_id": worker_result.broker_id,
                }));
            }
            Err(e) => {
                tracing::error!("Direct removal failed for attempt {attempt_id_clone}: {e}");
                let _ = app_clone.emit("removal:failed", serde_json::json!({
                    "attempt_id": attempt_id_clone,
                    "error": e,
                }));
            }
        }
    });

    Ok(attempt_id)
}
```

- [x] **Step 2: Check `WorkerResult` struct exists and has `attempt_id` + `broker_id`**

```bash
grep -n "pub struct WorkerResult" /home/evan/projects/spectral/src-tauri/src/removal_worker.rs
```

If the struct looks different, adjust the field names in the emit calls to match what `submit_removal_task` actually returns.

- [x] **Step 3: Check how `state.db()` and `state.broker_registry()` work**

```bash
grep -n "fn db\|fn broker_registry\|fn get_vault" /home/evan/projects/spectral/src-tauri/src/state.rs | head -10
```

Adjust method calls if the actual API differs.

- [x] **Step 4: Build**

```bash
cargo build -p spectral 2>&1 | grep "^error" | head -20
```

Fix any compile errors before proceeding.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/commands/scan.rs
git commit -m "feat(commands): add initiate_direct_removal command for non-scannable brokers"
```

---

### Task 2.3 — Add `initiate_bulk_removal` Command

**Files:**
- Modify: `src-tauri/src/commands/scan.rs`

**Context:** Creates standalone removal attempts for all brokers of a given category that are NOT `Scannable` (i.e., email/web-form/manual search method), then enqueues all attempts for processing. Returns a list of attempt IDs so the frontend can track progress.

**Task Status:** ✅ Complete

- [x] **Step 1: Add the command**

Add this function after `initiate_direct_removal` in `src-tauri/src/commands/scan.rs`:

```rust
/// Initiate removal attempts for all non-scannable brokers in a category.
///
/// For each broker in `broker_ids` that has a non-UrlTemplate search method,
/// creates a standalone removal attempt and enqueues it for processing.
/// Brokers with UrlTemplate search (auto-scannable) are skipped — use a scan first.
///
/// Returns a list of created removal attempt IDs.
#[tauri::command]
pub async fn initiate_bulk_removal<R: tauri::Runtime>(
    vault_id: String,
    profile_id: String,
    broker_ids: Vec<String>,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle<R>,
) -> Result<Vec<String>, String> {
    use spectral_broker::definition::SearchMethod;

    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| format!("Vault {vault_id} not found or locked"))?;

    let db = state
        .db(&vault_id)
        .map_err(|e| format!("Failed to get database: {e}"))?;

    let broker_registry = state.broker_registry();
    let mut attempt_ids: Vec<String> = Vec::new();

    // Create standalone removal attempts for all non-scannable brokers
    for broker_id_str in &broker_ids {
        let broker_id = match spectral_core::BrokerId::new(broker_id_str) {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!("Skipping invalid broker ID {broker_id_str}: {e}");
                continue;
            }
        };

        let broker_def = match broker_registry.get(&broker_id) {
            Ok(def) => def,
            Err(e) => {
                tracing::warn!("Skipping unknown broker {broker_id_str}: {e}");
                continue;
            }
        };

        // Skip scannable brokers — they need a scan first
        if matches!(broker_def.search, SearchMethod::UrlTemplate { .. }) {
            tracing::debug!("Skipping scannable broker {broker_id_str} in bulk removal");
            continue;
        }

        match spectral_db::removal_attempts::create_standalone_removal_attempt(
            db.pool(),
            broker_id_str,
            &profile_id,
        )
        .await
        {
            Ok(attempt) => attempt_ids.push(attempt.id),
            Err(e) => tracing::error!("Failed to create removal attempt for {broker_id_str}: {e}"),
        }
    }

    if attempt_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Process all attempts using existing batch machinery
    let semaphore = Arc::new(tokio::sync::Semaphore::new(3));
    let browser_engine: Arc<tokio::sync::Mutex<Option<Arc<spectral_browser::BrowserEngine>>>> =
        Arc::new(tokio::sync::Mutex::new(None));
    let db_arc = Arc::new(db.as_ref().clone());
    let vault_arc = Arc::new(vault.as_ref().clone());
    let registry_arc = Arc::new(broker_registry.as_ref().clone());

    let tasks: Vec<_> = attempt_ids
        .iter()
        .map(|attempt_id| {
            let db_clone = Arc::clone(&db_arc);
            let vault_clone = Arc::clone(&vault_arc);
            let registry_clone = Arc::clone(&registry_arc);
            let sem_clone = Arc::clone(&semaphore);
            let browser_clone = Arc::clone(&browser_engine);
            let attempt_id_clone = attempt_id.clone();
            let app_clone = app.clone();

            tokio::spawn(async move {
                let result = crate::removal_worker::submit_removal_task(
                    db_clone,
                    vault_clone,
                    attempt_id_clone.clone(),
                    registry_clone,
                    sem_clone,
                    browser_clone,
                )
                .await;

                match result {
                    Ok(worker_result) => {
                        let _ = app_clone.emit("removal:success", serde_json::json!({
                            "attempt_id": worker_result.attempt_id,
                            "broker_id": worker_result.broker_id,
                        }));
                    }
                    Err(e) => {
                        tracing::error!("Bulk removal failed for attempt {attempt_id_clone}: {e}");
                        let _ = app_clone.emit("removal:failed", serde_json::json!({
                            "attempt_id": attempt_id_clone,
                            "error": e,
                        }));
                    }
                }
            })
        })
        .collect();

    // Emit started event for each attempt
    for attempt_id in &attempt_ids {
        let _ = app.emit("removal:started", serde_json::json!({ "attempt_id": attempt_id }));
    }

    // Don't await tasks — return immediately and let them run in background
    drop(tasks);

    Ok(attempt_ids)
}
```

- [x] **Step 2: Build**

```bash
cargo build -p spectral 2>&1 | grep "^error" | head -20
```

- [x] **Step 3: Commit**

```bash
git add src-tauri/src/commands/scan.rs
git commit -m "feat(commands): add initiate_bulk_removal command for non-scannable broker batch removal"
```

---

### Task 2.4 — Register Commands + Update TypeScript Interfaces

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/api/brokers.ts`
- Modify: `src/lib/api/removal.ts`

**Task Status:** ✅ Complete

- [x] **Step 1: Register both new commands in `src-tauri/src/lib.rs`**

Find the `.invoke_handler(tauri::generate_handler![` block and add the two new commands:

```rust
// In the generate_handler! list, after submit_removals_for_confirmed:
commands::scan::initiate_direct_removal,
commands::scan::initiate_bulk_removal,
```

- [x] **Step 2: Update `BrokerDetail` TypeScript interface in `src/lib/api/brokers.ts`**

Find the `BrokerDetail` interface and add two new fields:

```typescript
export interface BrokerDetail extends BrokerSummary {
  removal_method: string;
  url: string;
  recheck_interval_days: number;
  last_verified: string;
  scan_status: string | null;
  finding_count: number | null;
  email_template: EmailTemplate | null;
  email_fallback: EmailFallbackResponse | null;
  /** Whether this broker can be auto-scanned or requires manual action */
  search_method_type: 'scannable' | 'web_form' | 'manual';
  /** URL for the removal action form, or null for email/manual methods */
  removal_action_url: string | null;
}
```

- [x] **Step 3: Add new API methods to `src/lib/api/removal.ts`**

Add after the existing `removalAPI` methods:

```typescript
/**
 * Initiate a direct removal attempt for one non-scannable broker.
 * Returns the removal attempt ID.
 */
async initiateDirectRemoval(
  vaultId: string,
  brokerId: string,
  profileId: string,
): Promise<string> {
  return await invoke<string>('initiate_direct_removal', {
    vaultId,
    brokerId,
    profileId,
  });
},

/**
 * Initiate bulk removal for all non-scannable brokers in a list.
 * Returns array of created removal attempt IDs.
 */
async initiateBulkRemoval(
  vaultId: string,
  profileId: string,
  brokerIds: string[],
): Promise<string[]> {
  return await invoke<string[]>('initiate_bulk_removal', {
    vaultId,
    profileId,
    brokerIds,
  });
},
```

- [x] **Step 4: Build and type-check**

```bash
cargo build -p spectral 2>&1 | grep "^error" | head -10
cd /home/evan/projects/spectral && npx tsc --noEmit 2>&1 | head -20
```

Expected: No errors.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src/lib/api/brokers.ts src/lib/api/removal.ts
git commit -m "feat(api): register new removal commands and update TypeScript interfaces"
```

---

### Task 2.5 — Phase 2 SonarQube Scan

**Task Status:** ✅ Complete

- [x] **Step 1: Run SonarQube scan** (same command as Task 1.4)
- [x] **Step 2: Address any Blocker/Critical issues** in Phase 2 files
- [x] **Step 3: Update Phase Tracking table** — mark Phase 2 ✅ Complete

---

## Phase 3: Smart Action UI — Per-Broker Action Buttons

**Goal:** Replace the static "Scan This Company" button on the adtech detail page with a smart `<RemovalActionButton>` that shows the appropriate action based on `search_method_type` and `removal_method`.

**Phase Status:** ✅ Complete

### Task 3.1 — Create `RemovalActionButton.svelte`

**Files:**
- Create: `src/lib/components/removals/RemovalActionButton.svelte`

**Context:** A self-contained component that receives a `BrokerDetail` and `profileId` and renders the correct primary action button. Logic:
- `search_method_type === 'scannable'` → **Scan** button (existing behavior)
- `removal_method` contains `Email` → **Send Removal Email** button (opens mailto:)
- `removal_method` contains `WebForm` or `BrowserForm` → **Submit Opt-Out Form** button (triggers `initiate_direct_removal`)
- `removal_method` contains `Manual` or `Phone` → **View Instructions** button (expands instructions inline)

**Task Status:** ✅ Complete

- [x] **Step 1: Create the component**

```svelte
<!-- src/lib/components/removals/RemovalActionButton.svelte -->
<script lang="ts">
	import type { BrokerDetail } from '$lib/api/brokers';
	import { removalAPI } from '$lib/api/removal';

	interface Props {
		broker: BrokerDetail;
		profileId: string;
		onScanClick: () => void;
		class?: string;
	}

	let { broker, profileId, onScanClick, class: extraClass = '' }: Props = $props();

	let submitting = $state(false);
	let submitted = $state(false);
	let submitError = $state<string | null>(null);
	let showInstructions = $state(false);

	const isEmailMethod = $derived(broker.removal_method.startsWith('Email'));
	const isWebFormMethod = $derived(
		broker.removal_method.startsWith('WebForm') ||
		broker.removal_method.startsWith('BrowserForm')
	);
	const isManualMethod = $derived(
		broker.removal_method.startsWith('Manual') ||
		broker.removal_method.startsWith('Phone')
	);

	function buildMailtoUrl(): string {
		if (!broker.email_template) return `mailto:?subject=Data Removal Request`;
		const { email, subject, body } = broker.email_template;
		return `mailto:${encodeURIComponent(email)}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
	}

	async function handleFormSubmit() {
		submitting = true;
		submitError = null;
		try {
			await removalAPI.initiateDirectRemoval(
				broker.id,  // vaultId is not on broker — see note below
				broker.id,
				profileId,
			);
			submitted = true;
		} catch (err) {
			submitError = err instanceof Error ? err.message : String(err);
		} finally {
			submitting = false;
		}
	}
</script>
```

> **Note on vaultId:** The component needs `vaultId` for the removal API call, but `BrokerDetail` doesn't carry it. Add a `vaultId` prop alongside `profileId`:

Update the Props interface:

```typescript
interface Props {
    broker: BrokerDetail;
    vaultId: string;
    profileId: string;
    onScanClick: () => void;
    class?: string;
}

let { broker, vaultId, profileId, onScanClick, class: extraClass = '' }: Props = $props();
```

Update `handleFormSubmit`:

```typescript
async function handleFormSubmit() {
    submitting = true;
    submitError = null;
    try {
        await removalAPI.initiateDirectRemoval(vaultId, broker.id, profileId);
        submitted = true;
    } catch (err) {
        submitError = err instanceof Error ? err.message : String(err);
    } finally {
        submitting = false;
    }
}
```

Complete template:

```svelte
{#if broker.search_method_type === 'scannable'}
	<!-- Scannable broker: delegate to parent scan handler -->
	<button
		onclick={onScanClick}
		class="flex-1 px-6 py-3 border-2 border-orange-600 text-orange-700 rounded-lg font-medium hover:bg-orange-50 transition-colors {extraClass}"
	>
		Scan This Company
	</button>

{:else if isEmailMethod}
	<!-- Email-method broker: open mailto: in user's email client -->
	{#if submitted}
		<p class="flex-1 px-6 py-3 text-green-700 font-medium text-center">
			✓ Email draft opened — send it from your email client
		</p>
	{:else}
		<a
			href={buildMailtoUrl()}
			class="flex-1 px-6 py-3 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 transition-colors text-center {extraClass}"
			onclick={() => (submitted = true)}
		>
			Send Removal Email
		</a>
	{/if}

{:else if isWebFormMethod}
	<!-- Web-form broker: trigger automated form submission -->
	{#if submitted}
		<p class="flex-1 px-6 py-3 text-green-700 font-medium text-center">
			✓ Form submission queued — check Removal History for progress
		</p>
	{:else if submitError}
		<div class="flex-1 flex flex-col gap-2">
			<p class="text-sm text-red-700">✗ {submitError}</p>
			<button
				onclick={handleFormSubmit}
				class="px-6 py-3 border-2 border-orange-600 text-orange-700 rounded-lg font-medium hover:bg-orange-50 transition-colors"
			>
				Retry
			</button>
		</div>
	{:else}
		<button
			onclick={handleFormSubmit}
			disabled={submitting}
			class="flex-1 px-6 py-3 bg-orange-600 text-white rounded-lg font-medium hover:bg-orange-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed {extraClass}"
		>
			{submitting ? 'Submitting...' : 'Submit Opt-Out Form'}
		</button>
	{/if}

{:else if isManualMethod}
	<!-- Manual/Phone: show instructions inline -->
	<button
		onclick={() => (showInstructions = !showInstructions)}
		class="flex-1 px-6 py-3 border-2 border-gray-400 text-gray-700 rounded-lg font-medium hover:bg-gray-50 transition-colors {extraClass}"
	>
		{showInstructions ? 'Hide Instructions' : 'View Removal Instructions'}
	</button>

	{#if showInstructions && broker.removal_action_url}
		<div class="w-full mt-3 p-4 bg-gray-50 border border-gray-200 rounded-lg">
			<p class="text-sm text-gray-700 mb-3">
				This company requires manual opt-out. Visit their privacy page:
			</p>
			<a
				href={broker.removal_action_url}
				target="_blank"
				rel="noopener noreferrer"
				class="inline-block px-4 py-2 bg-gray-700 text-white text-sm rounded hover:bg-gray-800 transition-colors"
			>
				Open Privacy / Opt-Out Page ↗
			</a>
		</div>
	{/if}
{/if}
```

- [x] **Step 2: Type-check the new component**

```bash
cd /home/evan/projects/spectral && npx tsc --noEmit 2>&1 | head -20
```

- [x] **Step 3: Commit**

```bash
git add src/lib/components/removals/RemovalActionButton.svelte
git commit -m "feat(ui): add RemovalActionButton component with smart action routing"
```

---

### Task 3.2 — Update the AdTech Detail Page

**Files:**
- Modify: `src/routes/adtech/[adtechId]/+page.svelte`

**Context:** Import `RemovalActionButton`, pass it the necessary props, and replace the static "Scan This Company" button in the action buttons section (lines 400–406).

**Task Status:** ✅ Complete

- [x] **Step 1: Add the import**

At the top of the `<script>` section, add:

```typescript
import RemovalActionButton from '$lib/components/removals/RemovalActionButton.svelte';
```

- [x] **Step 2: Add `vaultId` and `profileId` reactive values**

In the `<script>` section, the page already imports `vaultStore` and `profileStore`. Add two derived values:

```typescript
const vaultId = $derived(vaultStore.currentVaultId ?? '');
// profileId is loaded in handleTargetedScan — we need it at page level too
let currentProfileId = $state<string>('');

$effect(() => {
    async function resolveProfile() {
        if (!vaultId) return;
        await profileStore.loadProfiles(vaultId);
        if (profileStore.profiles.length > 0) {
            currentProfileId = profileStore.profiles[0].id;
        }
    }
    resolveProfile();
});
```

- [x] **Step 3: Replace the "Scan This Company" button with `<RemovalActionButton>`**

Find the Action Buttons section (around line 390–413):

```svelte
<!-- OLD: Action Buttons -->
<div class="flex flex-col sm:flex-row gap-4">
    <a href={adtech.url} ...>Visit Company Website ↗</a>
    <button onclick={handleTargetedScan} ...>
        {scanStarting || scanInProgress ? 'Scanning...' : 'Scan This Company'}
    </button>
    <button onclick={() => goto('/scan')} ...>Full Scan Center</button>
</div>
```

Replace with:

```svelte
<!-- NEW: Action Buttons -->
<div class="flex flex-col sm:flex-row gap-4 flex-wrap">
    <a
        href={adtech.url}
        target="_blank"
        rel="noopener noreferrer"
        class="flex-1 px-6 py-3 bg-orange-600 text-white rounded-lg font-medium hover:bg-orange-700 transition-colors text-center"
    >
        Visit Company Website ↗
    </a>

    {#if scanStarting || scanInProgress}
        <button
            disabled
            class="flex-1 px-6 py-3 border-2 border-orange-600 text-orange-700 rounded-lg font-medium opacity-50 cursor-not-allowed"
        >
            Scanning...
        </button>
    {:else}
        <RemovalActionButton
            broker={adtech}
            vaultId={vaultId}
            profileId={currentProfileId}
            onScanClick={handleTargetedScan}
        />
    {/if}

    <button
        onclick={() => goto('/scan')}
        class="flex-1 px-6 py-3 border border-gray-300 text-gray-700 rounded-lg font-medium hover:bg-gray-50 transition-colors"
    >
        Full Scan Center
    </button>
</div>
```

- [x] **Step 4: Run type-check and lint**

```bash
cd /home/evan/projects/spectral && npx tsc --noEmit 2>&1 | head -20
npx prettier --check src/routes/adtech/\[adtechId\]/+page.svelte
```

Fix any formatting issues:

```bash
npx prettier --write src/routes/adtech/\[adtechId\]/+page.svelte
```

- [x] **Step 5: Commit**

```bash
git add src/routes/adtech/\[adtechId\]/+page.svelte
git commit -m "feat(adtech): replace static scan button with smart RemovalActionButton"
```

---

### Task 3.3 — Phase 3 SonarQube Scan

**Task Status:** ✅ Complete

- [x] **Step 1: Run SonarQube scan**
- [x] **Step 2: Address any Blocker/Critical issues** in Phase 3 files
- [x] **Step 3: Update Phase Tracking table** — mark Phase 3 ✅ Complete

---

## Phase 4: Bulk Removal — Remove All

**Goal:** Add a "Remove All" button on the adtech list page that triggers `initiate_bulk_removal` for all non-scannable marketing brokers, with inline progress display.

**Phase Status:** ✅ Complete

### Task 4.1 — Update the AdTech List Page

**Files:**
- Modify: `src/routes/adtech/+page.svelte`

**Context:** The adtech list page currently shows a table of Marketing brokers with search/filter. We add a "Remove All" button in the header area and a progress panel below it.

**Task Status:** ✅ Complete

- [x] **Step 1: Add imports and state**

In `src/routes/adtech/+page.svelte`, add to the existing `<script>` block:

```typescript
import { removalAPI } from '$lib/api/removal';
import { vaultStore, profileStore } from '$lib/stores';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { onDestroy } from 'svelte';

let profileId = $state<string>('');
let bulkRemoving = $state(false);
let bulkAttemptIds = $state<string[]>([]);
let bulkDoneCount = $state(0);
let bulkFailCount = $state(0);
let bulkError = $state<string | null>(null);

let unlistenSuccess: UnlistenFn | null = null;
let unlistenFailed: UnlistenFn | null = null;

onDestroy(() => {
    unlistenSuccess?.();
    unlistenFailed?.();
});

$effect(() => {
    async function resolveProfile() {
        const vaultId = vaultStore.currentVaultId;
        if (!vaultId) return;
        await profileStore.loadProfiles(vaultId);
        if (profileStore.profiles.length > 0) {
            profileId = profileStore.profiles[0].id;
        }
    }
    resolveProfile();
});

async function handleRemoveAll() {
    const vaultId = vaultStore.currentVaultId;
    if (!vaultId || !profileId) {
        bulkError = 'No vault or profile available.';
        return;
    }

    bulkRemoving = true;
    bulkError = null;
    bulkDoneCount = 0;
    bulkFailCount = 0;

    // Listen for real-time progress events
    unlistenSuccess = await listen<{ attempt_id: string }>('removal:success', () => {
        bulkDoneCount += 1;
    });
    unlistenFailed = await listen<{ attempt_id: string; error: string }>('removal:failed', () => {
        bulkFailCount += 1;
    });

    try {
        const allBrokerIds = brokers.map((b) => b.id);
        bulkAttemptIds = await removalAPI.initiateBulkRemoval(vaultId, profileId, allBrokerIds);

        if (bulkAttemptIds.length === 0) {
            bulkError =
                'No non-scannable brokers found. All brokers in this list require a scan first.';
            bulkRemoving = false;
            return;
        }
    } catch (err) {
        bulkError = err instanceof Error ? err.message : String(err);
        bulkRemoving = false;
        unlistenSuccess?.();
        unlistenFailed?.();
    }
}

const bulkTotal = $derived(bulkAttemptIds.length);
const bulkComplete = $derived(bulkDoneCount + bulkFailCount >= bulkTotal && bulkTotal > 0);

$effect(() => {
    if (bulkComplete) {
        bulkRemoving = false;
        unlistenSuccess?.();
        unlistenFailed?.();
    }
});
```

- [x] **Step 2: Add "Remove All" button and progress panel to the template**

In the template, find the section just before the broker table/list. Add after the search/filter controls and before the table:

```svelte
<!-- Remove All Section -->
<div class="mb-6 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
    <div>
        <p class="text-sm text-gray-600">
            {filteredBrokers.length} companies shown.
            Non-scannable companies will have removal requests submitted automatically.
        </p>
    </div>
    <button
        onclick={handleRemoveAll}
        disabled={bulkRemoving || !profileId}
        class="px-6 py-2 bg-red-600 text-white rounded-lg font-medium hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
    >
        {bulkRemoving ? 'Removing...' : 'Remove All'}
    </button>
</div>

<!-- Bulk Progress -->
{#if bulkAttemptIds.length > 0 || bulkError}
    <div class="mb-6 p-4 border rounded-lg {bulkComplete ? (bulkFailCount > 0 ? 'bg-yellow-50 border-yellow-300' : 'bg-green-50 border-green-300') : 'bg-orange-50 border-orange-200'}">
        {#if bulkError}
            <p class="text-sm text-red-700">✗ {bulkError}</p>
        {:else if bulkComplete}
            <p class="text-sm font-medium {bulkFailCount > 0 ? 'text-yellow-800' : 'text-green-800'}">
                ✓ Bulk removal complete — {bulkDoneCount} submitted
                {bulkFailCount > 0 ? `, ${bulkFailCount} failed` : ''}.
                Check <a href="/removals" class="underline">Removal History</a> for details.
            </p>
        {:else}
            <div class="flex items-center gap-3">
                <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-orange-600"></div>
                <p class="text-sm text-gray-700">
                    Submitting removals: {bulkDoneCount + bulkFailCount} / {bulkTotal} complete
                </p>
            </div>
            <div class="mt-2 w-full bg-gray-200 rounded-full h-2">
                <div
                    class="h-2 rounded-full bg-orange-500 transition-all duration-300"
                    style="width: {bulkTotal > 0 ? Math.round(((bulkDoneCount + bulkFailCount) / bulkTotal) * 100) : 0}%"
                ></div>
            </div>
        {/if}
    </div>
{/if}
```

- [x] **Step 3: Format and type-check**

```bash
cd /home/evan/projects/spectral
npx prettier --write src/routes/adtech/+page.svelte
npx tsc --noEmit 2>&1 | head -20
```

- [x] **Step 4: Commit**

```bash
git add src/routes/adtech/+page.svelte
git commit -m "feat(adtech): add Remove All button with bulk removal progress tracking"
```

---

### Task 4.2 — Phase 4 SonarQube Scan

**Task Status:** ✅ Complete

- [x] **Step 1: Run full SonarQube scan against all changed files**
- [x] **Step 2: Address any Blocker/Critical issues**
- [x] **Step 3: Update Phase Tracking table** — mark Phase 4 ✅ Complete
- [x] **Step 4: Final commit**

```bash
git push origin feature/adtech-refinements
```

---

## Passing Criteria (All Phases)

Before marking a phase complete, verify:

- [x] `cargo build -p spectral` passes with zero errors
- [x] `cargo test` passes — no regressions in `spectral-db`, `spectral-scanner`, `spectral-browser`
- [x] `npx tsc --noEmit` passes — no TypeScript errors
- [x] `npx prettier --check src/` passes — no formatting violations
- [x] SonarQube shows no new Blocker or Critical issues

## Acceptance Criteria (Full Feature)

- [x] Adtech detail page: Klaviyo (email method) shows **Send Removal Email** button that opens a pre-filled mailto: link
- [x] Adtech detail page: TruthFinder (web-form method) shows **Submit Opt-Out Form** button that triggers automated submission
- [x] Adtech detail page: a `url-template` broker (e.g., Spokeo) still shows **Scan This Company** button
- [x] Adtech detail page: a manual-only broker shows **View Removal Instructions** with a link to the privacy page
- [x] Adtech list page: **Remove All** button creates removal attempts for all non-scannable brokers and shows live progress
- [x] **Remove All** skips scannable brokers (no broken scans triggered)
- [x] Removal History page shows all attempts created via direct and bulk removal
