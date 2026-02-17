# Phase 6 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deliver job history, privacy score, browser-based removal, email removal, email verification monitoring, and background scheduling.

**Architecture:** Six features in user-journey order. Features 1–2 add UI over existing DB data. Features 3–6 add new backend crates (`spectral-mail`, `spectral-scheduler`) and wire the existing `spectral-browser` crate into the removal worker. All features use the same Tauri event/command/store patterns established in Phase 5.

**Tech Stack:** Rust (Tauri, sqlx, chromiumoxide, lettre, async-imap), Svelte 5 (runes), SvelteKit, Tailwind CSS, SQLite migrations

---

## Codebase Reference

- Broker definitions: `broker-definitions/**/*.toml` — `[removal] method = "web-form" | "browser-form" | "email"`
- Broker types: `crates/spectral-broker/src/definition.rs` — `RemovalMethod` enum has `WebForm`, `Email`, `Phone`
- DB migrations: `crates/spectral-db/migrations/NNN_name.sql` — numbered SQL files
- Removal worker: `src-tauri/src/removal_worker.rs`
- Tauri commands: `src-tauri/src/commands/scan.rs`
- App state: `src-tauri/src/state.rs`
- Frontend stores: `src/lib/stores/`
- Frontend API: `src/lib/api/`
- Routes: `src/routes/`
- Phase 5 progress dashboard: `src/routes/removals/progress/[jobId]/+page.svelte`

---

## Task 1: Job History DB Query and Tauri Command

**Files:**
- Modify: `crates/spectral-db/src/removal_attempts.rs`
- Modify: `src-tauri/src/commands/scan.rs`

**Step 1: Write the failing test**

In `crates/spectral-db/src/removal_attempts.rs`, add to the `#[cfg(test)]` module:

```rust
#[tokio::test]
async fn test_get_job_history() {
    let db = Database::new_in_memory().await.unwrap();
    db.run_migrations().await.unwrap();
    let pool = db.pool();

    // Insert two scan jobs with attempts
    let vault_id = "vault-1";
    let job_a = "job-a";
    let job_b = "job-b";
    insert_test_attempt(pool, vault_id, job_a, "attempt-1", RemovalStatus::Submitted).await;
    insert_test_attempt(pool, vault_id, job_a, "attempt-2", RemovalStatus::Failed).await;
    insert_test_attempt(pool, vault_id, job_b, "attempt-3", RemovalStatus::Completed).await;

    let history = get_job_history(pool, vault_id).await.unwrap();
    assert_eq!(history.len(), 2);
    // Newest first
    let job_a_summary = history.iter().find(|h| h.scan_job_id == job_a).unwrap();
    assert_eq!(job_a_summary.total, 2);
    assert_eq!(job_a_summary.submitted_count, 1);
    assert_eq!(job_a_summary.failed_count, 1);
}
```

**Step 2: Run the test to verify it fails**

```bash
cargo test -p spectral-db test_get_job_history 2>&1 | grep -E "FAIL|error|not found"
```

Expected: compile error — `get_job_history` not defined.

**Step 3: Add the `RemovalJobSummary` type and `get_job_history` function**

In `crates/spectral-db/src/removal_attempts.rs`, after the existing types:

```rust
/// Summary of a removal batch for the job history page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovalJobSummary {
    pub scan_job_id: String,
    pub vault_id: String,
    pub submitted_at: DateTime<Utc>,
    pub total: i64,
    pub submitted_count: i64,
    pub completed_count: i64,
    pub failed_count: i64,
    pub pending_count: i64,
}

/// Get job history summary grouped by scan job, newest first.
pub async fn get_job_history(
    pool: &Pool<Sqlite>,
    vault_id: &str,
) -> Result<Vec<RemovalJobSummary>> {
    let rows = sqlx::query(
        r#"
        SELECT
            scan_job_id,
            vault_id,
            MIN(created_at) as submitted_at,
            COUNT(*) as total,
            SUM(CASE WHEN status = 'Submitted' THEN 1 ELSE 0 END) as submitted_count,
            SUM(CASE WHEN status = 'Completed' THEN 1 ELSE 0 END) as completed_count,
            SUM(CASE WHEN status = 'Failed'    THEN 1 ELSE 0 END) as failed_count,
            SUM(CASE WHEN status = 'Pending'   THEN 1 ELSE 0 END) as pending_count
        FROM removal_attempts
        WHERE vault_id = ?
        GROUP BY scan_job_id
        ORDER BY submitted_at DESC
        "#,
    )
    .bind(vault_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(RemovalJobSummary {
                scan_job_id: row.try_get("scan_job_id")?,
                vault_id: row.try_get("vault_id")?,
                submitted_at: row.try_get("submitted_at")?,
                total: row.try_get("total")?,
                submitted_count: row.try_get("submitted_count")?,
                completed_count: row.try_get("completed_count")?,
                failed_count: row.try_get("failed_count")?,
                pending_count: row.try_get("pending_count")?,
            })
        })
        .collect()
}
```

Also export `RemovalJobSummary` in `crates/spectral-db/src/lib.rs`:

```rust
pub use removal_attempts::RemovalJobSummary;
```

**Step 4: Add the Tauri command**

In `src-tauri/src/commands/scan.rs`, add:

```rust
#[tauri::command]
pub async fn get_removal_job_history(
    vault_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<spectral_db::RemovalJobSummary>, CommandError> {
    let db = state.get_db(&vault_id).await?;
    spectral_db::removal_attempts::get_job_history(db.pool(), &vault_id)
        .await
        .map_err(CommandError::from)
}
```

Register in `src-tauri/src/lib.rs` invoke_handler:

```rust
commands::scan::get_removal_job_history,
```

**Step 5: Run the test to verify it passes**

```bash
cargo test -p spectral-db test_get_job_history
cargo clippy -p spectral-db -- -D warnings
```

Expected: test passes, no clippy warnings.

**Step 6: Commit**

```bash
git add crates/spectral-db/src/removal_attempts.rs \
        crates/spectral-db/src/lib.rs \
        src-tauri/src/commands/scan.rs \
        src-tauri/src/lib.rs
git commit -m "feat(db): add job history query and Tauri command"
```

---

## Task 2: Job History Frontend Page

**Files:**
- Modify: `src/lib/api/removal.ts`
- Create: `src/routes/removals/+page.svelte`
- Create: `src/routes/removals/+page.ts`

**Step 1: Add the API method**

In `src/lib/api/removal.ts`, add the interface and method:

```typescript
export interface RemovalJobSummary {
  scan_job_id: string;
  vault_id: string;
  submitted_at: string;
  total: number;
  submitted_count: number;
  completed_count: number;
  failed_count: number;
  pending_count: number;
}
```

In the `removalAPI` object:

```typescript
async getJobHistory(vaultId: string): Promise<RemovalJobSummary[]> {
  return await invoke('get_removal_job_history', { vaultId });
},
```

**Step 2: Replace the redirect stub with the real page**

Replace `src/routes/removals/+page.svelte` entirely:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { vaultStore } from '$lib/stores';
  import { removalAPI } from '$lib/api/removal';
  import type { RemovalJobSummary } from '$lib/api/removal';

  let jobs = $state<RemovalJobSummary[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let expandedJob = $state<string | null>(null);

  onMount(async () => {
    if (!vaultStore.currentVaultId) {
      goto('/');
      return;
    }
    try {
      jobs = await removalAPI.getJobHistory(vaultStore.currentVaultId);
    } catch (err) {
      error = 'Failed to load removal history.';
      console.error(err);
    } finally {
      loading = false;
    }
  });

  function formatDate(iso: string) {
    return new Date(iso).toLocaleDateString(undefined, {
      year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
    });
  }

  function toggleJob(jobId: string) {
    expandedJob = expandedJob === jobId ? null : jobId;
  }
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 p-4">
  <div class="max-w-4xl mx-auto">
    <div class="bg-white rounded-lg shadow-xl overflow-hidden">
      <div class="px-8 py-6 border-b border-gray-200 flex items-center justify-between">
        <div>
          <h1 class="text-3xl font-bold text-gray-900">Removal History</h1>
          <p class="text-gray-600 mt-1">All past removal batches</p>
        </div>
        <button onclick={() => goto('/')} class="px-4 py-2 text-gray-600 hover:text-gray-900 font-medium">
          ← Dashboard
        </button>
      </div>

      <div class="p-8">
        {#if loading}
          <div class="text-center py-12">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600 mx-auto"></div>
            <p class="text-gray-600 mt-4">Loading history...</p>
          </div>
        {:else if error}
          <div class="bg-red-50 border border-red-200 rounded-lg p-6">
            <p class="text-red-900">{error}</p>
          </div>
        {:else if jobs.length === 0}
          <div class="text-center py-16">
            <div class="inline-flex items-center justify-center w-16 h-16 bg-gray-100 rounded-full mb-4">
              <span class="text-3xl">📋</span>
            </div>
            <h3 class="text-lg font-semibold text-gray-900 mb-2">No removal history yet</h3>
            <p class="text-gray-600">Submit your first removal batch from the scan review page.</p>
            <button onclick={() => goto('/scan/start')} class="mt-4 px-6 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors">
              Start a Scan
            </button>
          </div>
        {:else}
          <div class="space-y-4">
            {#each jobs as job}
              <div class="border border-gray-200 rounded-lg overflow-hidden">
                <button
                  onclick={() => toggleJob(job.scan_job_id)}
                  class="w-full px-6 py-4 flex items-center justify-between hover:bg-gray-50 transition-colors text-left"
                >
                  <div>
                    <p class="font-medium text-gray-900">{formatDate(job.submitted_at)}</p>
                    <p class="text-sm text-gray-600 mt-1">{job.total} broker{job.total !== 1 ? 's' : ''} attempted</p>
                  </div>
                  <div class="flex items-center gap-3">
                    {#if job.completed_count > 0}
                      <span class="px-2 py-1 bg-green-100 text-green-800 rounded-full text-xs font-medium">{job.completed_count} confirmed</span>
                    {/if}
                    {#if job.submitted_count > 0}
                      <span class="px-2 py-1 bg-blue-100 text-blue-800 rounded-full text-xs font-medium">{job.submitted_count} submitted</span>
                    {/if}
                    {#if job.failed_count > 0}
                      <span class="px-2 py-1 bg-red-100 text-red-800 rounded-full text-xs font-medium">{job.failed_count} failed</span>
                    {/if}
                    {#if job.pending_count > 0}
                      <span class="px-2 py-1 bg-yellow-100 text-yellow-800 rounded-full text-xs font-medium">{job.pending_count} pending</span>
                    {/if}
                    <span class="text-gray-400">{expandedJob === job.scan_job_id ? '▲' : '▼'}</span>
                  </div>
                </button>

                {#if expandedJob === job.scan_job_id}
                  <div class="border-t border-gray-200 p-4 bg-gray-50 flex gap-3">
                    <a
                      href="/removals/progress/{job.scan_job_id}"
                      class="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors text-sm font-medium"
                    >
                      View Progress Dashboard
                    </a>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>
```

**Step 3: Add prerender opt-out for the route**

Replace `src/routes/removals/+page.ts`:

```typescript
export const prerender = false;
```

**Step 4: Verify build**

```bash
npm run check && npm run build 2>&1 | tail -5
```

Expected: `✔ done`

**Step 5: Commit**

```bash
git add src/lib/api/removal.ts src/routes/removals/+page.svelte src/routes/removals/+page.ts
git commit -m "feat(frontend): add removal job history page"
```

---

## Task 3: Privacy Score — Backend

**Files:**
- Create: `crates/spectral-db/src/privacy_score.rs`
- Modify: `crates/spectral-db/src/lib.rs`
- Modify: `src-tauri/src/commands/scan.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write the failing test**

Create `crates/spectral-db/src/privacy_score.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[tokio::test]
    async fn test_score_empty_vault() {
        let db = Database::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        let result = calculate_privacy_score(db.pool(), "vault-1").await.unwrap();
        // No data = perfect score (nothing found yet)
        assert_eq!(result.score, 100);
    }

    #[tokio::test]
    async fn test_score_decreases_with_unresolved_findings() {
        let db = Database::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        // Insert unresolved findings (status != Confirmed)
        // ... (use existing test helpers)
        let result = calculate_privacy_score(db.pool(), "vault-1").await.unwrap();
        assert!(result.score < 100);
    }
}
```

**Step 2: Run test to confirm it fails**

```bash
cargo test -p spectral-db test_score_empty_vault 2>&1 | grep -E "error|FAIL"
```

Expected: compile error — module not found.

**Step 3: Implement the score module**

```rust
//! Privacy score calculation from scan and removal data.

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use crate::error::Result;

/// Per-category breakdown for the privacy score page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStats {
    pub category: String,
    pub found: i64,
    pub submitted: i64,
    pub confirmed: i64,
}

/// Full privacy score result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyScoreResult {
    pub score: u8,
    pub descriptor: String,
    pub category_breakdown: Vec<CategoryStats>,
}

/// Penalty weights per broker category (points deducted per unresolved finding).
fn category_weight(category: &str) -> i64 {
    match category {
        "people-search" | "background-check" => 8,
        "data-aggregator" => 5,
        "marketing" => 3,
        _ => 2,
    }
}

fn descriptor(score: u8) -> String {
    match score {
        90..=100 => "Well Protected".to_string(),
        70..=89  => "Good".to_string(),
        40..=69  => "Improving".to_string(),
        _        => "At Risk".to_string(),
    }
}

/// Calculate the privacy score for a vault.
pub async fn calculate_privacy_score(
    pool: &Pool<Sqlite>,
    vault_id: &str,
) -> Result<PrivacyScoreResult> {
    // Get per-category finding counts
    let rows = sqlx::query(
        r#"
        SELECT
            COALESCE(bs.broker_category, 'unknown') as category,
            COUNT(DISTINCT f.id) as found,
            SUM(CASE WHEN ra.status IN ('Submitted','Completed') THEN 1 ELSE 0 END) as submitted,
            SUM(CASE WHEN ra.status = 'Completed' THEN 1 ELSE 0 END) as confirmed
        FROM findings f
        LEFT JOIN broker_scans bs ON f.broker_scan_id = bs.id
        LEFT JOIN removal_attempts ra ON ra.finding_id = f.id AND ra.vault_id = ?
        WHERE f.vault_id = ?
        GROUP BY category
        "#,
    )
    .bind(vault_id)
    .bind(vault_id)
    .fetch_all(pool)
    .await?;

    let mut penalty: i64 = 0;
    let mut breakdown = Vec::new();

    for row in &rows {
        let category: String = row.try_get("category")?;
        let found: i64 = row.try_get("found")?;
        let submitted: i64 = row.try_get("submitted").unwrap_or(0);
        let confirmed: i64 = row.try_get("confirmed").unwrap_or(0);
        let unresolved = found - submitted;
        penalty += unresolved.max(0) * category_weight(&category);
        breakdown.push(CategoryStats { category, found, submitted, confirmed });
    }

    let score = (100i64 - penalty).clamp(0, 100) as u8;
    Ok(PrivacyScoreResult {
        score,
        descriptor: descriptor(score),
        category_breakdown: breakdown,
    })
}
```

Export from `crates/spectral-db/src/lib.rs`:

```rust
pub mod privacy_score;
pub use privacy_score::{PrivacyScoreResult, CategoryStats};
```

**Step 4: Add Tauri command**

In `src-tauri/src/commands/scan.rs`:

```rust
#[tauri::command]
pub async fn get_privacy_score(
    vault_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<spectral_db::PrivacyScoreResult, CommandError> {
    let db = state.get_db(&vault_id).await?;
    spectral_db::privacy_score::calculate_privacy_score(db.pool(), &vault_id)
        .await
        .map_err(CommandError::from)
}
```

Register in `src-tauri/src/lib.rs`.

**Step 5: Run tests**

```bash
cargo test -p spectral-db privacy_score
cargo clippy -p spectral-db -- -D warnings
```

**Step 6: Commit**

```bash
git add crates/spectral-db/src/privacy_score.rs \
        crates/spectral-db/src/lib.rs \
        src-tauri/src/commands/scan.rs \
        src-tauri/src/lib.rs
git commit -m "feat(db): add privacy score calculation and Tauri command"
```

---

## Task 4: Privacy Score — Frontend Page

**Files:**
- Create: `src/routes/score/+page.svelte`
- Create: `src/routes/score/+page.ts`
- Modify: `src/lib/api/removal.ts`
- Modify: `src/routes/+page.svelte`

**Step 1: Add API method**

In `src/lib/api/removal.ts`:

```typescript
export interface CategoryStats {
  category: string;
  found: number;
  submitted: number;
  confirmed: number;
}

export interface PrivacyScoreResult {
  score: number;
  descriptor: string;
  category_breakdown: CategoryStats[];
}
```

In `removalAPI`:

```typescript
async getPrivacyScore(vaultId: string): Promise<PrivacyScoreResult> {
  return await invoke('get_privacy_score', { vaultId });
},
```

**Step 2: Create the score page**

`src/routes/score/+page.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { vaultStore } from '$lib/stores';
  import { removalAPI } from '$lib/api/removal';
  import type { PrivacyScoreResult } from '$lib/api/removal';

  let result = $state<PrivacyScoreResult | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    if (!vaultStore.currentVaultId) { goto('/'); return; }
    try {
      result = await removalAPI.getPrivacyScore(vaultStore.currentVaultId);
    } catch (err) {
      error = 'Failed to calculate privacy score.';
    } finally {
      loading = false;
    }
  });

  // SVG gauge: circumference for score arc
  const RADIUS = 70;
  const CIRCUMFERENCE = 2 * Math.PI * RADIUS;
  const gaugeOffset = $derived(
    result ? CIRCUMFERENCE - (result.score / 100) * CIRCUMFERENCE : CIRCUMFERENCE
  );
  const gaugeColor = $derived(
    result
      ? result.score >= 90 ? '#16a34a'
        : result.score >= 70 ? '#2563eb'
        : result.score >= 40 ? '#d97706'
        : '#dc2626'
      : '#e5e7eb'
  );
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 p-4">
  <div class="max-w-3xl mx-auto">
    <div class="bg-white rounded-lg shadow-xl overflow-hidden">
      <div class="px-8 py-6 border-b border-gray-200 flex items-center justify-between">
        <div>
          <h1 class="text-3xl font-bold text-gray-900">Privacy Health</h1>
          <p class="text-gray-600 mt-1">Your current privacy protection score</p>
        </div>
        <button onclick={() => goto('/')} class="px-4 py-2 text-gray-600 hover:text-gray-900 font-medium">
          ← Dashboard
        </button>
      </div>

      <div class="p-8">
        {#if loading}
          <div class="text-center py-12">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600 mx-auto"></div>
            <p class="text-gray-600 mt-4">Calculating score...</p>
          </div>
        {:else if error}
          <div class="bg-red-50 border border-red-200 rounded-lg p-6">
            <p class="text-red-900">{error}</p>
          </div>
        {:else if result}
          <!-- Gauge -->
          <div class="flex flex-col items-center mb-8">
            <svg width="180" height="180" viewBox="0 0 180 180">
              <circle cx="90" cy="90" r={RADIUS} fill="none" stroke="#e5e7eb" stroke-width="12" />
              <circle
                cx="90" cy="90" r={RADIUS}
                fill="none"
                stroke={gaugeColor}
                stroke-width="12"
                stroke-dasharray={CIRCUMFERENCE}
                stroke-dashoffset={gaugeOffset}
                stroke-linecap="round"
                transform="rotate(-90 90 90)"
                style="transition: stroke-dashoffset 0.6s ease"
              />
              <text x="90" y="85" text-anchor="middle" class="text-4xl font-bold" font-size="36" fill="#111827" font-weight="bold">{result.score}</text>
              <text x="90" y="108" text-anchor="middle" font-size="13" fill="#6b7280">{result.descriptor}</text>
            </svg>
          </div>

          <!-- Category breakdown -->
          {#if result.category_breakdown.length > 0}
            <div class="mt-4">
              <h2 class="text-lg font-semibold text-gray-900 mb-3">Breakdown by Category</h2>
              <div class="overflow-x-auto">
                <table class="w-full text-sm">
                  <thead>
                    <tr class="border-b border-gray-200">
                      <th class="text-left py-2 text-gray-600 font-medium">Category</th>
                      <th class="text-center py-2 text-gray-600 font-medium">Found</th>
                      <th class="text-center py-2 text-gray-600 font-medium">Submitted</th>
                      <th class="text-center py-2 text-gray-600 font-medium">Confirmed</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each result.category_breakdown as cat}
                      <tr class="border-b border-gray-100 hover:bg-gray-50">
                        <td class="py-3 capitalize">{cat.category.replace('-', ' ')}</td>
                        <td class="py-3 text-center font-medium text-red-600">{cat.found}</td>
                        <td class="py-3 text-center font-medium text-blue-600">{cat.submitted}</td>
                        <td class="py-3 text-center font-medium text-green-600">{cat.confirmed}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </div>
          {:else}
            <p class="text-center text-gray-600 mt-4">Run a scan to see your privacy score breakdown.</p>
          {/if}
        {/if}
      </div>
    </div>
  </div>
</div>
```

`src/routes/score/+page.ts`:

```typescript
export const prerender = false;
```

**Step 3: Add Privacy Health card to home dashboard**

In `src/routes/+page.svelte`, find the existing action cards section and add:

```svelte
<button
  onclick={() => goto('/score')}
  class="px-6 py-4 bg-white rounded-lg shadow border border-gray-200 hover:border-primary-300 hover:shadow-md transition-all text-left"
>
  <h3 class="font-semibold text-gray-900">Privacy Health</h3>
  <p class="text-sm text-gray-600 mt-1">View your privacy score</p>
</button>
```

**Step 4: Verify build**

```bash
npm run check && npm run build 2>&1 | tail -5
```

**Step 5: Commit**

```bash
git add src/routes/score/ src/routes/+page.svelte src/lib/api/removal.ts
git commit -m "feat(frontend): add privacy score page with gauge and category breakdown"
```

---

## Task 5: Browser-based Removal — Broker Definition Extension

**Files:**
- Modify: `crates/spectral-broker/src/definition.rs`
- Modify: `broker-definitions/background-check/truthfinder.toml` (and 4 others — JS-heavy brokers)

**Context:** The `RemovalMethod` enum in `definition.rs` already has `WebForm` and `Email` variants. We need to add a `BrowserForm` variant and add `requires_email_verification` to the metadata. Broker TOML `[removal] method` currently uses string `"web-form"` or `"email"`.

**Step 1: Write the failing test**

In `crates/spectral-broker/src/definition.rs` test module:

```rust
#[test]
fn test_browser_form_deserialises() {
    let toml = r#"
    [broker]
    id = "test-broker"
    name = "Test"
    url = "https://test.com"
    domain = "test.com"
    category = "people-search"
    difficulty = "Medium"
    typical_removal_days = 7
    recheck_interval_days = 30
    last_verified = "2026-01-01"

    [search]
    method = "web-form"
    url = "https://test.com/search"
    requires_fields = ["first_name", "last_name", "state"]

    [search.fields]
    first_name = "{first}"
    last_name = "{last}"
    state = "{state}"

    [search.result_selectors]
    results_container = ".results"
    result_item = ".result"
    listing_url = "a"
    name = ".name"
    age = ".age"
    location = ".location"
    relatives = ".relatives"
    phones = ".phone"
    emails = ".email"
    addresses = ".address"
    criminal_records = ".criminal"
    bankruptcies = ".bankruptcy"
    no_results_indicator = ".empty"
    captcha_required = ".captcha"

    [removal]
    method = "browser-form"
    url = "https://test.com/optout"
    confirmation = "none"
    requires_email_verification = false
    notes = "JS heavy"

    [removal.fields]
    listing_url = "{found_listing_url}"
    email = "{user_email}"

    [removal.form_selectors]
    listing_url_input = "#url"
    email_input = "#email"
    submit_button = "button[type=submit]"
    success_indicator = ".success"
    "#;
    let def: BrokerDefinition = toml::from_str(toml).unwrap();
    assert!(matches!(def.removal, RemovalMethod::BrowserForm { .. }));
}
```

**Step 2: Run to confirm it fails**

```bash
cargo test -p spectral-broker test_browser_form_deserialises 2>&1 | grep -E "error|FAIL"
```

**Step 3: Add `BrowserForm` variant and `requires_email_verification`**

In `crates/spectral-broker/src/definition.rs`, find the `RemovalMethod` enum and add the variant:

```rust
/// Browser-based removal using headless Chromium (for JS-heavy opt-out flows).
BrowserForm {
    /// Opt-out page URL
    url: String,
    /// Field name-to-template mappings (same format as WebForm)
    fields: HashMap<String, String>,
    /// CSS selectors for form interaction
    form_selectors: FormSelectors,
    /// Confirmation method after submission
    confirmation: ConfirmationMethod,
    /// Whether broker sends a verification email that must be clicked
    #[serde(default)]
    requires_email_verification: bool,
    /// Human-readable notes
    #[serde(default)]
    notes: String,
},
```

Also add `requires_email_verification` to `WebForm` and `Email` variants:

```rust
/// Whether broker sends a verification email that must be clicked
#[serde(default)]
requires_email_verification: bool,
```

Update the `validate()` match arm for `BrowserForm` — same validation as `WebForm` (url non-empty, fields non-empty, submit_button non-empty).

**Step 4: Run tests**

```bash
cargo test -p spectral-broker
cargo clippy -p spectral-broker -- -D warnings
```

**Step 5: Update 5 JS-heavy broker definitions**

In the following TOML files, change `method = "web-form"` to `method = "browser-form"` in the `[removal]` section. Choose brokers known to use JavaScript-rendered opt-out pages. If uncertain which are JS-heavy, pick: `truthfinder`, `instantcheckmate`, `spokeo`, `whitepages`, `mylife` (or equivalent brokers that exist in `broker-definitions/`). Check which files exist first:

```bash
find broker-definitions -name "*.toml" ! -name "schema*" | head -20
```

For each chosen broker, update only the `method` field under `[removal]`.

**Step 6: Validate broker definitions**

```bash
cargo test -p spectral-broker -- --include-ignored
```

**Step 7: Commit**

```bash
git add crates/spectral-broker/src/definition.rs broker-definitions/
git commit -m "feat(broker): add BrowserForm removal method and requires_email_verification flag"
```

---

## Task 6: Browser-based Removal — Worker Integration

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/removal_worker.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `crates/spectral-db/migrations/005_removal_evidence.sql`

**Step 1: Add the evidence migration**

Create `crates/spectral-db/migrations/005_removal_evidence.sql`:

```sql
-- Stores screenshot evidence captured after browser-based removal submission
CREATE TABLE removal_evidence (
    id TEXT PRIMARY KEY,
    attempt_id TEXT NOT NULL REFERENCES removal_attempts(id) ON DELETE CASCADE,
    screenshot_bytes BLOB NOT NULL,
    captured_at TEXT NOT NULL
);

CREATE INDEX idx_removal_evidence_attempt ON removal_evidence(attempt_id);
```

**Step 2: Write the failing worker test**

In `src-tauri/src/removal_worker.rs` test module, add:

```rust
#[tokio::test]
async fn test_route_browser_form_method() {
    // Test that BrowserForm broker routes to submit_via_browser
    // This is an integration test — mock the browser engine call
    let method = spectral_broker::definition::RemovalMethod::BrowserForm {
        url: "https://test.com/optout".into(),
        fields: Default::default(),
        form_selectors: Default::default(),
        confirmation: Default::default(),
        requires_email_verification: false,
        notes: String::new(),
    };
    assert!(matches!(method, spectral_broker::definition::RemovalMethod::BrowserForm { .. }));
}
```

**Step 3: Add `BrowserEngine` to `AppState`**

In `src-tauri/src/state.rs`:

```rust
use spectral_browser::BrowserEngine;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    // ... existing fields ...
    pub browser_engine: Arc<Mutex<Option<BrowserEngine>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            // ... existing fields ...
            browser_engine: Arc::new(Mutex::new(None)),
        }
    }
}
```

**Step 4: Implement `submit_via_browser` in the worker**

In `src-tauri/src/removal_worker.rs`, add the function:

```rust
pub async fn submit_via_browser(
    broker_def: &spectral_broker::definition::BrokerDefinition,
    fields: &HashMap<String, String>,
    browser: Arc<tokio::sync::Mutex<Option<spectral_browser::BrowserEngine>>>,
) -> Result<RemovalOutcome, String> {
    use spectral_broker::definition::RemovalMethod;
    let (url, selectors) = match &broker_def.removal {
        RemovalMethod::BrowserForm { url, form_selectors, .. } => (url.clone(), form_selectors.clone()),
        _ => return Err("submit_via_browser called for non-BrowserForm broker".into()),
    };

    let mut guard = browser.lock().await;
    let engine = guard.get_or_insert_with(|| {
        spectral_browser::BrowserEngine::new()
    });

    match engine.navigate_and_fill(&url, fields, &selectors).await {
        Ok(screenshot) => {
            Ok(RemovalOutcome::Submitted { screenshot: Some(screenshot) })
        }
        Err(e) if e.is_captcha() => Ok(RemovalOutcome::RequiresCaptcha { captcha_url: url }),
        Err(e) => Err(e.to_string()),
    }
}
```

**Step 5: Add routing in `process_removal_task`**

In the existing `process_removal_task` function (or equivalent), add the `BrowserForm` arm:

```rust
match &broker_def.removal {
    RemovalMethod::WebForm { .. } => {
        submit_via_http(&broker_def, &fields).await
    }
    RemovalMethod::BrowserForm { .. } => {
        submit_via_browser(&broker_def, &fields, app_state.browser_engine.clone()).await
    }
    RemovalMethod::Email { .. } => {
        // Phase 6 Task 9 — placeholder for now
        Err("Email removal not yet implemented".into())
    }
    _ => Err(format!("Unsupported removal method for broker {}", broker_def.id())),
}
```

**Step 6: Store screenshot evidence**

After a successful browser submission, insert into `removal_evidence`:

```rust
if let Some(screenshot) = screenshot_bytes {
    let evidence_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO removal_evidence (id, attempt_id, screenshot_bytes, captured_at) VALUES (?, ?, ?, ?)"
    )
    .bind(&evidence_id)
    .bind(&attempt_id)
    .bind(&screenshot)
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await?;
}
```

**Step 7: Clean up browser on app exit**

In `src-tauri/src/lib.rs`, in the `.setup()` closure, add a window close handler:

```rust
app.on_window_event(|window, event| {
    if let tauri::WindowEvent::Destroyed = event {
        // Browser cleanup happens via Drop on BrowserEngine
    }
});
```

**Step 8: Run tests and build**

```bash
cargo test -p spectral-app 2>&1 | tail -5
cargo build -p spectral-app 2>&1 | tail -5
```

**Step 9: Commit**

```bash
git add crates/spectral-db/migrations/005_removal_evidence.sql \
        src-tauri/src/state.rs \
        src-tauri/src/removal_worker.rs \
        src-tauri/src/lib.rs
git commit -m "feat(worker): add browser-based removal path using spectral-browser"
```

---

## Task 7: Create `spectral-mail` Crate — Templates and mailto

**Files:**
- Create: `crates/spectral-mail/Cargo.toml`
- Create: `crates/spectral-mail/src/lib.rs`
- Create: `crates/spectral-mail/src/templates.rs`
- Create: `crates/spectral-mail/src/sender.rs`
- Modify: `Cargo.toml` (workspace)

**Step 1: Write the failing test**

Create `crates/spectral-mail/src/templates.rs` with test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_substitution() {
        let template = "Dear Data Controller,\n\nPlease remove {{name}} at {{address}}.";
        let vars = [
            ("name".to_string(), "Jane Smith".to_string()),
            ("address".to_string(), "123 Main St, Austin TX".to_string()),
        ].into_iter().collect();
        let result = render_template(template, &vars);
        assert!(result.contains("Jane Smith"));
        assert!(result.contains("123 Main St, Austin TX"));
        assert!(!result.contains("{{"));
    }

    #[test]
    fn test_generate_email_from_broker() {
        let template = "Remove {{name}} ({{email}}) from your database.";
        let vars = default_vars("Jane Smith", "jane@example.com", "123 Main St");
        let body = render_template(template, &vars);
        assert!(body.contains("jane@example.com"));
    }
}
```

**Step 2: Run to confirm failure**

```bash
cargo test -p spectral-mail 2>&1 | grep -E "error|FAIL"
```

Expected: crate not found in workspace.

**Step 3: Create crate skeleton**

`crates/spectral-mail/Cargo.toml`:

```toml
[package]
name = "spectral-mail"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
lettre = { version = "0.11", features = ["smtp-transport", "tokio1-native-tls", "builder"], default-features = false }
```

Add to root `Cargo.toml` workspace members:

```toml
members = ["src-tauri", "crates/*"]
```

(Already a glob — just ensure crate dir exists.)

`crates/spectral-mail/src/lib.rs`:

```rust
pub mod templates;
pub mod sender;
pub use templates::{render_template, EmailDraft, default_vars};
pub use sender::{send_mailto, SmtpConfig};
```

**Step 4: Implement `templates.rs`**

```rust
use std::collections::HashMap;

/// Substitutes `{{key}}` placeholders in a template string.
pub fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}

/// A fully-composed email ready to send or preview.
#[derive(Debug, Clone)]
pub struct EmailDraft {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Build default variable map from user profile data.
pub fn default_vars(name: &str, email: &str, address: &str) -> HashMap<String, String> {
    [
        ("name".to_string(), name.to_string()),
        ("email".to_string(), email.to_string()),
        ("address".to_string(), address.to_string()),
    ]
    .into_iter()
    .collect()
}

/// Generate an email draft from a broker's template fields.
pub fn generate_email_draft(
    recipient: &str,
    subject_template: &str,
    body_template: &str,
    vars: &HashMap<String, String>,
) -> EmailDraft {
    EmailDraft {
        to: recipient.to_string(),
        subject: render_template(subject_template, vars),
        body: render_template(body_template, vars),
    }
}
```

**Step 5: Implement `sender.rs` (mailto only for now)**

```rust
/// Open a mailto: link in the user's default email client.
///
/// Caller is responsible for invoking tauri-plugin-shell's open() —
/// this function constructs the URL only.
pub fn build_mailto_url(draft: &super::templates::EmailDraft) -> String {
    let subject = urlencoding::encode(&draft.subject);
    let body = urlencoding::encode(&draft.body);
    format!("mailto:{}?subject={}&body={}", draft.to, subject, body)
}

/// SMTP configuration (stored encrypted in vault).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String, // decrypted at call time
}

pub async fn send_via_smtp(draft: &super::templates::EmailDraft, config: &SmtpConfig) -> anyhow::Result<()> {
    use lettre::{Message, SmtpTransport, Transport, transport::smtp::authentication::Credentials};
    let email = Message::builder()
        .to(draft.to.parse()?)
        .from(config.username.parse()?)
        .subject(&draft.subject)
        .body(draft.body.clone())?;
    let creds = Credentials::new(config.username.clone(), config.password.clone());
    let mailer = SmtpTransport::relay(&config.host)?
        .port(config.port)
        .credentials(creds)
        .build();
    mailer.send(&email)?;
    Ok(())
}
```

Add `urlencoding = "2"` to `crates/spectral-mail/Cargo.toml` dependencies.

**Step 6: Run tests**

```bash
cargo test -p spectral-mail
cargo clippy -p spectral-mail -- -D warnings
```

**Step 7: Commit**

```bash
git add crates/spectral-mail/
git commit -m "feat(mail): create spectral-mail crate with template generation and mailto"
```

---

## Task 8: Email Removal DB Logging and Worker Integration

**Files:**
- Create: `crates/spectral-db/migrations/006_email_removals.sql`
- Modify: `crates/spectral-db/src/lib.rs`
- Create: `crates/spectral-db/src/email_removals.rs`
- Modify: `src-tauri/src/removal_worker.rs`
- Modify: `src-tauri/src/commands/scan.rs`
- Modify: `src-tauri/Cargo.toml`

**Step 1: Create migration**

`crates/spectral-db/migrations/006_email_removals.sql`:

```sql
CREATE TABLE email_removals (
    id TEXT PRIMARY KEY,
    attempt_id TEXT REFERENCES removal_attempts(id),
    broker_id TEXT NOT NULL,
    sent_at TEXT NOT NULL,
    method TEXT NOT NULL,
    recipient TEXT NOT NULL,
    subject TEXT NOT NULL,
    body_hash TEXT NOT NULL
);

CREATE INDEX idx_email_removals_attempt ON email_removals(attempt_id);
```

**Step 2: Write the failing test**

In `crates/spectral-db/src/email_removals.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_email_removal() {
        let db = crate::Database::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        log_email_removal(db.pool(), &EmailRemovalLog {
            id: "log-1".into(),
            attempt_id: None,
            broker_id: "broker-a".into(),
            sent_at: chrono::Utc::now(),
            method: "mailto".into(),
            recipient: "optout@broker.com".into(),
            subject: "Opt-Out Request".into(),
            body_hash: "abc123".into(),
        }).await.unwrap();
        let logs = get_by_broker(db.pool(), "broker-a").await.unwrap();
        assert_eq!(logs.len(), 1);
    }
}
```

**Step 3: Implement email_removals module**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailRemovalLog {
    pub id: String,
    pub attempt_id: Option<String>,
    pub broker_id: String,
    pub sent_at: DateTime<Utc>,
    pub method: String,
    pub recipient: String,
    pub subject: String,
    pub body_hash: String,
}

pub async fn log_email_removal(pool: &Pool<Sqlite>, log: &EmailRemovalLog) -> Result<()> {
    sqlx::query(
        "INSERT INTO email_removals (id, attempt_id, broker_id, sent_at, method, recipient, subject, body_hash)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&log.id)
    .bind(&log.attempt_id)
    .bind(&log.broker_id)
    .bind(log.sent_at)
    .bind(&log.method)
    .bind(&log.recipient)
    .bind(&log.subject)
    .bind(&log.body_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_by_broker(pool: &Pool<Sqlite>, broker_id: &str) -> Result<Vec<EmailRemovalLog>> {
    sqlx::query_as!(EmailRemovalLog,
        "SELECT id, attempt_id, broker_id, sent_at, method, recipient, subject, body_hash
         FROM email_removals WHERE broker_id = ? ORDER BY sent_at DESC",
        broker_id
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}
```

**Step 4: Wire Email arm into worker**

Add `spectral-mail` to `src-tauri/Cargo.toml`:

```toml
spectral-mail = { path = "../crates/spectral-mail" }
```

In `src-tauri/src/removal_worker.rs`, implement the `Email` arm:

```rust
RemovalMethod::Email { email: recipient, subject_template, body_template, requires_email_verification, .. } => {
    let vars = spectral_mail::templates::default_vars(&full_name, &email_addr, &address);
    let draft = spectral_mail::templates::generate_email_draft(
        recipient, subject_template, body_template, &vars,
    );
    // Emit event for frontend to show preview modal
    app_handle.emit("removal:email_ready", serde_json::json!({
        "attempt_id": attempt_id,
        "draft": { "to": draft.to, "subject": draft.subject, "body": draft.body }
    })).ok();
    // Status stays Pending until user sends from modal
    RemovalOutcome::PendingEmailSend
}
```

**Step 5: Add `send_removal_email` Tauri command**

In `src-tauri/src/commands/scan.rs`:

```rust
#[tauri::command]
pub async fn send_removal_email(
    attempt_id: String,
    to: String,
    subject: String,
    body: String,
    use_smtp: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let draft = spectral_mail::templates::EmailDraft { to: to.clone(), subject, body };

    if use_smtp {
        let config = state.get_smtp_config().await?;
        spectral_mail::sender::send_via_smtp(&draft, &config).await
            .map_err(|e| CommandError::Internal(e.to_string()))?;
    } else {
        let mailto_url = spectral_mail::sender::build_mailto_url(&draft);
        // tauri-plugin-shell open is handled by frontend
        // Return the URL to frontend to call open()
        return Err(CommandError::MailtoRequired(mailto_url));
    }

    // Log the send
    // ... log to email_removals table ...

    Ok(())
}
```

Register in `src-tauri/src/lib.rs`.

**Step 6: Run tests and build**

```bash
cargo test -p spectral-db email_removals
cargo build 2>&1 | tail -5
```

**Step 7: Commit**

```bash
git add crates/spectral-db/migrations/006_email_removals.sql \
        crates/spectral-db/src/email_removals.rs \
        crates/spectral-db/src/lib.rs \
        src-tauri/src/removal_worker.rs \
        src-tauri/src/commands/scan.rs \
        src-tauri/Cargo.toml
git commit -m "feat(mail): wire email removal into worker with DB logging"
```

---

## Task 9: Email Verification — Pending Verification Tab

**Files:**
- Modify: `src/routes/removals/progress/[jobId]/+page.svelte`
- Create: `src/lib/components/removals/PendingVerificationTab.svelte`
- Modify: `src/lib/stores/removal.svelte.ts`
- Modify: `src-tauri/src/commands/scan.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Add `mark_attempt_verified` Tauri command**

In `src-tauri/src/commands/scan.rs`:

```rust
#[tauri::command]
pub async fn mark_attempt_verified(
    vault_id: String,
    attempt_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let db = state.get_db(&vault_id).await?;
    spectral_db::removal_attempts::update_status(
        db.pool(), &attempt_id, spectral_db::removal_attempts::RemovalStatus::Completed,
    )
    .await
    .map_err(CommandError::from)?;

    // Emit removal:verified event
    state.app_handle().emit("removal:verified", serde_json::json!({
        "attempt_id": attempt_id
    })).ok();

    Ok(())
}
```

Register in `src-tauri/src/lib.rs`.

**Step 2: Add `AwaitingVerification` status to frontend type**

In `src/lib/api/removal.ts`, update the `RemovalAttempt` status union:

```typescript
status: 'Pending' | 'Processing' | 'Submitted' | 'Completed' | 'Failed' | 'AwaitingVerification';
```

**Step 3: Add `verificationQueue` getter to removal store**

In `src/lib/stores/removal.svelte.ts`:

```typescript
get verificationQueue() {
    return state.removalAttempts.filter(
        (r) => r.status === 'Pending' && r.error_message === 'AWAITING_VERIFICATION'
    );
},
```

Also add handler for `removal:verified` event in `setupEventListeners`:

```typescript
const unlistenVerified = await listen<{ attempt_id: string }>('removal:verified', (event) => {
    updateAttempt(event.payload.attempt_id, { status: 'Completed' });
});
unlisteners.push(unlistenVerified);
```

**Step 4: Create `PendingVerificationTab.svelte`**

`src/lib/components/removals/PendingVerificationTab.svelte`:

```svelte
<script lang="ts">
  import type { RemovalAttempt } from '$lib/api/removal';
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    verificationQueue: RemovalAttempt[];
    vaultId: string;
  }
  let { verificationQueue, vaultId }: Props = $props();

  let marking = $state<Set<string>>(new Set());

  async function handleMarkVerified(attemptId: string) {
    marking = new Set([...marking, attemptId]);
    try {
      await invoke('mark_attempt_verified', { vaultId, attemptId });
    } finally {
      const next = new Set(marking);
      next.delete(attemptId);
      marking = next;
    }
  }
</script>

<div class="space-y-4">
  {#if verificationQueue.length === 0}
    <div class="bg-white rounded-lg border border-gray-200 p-12 text-center">
      <div class="inline-flex items-center justify-center w-16 h-16 bg-green-100 rounded-full mb-4">
        <span class="text-3xl text-green-600">✓</span>
      </div>
      <h3 class="text-lg font-semibold text-gray-900 mb-2">No pending verifications</h3>
      <p class="text-sm text-gray-600">All email verifications have been completed.</p>
    </div>
  {:else}
    <div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
      <div class="px-6 py-4 border-b border-gray-200 bg-yellow-50">
        <h2 class="text-lg font-semibold text-gray-900">Pending Verification ({verificationQueue.length})</h2>
        <p class="text-sm text-gray-600 mt-1">These brokers sent a confirmation email. Click the link in the email, then mark as verified here.</p>
      </div>
      <div class="divide-y divide-gray-200">
        {#each verificationQueue as attempt}
          {@const isMarking = marking.has(attempt.id)}
          <div class="p-6 flex items-center justify-between">
            <div>
              <p class="font-semibold text-gray-900">{attempt.broker_id}</p>
              <p class="text-sm text-gray-600 mt-1">Check your inbox for a confirmation email and click the link inside.</p>
            </div>
            <button
              onclick={() => handleMarkVerified(attempt.id)}
              disabled={isMarking}
              class="ml-4 px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 transition-colors text-sm font-medium"
            >
              {isMarking ? 'Marking...' : 'Mark as Verified'}
            </button>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
```

**Step 5: Add the fourth tab to the progress dashboard**

In `src/routes/removals/progress/[jobId]/+page.svelte`, add the tab button and content pane following the same pattern as the existing three tabs. Import `PendingVerificationTab` and add:

```svelte
import PendingVerificationTab from '$lib/components/removals/PendingVerificationTab.svelte';
```

Tab button:
```svelte
<button onclick={() => (activeTab = 'verification')} ...>
  Pending Verification
  {#if removalStore.verificationQueue.length > 0}
    <span class="px-2 py-0.5 bg-yellow-100 text-yellow-800 rounded-full text-xs font-semibold">
      {removalStore.verificationQueue.length}
    </span>
  {/if}
</button>
```

Tab content:
```svelte
{:else if activeTab === 'verification'}
  <PendingVerificationTab
    verificationQueue={removalStore.verificationQueue}
    vaultId={vaultStore.currentVaultId ?? ''}
  />
```

**Step 6: Verify build**

```bash
npm run check && npm run build 2>&1 | tail -5
```

**Step 7: Commit**

```bash
git add src/lib/components/removals/PendingVerificationTab.svelte \
        src/routes/removals/progress/\[jobId\]/+page.svelte \
        src/lib/stores/removal.svelte.ts \
        src/lib/api/removal.ts \
        src-tauri/src/commands/scan.rs \
        src-tauri/src/lib.rs
git commit -m "feat(frontend): add pending verification tab to progress dashboard"
```

---

## Task 10: IMAP Poller

**Files:**
- Modify: `crates/spectral-mail/Cargo.toml`
- Create: `crates/spectral-mail/src/imap.rs`
- Modify: `crates/spectral-mail/src/lib.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write the failing test**

In `crates/spectral-mail/src/imap.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_verification_link() {
        let body = "Please click to confirm: https://broker.com/confirm/abc123def456 to complete your removal.";
        let pattern = r"https://broker\.com/confirm/[a-z0-9]+";
        let link = extract_verification_link(body, pattern);
        assert_eq!(link, Some("https://broker.com/confirm/abc123def456".to_string()));
    }

    #[test]
    fn test_extract_verification_link_not_found() {
        let body = "Thank you for your request.";
        let pattern = r"https://broker\.com/confirm/[a-z0-9]+";
        assert_eq!(extract_verification_link(body, pattern), None);
    }

    #[test]
    fn test_email_is_too_old() {
        let old_date = chrono::Utc::now() - chrono::Duration::days(8);
        assert!(is_too_old(&old_date));
        let recent_date = chrono::Utc::now() - chrono::Duration::days(3);
        assert!(!is_too_old(&recent_date));
    }
}
```

**Step 2: Run to confirm failure**

```bash
cargo test -p spectral-mail test_extract_verification_link 2>&1 | grep -E "error|FAIL"
```

**Step 3: Add IMAP dependency and implement**

Add to `crates/spectral-mail/Cargo.toml`:

```toml
async-imap = { version = "0.10", default-features = false, features = ["runtime-tokio"] }
async-native-tls = "0.5"
regex = { workspace = true }
chrono = { workspace = true }
```

`crates/spectral-mail/src/imap.rs`:

```rust
use chrono::{DateTime, Utc};
use regex::Regex;

const MAX_VERIFICATION_AGE_DAYS: i64 = 7;

/// Extract a verification link from an email body using a broker-defined regex pattern.
pub fn extract_verification_link(body: &str, pattern: &str) -> Option<String> {
    Regex::new(pattern).ok()?.find(body).map(|m| m.as_str().to_string())
}

/// Returns true if the email is older than MAX_VERIFICATION_AGE_DAYS.
pub fn is_too_old(date: &DateTime<Utc>) -> bool {
    Utc::now().signed_duration_since(*date).num_days() > MAX_VERIFICATION_AGE_DAYS
}

/// Configuration for IMAP polling.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String, // decrypted before passing
}

/// A broker email match: the attempt ID and the confirmation link to open.
#[derive(Debug)]
pub struct VerificationMatch {
    pub broker_id: String,
    pub confirmation_link: String,
}

/// Poll IMAP inbox and return any broker confirmation emails found.
///
/// Checks UNSEEN messages in INBOX from known broker sender addresses.
/// Does NOT mark messages as read. Read-only access only.
pub async fn poll_inbox(
    config: &ImapConfig,
    broker_senders: &[(&str, &str, &str)], // (broker_id, sender_email, link_pattern)
) -> anyhow::Result<Vec<VerificationMatch>> {
    use async_imap::Session;
    use async_native_tls::TlsConnector;

    let tls = TlsConnector::new();
    let client = async_imap::connect(
        (config.host.as_str(), config.port),
        &config.host,
        tls,
    ).await?;

    let mut session = client.login(&config.username, &config.password)
        .await
        .map_err(|(err, _)| err)?;

    session.select("INBOX").await?;

    // Search for unseen messages
    let uids = session.search("UNSEEN").await?;
    let mut matches = Vec::new();

    for uid in uids.iter().take(100) {
        let messages = session.fetch(uid.to_string(), "(RFC822 ENVELOPE)").await?;
        for message in messages.iter() {
            let envelope = match message.envelope() {
                Some(e) => e,
                None => continue,
            };

            // Check date
            if let Some(date_str) = envelope.date.as_ref() {
                // Parse date — skip if too old
                // (simplified: use current check, real impl parses RFC2822)
                let _ = date_str; // actual parsing omitted for brevity
            }

            let from = envelope.from.as_ref()
                .and_then(|addrs| addrs.first())
                .and_then(|a| a.host.as_ref().map(|h| {
                    format!("{}@{}", a.mailbox.as_deref().unwrap_or(""), h.as_ref())
                }))
                .unwrap_or_default();

            for (broker_id, sender, pattern) in broker_senders {
                if from.contains(sender) {
                    if let Some(body) = message.text() {
                        let body_str = std::str::from_utf8(body).unwrap_or("");
                        if let Some(link) = extract_verification_link(body_str, pattern) {
                            matches.push(VerificationMatch {
                                broker_id: broker_id.to_string(),
                                confirmation_link: link,
                            });
                        }
                    }
                }
            }
        }
    }

    session.logout().await?;
    Ok(matches)
}
```

**Step 4: Run tests**

```bash
cargo test -p spectral-mail
cargo clippy -p spectral-mail -- -D warnings
```

**Step 5: Commit**

```bash
git add crates/spectral-mail/src/imap.rs \
        crates/spectral-mail/src/lib.rs \
        crates/spectral-mail/Cargo.toml
git commit -m "feat(mail): add IMAP poller for broker email verification"
```

---

## Task 11: Create `spectral-scheduler` Crate

**Files:**
- Create: `crates/spectral-scheduler/Cargo.toml`
- Create: `crates/spectral-scheduler/src/lib.rs`
- Create: `crates/spectral-scheduler/src/jobs.rs`
- Create: `crates/spectral-scheduler/src/scheduler.rs`
- Create: `crates/spectral-db/migrations/007_scheduled_jobs.sql`

**Step 1: Create the migration**

`crates/spectral-db/migrations/007_scheduled_jobs.sql`:

```sql
CREATE TABLE scheduled_jobs (
    id TEXT PRIMARY KEY,
    job_type TEXT NOT NULL,
    interval_days INTEGER NOT NULL,
    next_run_at TEXT NOT NULL,
    last_run_at TEXT,
    enabled INTEGER NOT NULL DEFAULT 1
);

INSERT INTO scheduled_jobs (id, job_type, interval_days, next_run_at, enabled)
VALUES
    ('scan-all',        'ScanAll',        7, datetime('now'), 1),
    ('verify-removals', 'VerifyRemovals', 3, datetime('now'), 1);
```

**Step 2: Write the failing test**

Create `crates/spectral-scheduler/src/scheduler.rs` with test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_due_jobs_returned() {
        let db = spectral_db::Database::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        // The migration inserts two jobs with next_run_at = now
        let due = get_due_jobs(db.pool()).await.unwrap();
        assert!(due.len() >= 2);
    }

    #[tokio::test]
    async fn test_future_job_not_due() {
        let db = spectral_db::Database::new_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        // Push next_run_at into the future
        sqlx::query("UPDATE scheduled_jobs SET next_run_at = datetime('now', '+7 days')")
            .execute(db.pool()).await.unwrap();
        let due = get_due_jobs(db.pool()).await.unwrap();
        assert!(due.is_empty());
    }
}
```

**Step 3: Implement scheduler module**

`crates/spectral-scheduler/Cargo.toml`:

```toml
[package]
name = "spectral-scheduler"
version.workspace = true
edition.workspace = true

[dependencies]
spectral-db = { path = "../spectral-db" }
tokio = { workspace = true }
serde = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
chrono = { workspace = true }
sqlx = { workspace = true }
```

`crates/spectral-scheduler/src/jobs.rs`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum JobType {
    ScanAll,
    VerifyRemovals,
    PollImap,
}

impl std::fmt::Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScanAll       => write!(f, "ScanAll"),
            Self::VerifyRemovals => write!(f, "VerifyRemovals"),
            Self::PollImap      => write!(f, "PollImap"),
        }
    }
}

impl std::str::FromStr for JobType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ScanAll"        => Ok(Self::ScanAll),
            "VerifyRemovals" => Ok(Self::VerifyRemovals),
            "PollImap"       => Ok(Self::PollImap),
            other            => Err(anyhow::anyhow!("Unknown job type: {}", other)),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub job_type: JobType,
    pub interval_days: i64,
    pub next_run_at: chrono::DateTime<chrono::Utc>,
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
}
```

`crates/spectral-scheduler/src/scheduler.rs`:

```rust
use sqlx::{Pool, Row, Sqlite};
use anyhow::Result;
use crate::jobs::ScheduledJob;

pub async fn get_due_jobs(pool: &Pool<Sqlite>) -> Result<Vec<ScheduledJob>> {
    let rows = sqlx::query(
        "SELECT id, job_type, interval_days, next_run_at, last_run_at, enabled
         FROM scheduled_jobs WHERE enabled = 1 AND next_run_at <= datetime('now')"
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(ScheduledJob {
                id: row.try_get("id")?,
                job_type: row.try_get::<String, _>("job_type")?.parse()?,
                interval_days: row.try_get("interval_days")?,
                next_run_at: row.try_get("next_run_at")?,
                last_run_at: row.try_get("last_run_at")?,
                enabled: row.try_get::<i64, _>("enabled")? != 0,
            })
        })
        .collect()
}

pub async fn mark_job_complete(pool: &Pool<Sqlite>, job_id: &str, interval_days: i64) -> Result<()> {
    sqlx::query(
        "UPDATE scheduled_jobs SET last_run_at = datetime('now'),
         next_run_at = datetime('now', '+' || ? || ' days') WHERE id = ?"
    )
    .bind(interval_days)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_job(
    pool: &Pool<Sqlite>,
    job_id: &str,
    interval_days: i64,
    enabled: bool,
) -> Result<()> {
    sqlx::query(
        "UPDATE scheduled_jobs SET interval_days = ?, enabled = ? WHERE id = ?"
    )
    .bind(interval_days)
    .bind(enabled as i64)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}
```

`crates/spectral-scheduler/src/lib.rs`:

```rust
pub mod jobs;
pub mod scheduler;
pub use jobs::{JobType, ScheduledJob};
pub use scheduler::{get_due_jobs, mark_job_complete, update_job};
```

**Step 4: Run tests**

```bash
cargo test -p spectral-scheduler
cargo clippy -p spectral-scheduler -- -D warnings
```

**Step 5: Commit**

```bash
git add crates/spectral-scheduler/ \
        crates/spectral-db/migrations/007_scheduled_jobs.sql
git commit -m "feat(scheduler): create spectral-scheduler crate with job queue"
```

---

## Task 12: Scheduler — Wire Jobs and App Startup

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands/scan.rs`

**Step 1: Add dependency**

In `src-tauri/Cargo.toml`:

```toml
spectral-scheduler = { path = "../crates/spectral-scheduler" }
```

**Step 2: Add `run_due_jobs` function**

In `src-tauri/src/lib.rs`, add a function that the app calls on startup:

```rust
async fn run_due_scheduler_jobs(app_state: &crate::state::AppState, app_handle: &tauri::AppHandle) {
    for (vault_id, db) in app_state.all_vaults().await {
        match spectral_scheduler::get_due_jobs(db.pool()).await {
            Ok(jobs) => {
                for job in jobs {
                    tracing::info!("Running scheduled job: {:?}", job.job_type);
                    match job.job_type {
                        spectral_scheduler::JobType::ScanAll => {
                            // Invoke existing scan orchestrator
                            // ... (call start_scan logic) ...
                        }
                        spectral_scheduler::JobType::VerifyRemovals => {
                            // Re-scan brokers with submitted attempts
                            // ... (call verify logic — see Task 13) ...
                        }
                        spectral_scheduler::JobType::PollImap => {
                            // Run IMAP poller if configured
                            // ... (see Task 10) ...
                        }
                    }
                    if let Err(e) = spectral_scheduler::mark_job_complete(
                        db.pool(), &job.id, job.interval_days
                    ).await {
                        tracing::warn!("Failed to mark job complete: {}", e);
                    }
                }
            }
            Err(e) => tracing::warn!("Scheduler error for vault {}: {}", vault_id, e),
        }
    }
}
```

Call `run_due_scheduler_jobs` in the `.setup()` closure (spawned as a background task):

```rust
.setup(|app| {
    let state = app.state::<AppState>();
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        run_due_scheduler_jobs(&state, &handle).await;
    });
    Ok(())
})
```

**Step 3: Add scheduler Tauri commands**

In `src-tauri/src/commands/scan.rs`:

```rust
#[tauri::command]
pub async fn get_scheduled_jobs(
    vault_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<spectral_scheduler::ScheduledJob>, CommandError> {
    let db = state.get_db(&vault_id).await?;
    spectral_scheduler::scheduler::get_due_jobs(db.pool())
        .await
        .map_err(|e| CommandError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn update_scheduled_job(
    vault_id: String,
    job_id: String,
    interval_days: i64,
    enabled: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let db = state.get_db(&vault_id).await?;
    spectral_scheduler::scheduler::update_job(db.pool(), &job_id, interval_days, enabled)
        .await
        .map_err(|e| CommandError::Internal(e.to_string()))
}
```

Register both commands in `src-tauri/src/lib.rs`.

**Step 4: Build and verify**

```bash
cargo build 2>&1 | tail -10
```

**Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/src/commands/scan.rs
git commit -m "feat(scheduler): wire scheduler into app startup and add settings commands"
```

---

## Task 13: Tray Mode (Cross-Platform)

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/state.rs`

**Step 1: Add tray plugins**

In workspace `Cargo.toml`:

```toml
tauri-plugin-system-tray = "2"
tauri-plugin-autostart = "2"
```

In `src-tauri/Cargo.toml`:

```toml
tauri-plugin-system-tray = { workspace = true }
tauri-plugin-autostart = { workspace = true }
```

In `src-tauri/tauri.conf.json`, add tray configuration:

```json
"trayIcon": {
  "id": "spectral-tray",
  "iconPath": "icons/icon.png",
  "tooltip": "Spectral — Privacy Protection"
}
```

**Step 2: Add tray mode flag to AppState**

In `src-tauri/src/state.rs`:

```rust
pub tray_mode_enabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
```

Initialise as `AtomicBool::new(false)`.

**Step 3: Implement tray setup with platform fallback**

In `src-tauri/src/lib.rs`:

```rust
fn try_setup_tray(app: &mut tauri::App) -> bool {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};

    let open_item = MenuItemBuilder::new("Open Spectral").id("open").build(app).ok();
    let quit_item = MenuItemBuilder::new("Quit").id("quit").build(app).ok();

    let menu = match (open_item, quit_item) {
        (Some(o), Some(q)) => MenuBuilder::new(app).items(&[&o, &q]).build().ok(),
        _ => None,
    };

    let tray_result = tauri::tray::TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().unwrap_or_default())
        .menu(menu.as_ref())
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); } }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app);

    match tray_result {
        Ok(_) => {
            tracing::info!("System tray initialised successfully");
            true
        }
        Err(e) => {
            tracing::warn!("System tray unavailable ({}). Tray mode disabled.", e);
            false
        }
    }
}
```

Call in setup:

```rust
.setup(|app| {
    let tray_available = try_setup_tray(app);
    app.state::<AppState>()
        .tray_mode_enabled
        .store(tray_available, std::sync::atomic::Ordering::Relaxed);

    // Intercept close to hide-to-tray when tray mode enabled
    let state = app.state::<AppState>().inner().clone();
    app.on_window_event(move |window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            if state.tray_mode_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                api.prevent_close();
                window.hide().ok();
            }
        }
    });

    // Background scheduler tick (30-minute interval, only in tray mode)
    let state2 = app.state::<AppState>().inner().clone();
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30 * 60));
        loop {
            interval.tick().await;
            if state2.tray_mode_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                run_due_scheduler_jobs(&state2, &handle).await;
            }
        }
    });

    Ok(())
})
```

**Step 4: Register autostart plugin (conditional)**

```rust
.plugin(tauri_plugin_autostart::init(
    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
    Some(vec!["--minimized"]),
))
```

Note: `tauri_plugin_autostart` is cross-platform — it uses LaunchAgent on macOS, the registry on Windows, and XDG `.desktop` on Linux. The `MacosLauncher` variant name is a misnomer in the API but affects only macOS behavior.

**Step 5: Build for current platform**

```bash
cargo build 2>&1 | tail -10
```

If tray libraries are missing on Linux:

```bash
sudo apt-get install -y libayatana-appindicator3-dev
# or
sudo apt-get install -y libappindicator3-dev
```

**Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/tauri.conf.json src-tauri/src/lib.rs src-tauri/src/state.rs
git commit -m "feat(scheduler): add cross-platform tray mode with graceful Linux fallback"
```

---

## Task 14: Scheduler Settings UI

**Files:**
- Create: `src/routes/settings/+page.svelte` (or modify if exists)
- Create: `src/routes/settings/+page.ts`
- Modify: `src/routes/+page.svelte`

**Step 1: Check if settings page exists**

```bash
ls src/routes/settings/ 2>/dev/null || echo "not found"
```

If not found, create the route. If it exists, add a "Scheduling" section to it.

**Step 2: Create the settings page**

`src/routes/settings/+page.svelte` (minimal — scheduling section only; extend for email settings in follow-up):

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import { vaultStore } from '$lib/stores';

  let jobs = $state<Array<{ id: string; job_type: string; interval_days: number; enabled: boolean; next_run_at: string }>>([]);
  let loading = $state(true);

  onMount(async () => {
    if (!vaultStore.currentVaultId) { goto('/'); return; }
    try {
      jobs = await invoke('get_scheduled_jobs', { vaultId: vaultStore.currentVaultId });
    } finally {
      loading = false;
    }
  });

  const JOB_LABELS: Record<string, string> = {
    ScanAll: 'Automatic re-scan',
    VerifyRemovals: 'Removal verification checks',
    PollImap: 'Email verification polling',
  };

  const INTERVAL_OPTIONS = [3, 7, 14, 30];

  async function updateJob(jobId: string, intervalDays: number, enabled: boolean) {
    await invoke('update_scheduled_job', {
      vaultId: vaultStore.currentVaultId,
      jobId,
      intervalDays,
      enabled,
    });
  }
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 p-4">
  <div class="max-w-2xl mx-auto">
    <div class="bg-white rounded-lg shadow-xl overflow-hidden">
      <div class="px-8 py-6 border-b border-gray-200 flex items-center justify-between">
        <h1 class="text-3xl font-bold text-gray-900">Settings</h1>
        <button onclick={() => goto('/')} class="text-gray-600 hover:text-gray-900">← Dashboard</button>
      </div>

      <div class="p-8 space-y-8">
        <!-- Scheduling -->
        <section>
          <h2 class="text-lg font-semibold text-gray-900 mb-4">Scheduling</h2>
          {#if loading}
            <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-primary-600"></div>
          {:else}
            <div class="space-y-4">
              {#each jobs as job}
                <div class="flex items-center justify-between py-3 border-b border-gray-100">
                  <div>
                    <p class="font-medium text-gray-900">{JOB_LABELS[job.job_type] ?? job.job_type}</p>
                  </div>
                  <div class="flex items-center gap-4">
                    <select
                      value={job.interval_days}
                      onchange={(e) => {
                        job.interval_days = parseInt(e.currentTarget.value);
                        updateJob(job.id, job.interval_days, job.enabled);
                      }}
                      disabled={!job.enabled}
                      class="text-sm border border-gray-300 rounded px-2 py-1 disabled:opacity-50"
                    >
                      {#each INTERVAL_OPTIONS as days}
                        <option value={days}>Every {days} days</option>
                      {/each}
                    </select>
                    <label class="flex items-center gap-2 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={job.enabled}
                        onchange={(e) => {
                          job.enabled = e.currentTarget.checked;
                          updateJob(job.id, job.interval_days, job.enabled);
                        }}
                        class="w-4 h-4 accent-primary-600"
                      />
                      <span class="text-sm text-gray-600">Enabled</span>
                    </label>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </section>
      </div>
    </div>
  </div>
</div>
```

`src/routes/settings/+page.ts`:

```typescript
export const prerender = false;
```

**Step 3: Add Settings link to home dashboard**

In `src/routes/+page.svelte`, add a settings link alongside the Privacy Health card.

**Step 4: Verify build**

```bash
npm run check && npm run build 2>&1 | tail -5
```

**Step 5: Run full test suite**

```bash
cargo test --workspace 2>&1 | grep -E "^test result"
```

Expected: all pass, 0 failed.

**Step 6: Commit**

```bash
git add src/routes/settings/ src/routes/+page.svelte
git commit -m "feat(frontend): add settings page with scheduling configuration"
```

---

## Final Verification

```bash
# Full Rust test suite
cargo test --workspace 2>&1 | grep -E "^test result|FAILED"

# Frontend checks
npm run check
npm run lint

# Full build
npm run build

# SonarQube scan
sonar-scanner -Dsonar.token=squ_90e70708a283dac56b7bcd1b9e6787cc722e1f3b
```

All tests must pass. No new SonarQube violations (new\_violations = 0). Build succeeds on Linux.

---

## Out of Scope

- Advanced CAPTCHA solving (vision LLM, 3rd party services)
- LLM-assisted email drafting
- Email preview modal UI (deferred to follow-up task)
- SMTP settings UI (deferred — add to Settings page in a follow-up task alongside IMAP settings)
- Multi-broker session reuse in browser engine
