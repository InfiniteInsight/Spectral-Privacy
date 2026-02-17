# Phase 5 Frontend: Removal Submission UI - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build real-time UI for monitoring removal submission progress with tabbed interface for managing CAPTCHA and failed queues.

**Architecture:** Single-page Svelte component with local tab state, Tauri event listeners for real-time updates, and Svelte 5 stores for state management.

**Tech Stack:** SvelteKit, Svelte 5 runes, TypeScript, Tailwind CSS, Tauri APIs (invoke, listen, shell)

---

## Task 1: Add Backend Command for Query by Job ID

**Files:**
- Modify: `src-tauri/src/commands/scan.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `crates/spectral-db/src/removal_attempts.rs`

**Background:** The design requires querying removal attempts by `job_id` to load initial state. Currently, we can only query by finding_id. Need to add this query capability.

**Step 1: Add database query function**

Modify `crates/spectral-db/src/removal_attempts.rs`, add after `get_failed_queue`:

```rust
/// Get all removal attempts for a specific batch job.
pub async fn get_by_job_id(
    pool: &Pool<Sqlite>,
    job_id: &str,
) -> Result<Vec<RemovalAttempt>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, finding_id, broker_id, status, created_at, submitted_at, completed_at, error_message
         FROM removal_attempts
         WHERE job_id = ?
         ORDER BY created_at ASC",
    )
    .bind(job_id)
    .fetch_all(pool)
    .await?;

    parse_removal_attempts_from_rows(rows)
}
```

**Step 2: Add job_id column to schema**

Wait - check if `job_id` column exists in removal_attempts table. If not, we need a migration.

Run: `sqlite3 path/to/vault.db ".schema removal_attempts"`

If `job_id` doesn't exist, add migration in `crates/spectral-db/src/migrations.rs`:

```rust
// Add after existing migrations
sqlx::query(
    "ALTER TABLE removal_attempts ADD COLUMN job_id TEXT"
)
.execute(pool)
.await?;
```

**Alternative:** Store job_id in process_removal_batch and update removal_attempts when creating tasks. Check design doc - job_id comes from `process_removal_batch` result.

**Decision:** For Phase 5 frontend MVP, we can query by finding_id and filter by scan_job_id through findings table. Skip job_id column for now.

**Revised Step 1: Add query by scan_job_id**

Modify `crates/spectral-db/src/removal_attempts.rs`, add:

```rust
/// Get all removal attempts for a scan job (via findings table).
pub async fn get_by_scan_job_id(
    pool: &Pool<Sqlite>,
    scan_job_id: &str,
) -> Result<Vec<RemovalAttempt>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT ra.id, ra.finding_id, ra.broker_id, ra.status, ra.created_at,
                ra.submitted_at, ra.completed_at, ra.error_message
         FROM removal_attempts ra
         INNER JOIN findings f ON ra.finding_id = f.id
         WHERE f.broker_scan_id IN (
           SELECT id FROM broker_scans WHERE scan_job_id = ?
         )
         ORDER BY ra.created_at ASC",
    )
    .bind(scan_job_id)
    .fetch_all(pool)
    .await?;

    parse_removal_attempts_from_rows(rows)
}
```

**Step 2: Add Tauri command**

Modify `src-tauri/src/commands/scan.rs`, add after `retry_removal`:

```rust
#[tauri::command]
pub async fn get_removal_attempts_by_scan_job<R: tauri::Runtime>(
    state: State<'_, AppState>,
    vault_id: String,
    scan_job_id: String,
) -> Result<Vec<spectral_db::removal_attempts::RemovalAttempt>, String> {
    let vault = state
        .get_vault(&vault_id)
        .ok_or_else(|| "Vault not found or not unlocked".to_string())?;

    let db = vault
        .database()
        .map_err(|e| format!("Failed to access database: {}", e))?;

    spectral_db::removal_attempts::get_by_scan_job_id(db.pool(), &scan_job_id)
        .await
        .map_err(|e| format!("Failed to query removal attempts: {}", e))
}
```

**Step 3: Register command**

Modify `src-tauri/src/lib.rs`, add to `invoke_handler`:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    commands::scan::get_removal_attempts_by_scan_job,
])
```

**Step 4: Test query**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS (no new tests, just verify no regressions)

**Step 5: Commit**

```bash
git add crates/spectral-db/src/removal_attempts.rs src-tauri/src/commands/scan.rs src-tauri/src/lib.rs
git commit -m "feat(db): add query for removal attempts by scan job ID

Add get_by_scan_job_id database query and Tauri command to fetch
removal attempts for a scan job via findings table join.

Enables Phase 5 frontend to load initial removal state.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Create Removal API Layer

**Files:**
- Create: `src/lib/api/removal.ts`

**Step 1: Create API file with TypeScript interfaces**

Create `src/lib/api/removal.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface BatchSubmissionResult {
	job_id: string;
	total_count: number;
	queued_count: number;
}

export interface RemovalAttempt {
	id: string;
	finding_id: string;
	broker_id: string;
	status: 'Pending' | 'Processing' | 'Submitted' | 'Completed' | 'Failed';
	created_at: string;
	submitted_at: string | null;
	completed_at: string | null;
	error_message: string | null;
}

export const removalAPI = {
	/**
	 * Process a batch of removal attempts concurrently
	 */
	async processBatch(
		vaultId: string,
		removalAttemptIds: string[]
	): Promise<BatchSubmissionResult> {
		return await invoke<BatchSubmissionResult>('process_removal_batch', {
			vaultId,
			removalAttemptIds
		});
	},

	/**
	 * Get removal attempts by scan job ID
	 */
	async getByScanJob(vaultId: string, scanJobId: string): Promise<RemovalAttempt[]> {
		return await invoke<RemovalAttempt[]>('get_removal_attempts_by_scan_job', {
			vaultId,
			scanJobId
		});
	},

	/**
	 * Get CAPTCHA queue
	 */
	async getCaptchaQueue(vaultId: string): Promise<RemovalAttempt[]> {
		return await invoke<RemovalAttempt[]>('get_captcha_queue', { vaultId });
	},

	/**
	 * Get failed queue
	 */
	async getFailedQueue(vaultId: string): Promise<RemovalAttempt[]> {
		return await invoke<RemovalAttempt[]>('get_failed_queue', { vaultId });
	},

	/**
	 * Retry a failed removal attempt
	 */
	async retry(vaultId: string, removalAttemptId: string): Promise<void> {
		return await invoke('retry_removal', { vaultId, removalAttemptId });
	}
};
```

**Step 2: Export from index**

Modify `src/lib/api/index.ts`, add:

```typescript
export * from './removal';
```

**Step 3: Verify TypeScript compiles**

Run: `npm run check`
Expected: No TypeScript errors

**Step 4: Commit**

```bash
git add src/lib/api/removal.ts src/lib/api/index.ts
git commit -m "feat(frontend): add removal API command wrappers

Add TypeScript interfaces and command wrappers for removal
submission backend: batch processing, queue queries, retry.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Create Removal Store with State Management

**Files:**
- Create: `src/lib/stores/removal.svelte.ts`
- Modify: `src/lib/stores/index.ts`

**Step 1: Create removal store with Svelte 5 runes**

Create `src/lib/stores/removal.svelte.ts`:

```typescript
import { removalAPI, type RemovalAttempt, type BatchSubmissionResult } from '$lib/api/removal';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

class RemovalStore {
	// State
	removalAttempts = $state<RemovalAttempt[]>([]);
	scanJobId = $state<string | null>(null);
	batchJobId = $state<string | null>(null);
	loading = $state(false);
	error = $state<string | null>(null);

	// Event listener cleanup functions
	private unlistenFns: UnlistenFn[] = [];

	// Derived counts
	submitted = $derived(
		this.removalAttempts.filter(
			(r) => r.status === 'Submitted' || r.status === 'Completed'
		)
	);

	captchaQueue = $derived(
		this.removalAttempts.filter(
			(r) => r.status === 'Pending' && r.error_message?.startsWith('CAPTCHA_REQUIRED')
		)
	);

	failedQueue = $derived(this.removalAttempts.filter((r) => r.status === 'Failed'));

	inProgress = $derived(this.removalAttempts.filter((r) => r.status === 'Processing'));

	allComplete = $derived(
		this.removalAttempts.length > 0 &&
			this.removalAttempts.every(
				(r) =>
					r.status === 'Submitted' ||
					r.status === 'Completed' ||
					r.status === 'Failed' ||
					(r.status === 'Pending' && r.error_message?.startsWith('CAPTCHA_REQUIRED'))
			)
	);

	/**
	 * Load removal attempts for a scan job
	 */
	async loadRemovalAttempts(vaultId: string, scanJobId: string) {
		this.loading = true;
		this.error = null;
		this.scanJobId = scanJobId;

		try {
			this.removalAttempts = await removalAPI.getByScanJob(vaultId, scanJobId);
		} catch (err) {
			this.error = 'Failed to load removal attempts';
			console.error('Load removal attempts error:', err);
		} finally {
			this.loading = false;
		}
	}

	/**
	 * Start batch processing
	 */
	async startBatchProcessing(vaultId: string, removalAttemptIds: string[]) {
		this.error = null;

		try {
			const result = await removalAPI.processBatch(vaultId, removalAttemptIds);
			this.batchJobId = result.job_id;
			return result;
		} catch (err) {
			this.error = 'Failed to start batch processing';
			console.error('Batch processing error:', err);
			throw err;
		}
	}

	/**
	 * Retry a failed removal attempt
	 */
	async retryRemoval(vaultId: string, attemptId: string) {
		this.error = null;

		try {
			await removalAPI.retry(vaultId, attemptId);
		} catch (err) {
			this.error = 'Failed to retry removal';
			console.error('Retry removal error:', err);
			throw err;
		}
	}

	/**
	 * Set up event listeners for real-time updates
	 */
	async setupEventListeners() {
		// Clean up existing listeners
		await this.cleanupEventListeners();

		// Listen for removal:started events
		const unlistenStarted = await listen('removal:started', (event: any) => {
			const { attempt_id } = event.payload;
			this.updateAttempt(attempt_id, { status: 'Processing' });
		});
		this.unlistenFns.push(unlistenStarted);

		// Listen for removal:success events
		const unlistenSuccess = await listen('removal:success', (event: any) => {
			const { attempt_id } = event.payload;
			this.updateAttempt(attempt_id, {
				status: 'Submitted',
				submitted_at: new Date().toISOString()
			});
		});
		this.unlistenFns.push(unlistenSuccess);

		// Listen for removal:captcha events
		const unlistenCaptcha = await listen('removal:captcha', (event: any) => {
			const { attempt_id, outcome } = event.payload;
			// Extract URL from outcome string
			const url = this.extractCaptchaUrl(outcome);
			this.updateAttempt(attempt_id, {
				status: 'Pending',
				error_message: `CAPTCHA_REQUIRED:${url}`
			});
		});
		this.unlistenFns.push(unlistenCaptcha);

		// Listen for removal:failed events
		const unlistenFailed = await listen('removal:failed', (event: any) => {
			const { attempt_id, error } = event.payload;
			this.updateAttempt(attempt_id, {
				status: 'Failed',
				error_message: error
			});
		});
		this.unlistenFns.push(unlistenFailed);

		// Listen for removal:retry events
		const unlistenRetry = await listen('removal:retry', (event: any) => {
			const { attempt_id } = event.payload;
			this.updateAttempt(attempt_id, {
				status: 'Processing',
				error_message: null
			});
		});
		this.unlistenFns.push(unlistenRetry);
	}

	/**
	 * Clean up event listeners
	 */
	async cleanupEventListeners() {
		for (const unlisten of this.unlistenFns) {
			await unlisten();
		}
		this.unlistenFns = [];
	}

	/**
	 * Update a specific removal attempt
	 */
	private updateAttempt(attemptId: string, updates: Partial<RemovalAttempt>) {
		const index = this.removalAttempts.findIndex((a) => a.id === attemptId);
		if (index !== -1) {
			this.removalAttempts[index] = { ...this.removalAttempts[index], ...updates };
		}
	}

	/**
	 * Extract CAPTCHA URL from outcome string
	 */
	private extractCaptchaUrl(outcome: string): string {
		// Outcome format: "RequiresCaptcha { captcha_url: 'https://...' }"
		const match = outcome.match(/captcha_url:\s*['"]([^'"]+)['"]/);
		return match ? match[1] : '';
	}

	/**
	 * Reset store state
	 */
	reset() {
		this.removalAttempts = [];
		this.scanJobId = null;
		this.batchJobId = null;
		this.loading = false;
		this.error = null;
	}
}

export const removalStore = new RemovalStore();
```

**Step 2: Export from index**

Modify `src/lib/stores/index.ts`, add:

```typescript
export { removalStore } from './removal.svelte';
```

**Step 3: Verify TypeScript compiles**

Run: `npm run check`
Expected: No TypeScript errors

**Step 4: Commit**

```bash
git add src/lib/stores/removal.svelte.ts src/lib/stores/index.ts
git commit -m "feat(frontend): add removal store with event listeners

Add Svelte 5 store for managing removal attempt state with:
- Real-time event listeners for batch progress updates
- Derived counts for queues and completion status
- Methods for loading, batch processing, and retry

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Create Overview Tab Component

**Files:**
- Create: `src/lib/components/removals/OverviewTab.svelte`

**Step 1: Create overview tab component**

Create `src/lib/components/removals/OverviewTab.svelte`:

```svelte
<script lang="ts">
	import type { RemovalAttempt } from '$lib/api/removal';

	interface Props {
		removalAttempts: RemovalAttempt[];
		allComplete: boolean;
	}

	let { removalAttempts, allComplete }: Props = $props();

	// Counts
	const total = $derived(removalAttempts.length);
	const submitted = $derived(
		removalAttempts.filter((r) => r.status === 'Submitted' || r.status === 'Completed').length
	);
	const inProgress = $derived(removalAttempts.filter((r) => r.status === 'Processing').length);
	const captcha = $derived(
		removalAttempts.filter(
			(r) => r.status === 'Pending' && r.error_message?.startsWith('CAPTCHA_REQUIRED')
		).length
	);
	const failed = $derived(removalAttempts.filter((r) => r.status === 'Failed').length);

	// Progress percentage
	const progressPercent = $derived(
		total > 0 ? Math.round(((submitted + captcha + failed) / total) * 100) : 0
	);

	// Recent activity (last 10 items)
	const recentActivity = $derived(
		[...removalAttempts]
			.sort((a, b) => {
				const timeA = a.submitted_at || a.created_at;
				const timeB = b.submitted_at || b.created_at;
				return new Date(timeB).getTime() - new Date(timeA).getTime();
			})
			.slice(0, 10)
	);

	function getStatusBadge(attempt: RemovalAttempt) {
		if (attempt.status === 'Submitted' || attempt.status === 'Completed') {
			return { text: 'Submitted', color: 'bg-green-100 text-green-800' };
		} else if (attempt.status === 'Processing') {
			return { text: 'Processing', color: 'bg-blue-100 text-blue-800' };
		} else if (attempt.error_message?.startsWith('CAPTCHA_REQUIRED')) {
			return { text: 'CAPTCHA', color: 'bg-yellow-100 text-yellow-800' };
		} else if (attempt.status === 'Failed') {
			return { text: 'Failed', color: 'bg-red-100 text-red-800' };
		} else {
			return { text: 'Pending', color: 'bg-gray-100 text-gray-800' };
		}
	}

	function formatTime(isoString: string) {
		const date = new Date(isoString);
		return date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
	}
</script>

<div class="space-y-6">
	<!-- Batch Statistics -->
	<div class="bg-white rounded-lg border border-gray-200 p-6">
		<h2 class="text-lg font-semibold text-gray-900 mb-4">Batch Statistics</h2>

		{#if allComplete}
			<div class="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg">
				<p class="text-sm font-medium text-green-900">✓ Batch Complete</p>
				<p class="text-xs text-green-700 mt-1">All removals processed</p>
			</div>
		{/if}

		<div class="grid grid-cols-2 md:grid-cols-5 gap-4">
			<div class="text-center">
				<div class="text-3xl font-bold text-gray-900">{total}</div>
				<div class="text-sm text-gray-600 mt-1">Total</div>
			</div>
			<div class="text-center">
				<div class="text-3xl font-bold text-green-600">{submitted}</div>
				<div class="text-sm text-gray-600 mt-1">Submitted</div>
			</div>
			<div class="text-center">
				<div class="text-3xl font-bold text-blue-600">{inProgress}</div>
				<div class="text-sm text-gray-600 mt-1">In Progress</div>
			</div>
			<div class="text-center">
				<div class="text-3xl font-bold text-yellow-600">{captcha}</div>
				<div class="text-sm text-gray-600 mt-1">CAPTCHA</div>
			</div>
			<div class="text-center">
				<div class="text-3xl font-bold text-red-600">{failed}</div>
				<div class="text-sm text-gray-600 mt-1">Failed</div>
			</div>
		</div>
	</div>

	<!-- Progress Bar -->
	<div class="bg-white rounded-lg border border-gray-200 p-6">
		<div class="flex items-center justify-between mb-2">
			<h2 class="text-lg font-semibold text-gray-900">Progress</h2>
			<span class="text-sm font-medium text-gray-700">{progressPercent}%</span>
		</div>
		<div class="w-full bg-gray-200 rounded-full h-4">
			<div
				class="bg-blue-600 h-4 rounded-full transition-all duration-300"
				style="width: {progressPercent}%"
			></div>
		</div>
	</div>

	<!-- Recent Activity -->
	<div class="bg-white rounded-lg border border-gray-200 p-6">
		<h2 class="text-lg font-semibold text-gray-900 mb-4">Recent Activity</h2>

		{#if recentActivity.length === 0}
			<p class="text-sm text-gray-500">No activity yet</p>
		{:else}
			<div class="space-y-3">
				{#each recentActivity as attempt}
					{@const badge = getStatusBadge(attempt)}
					<div class="flex items-center justify-between py-2 border-b border-gray-100 last:border-0">
						<div class="flex-1">
							<div class="text-sm font-medium text-gray-900">{attempt.broker_id}</div>
							<div class="text-xs text-gray-500 mt-1">
								{formatTime(attempt.submitted_at || attempt.created_at)}
							</div>
						</div>
						<span class="px-3 py-1 rounded-full text-xs font-medium {badge.color}">
							{badge.text}
						</span>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>
```

**Step 2: Verify component compiles**

Run: `npm run check`
Expected: No TypeScript errors

**Step 3: Commit**

```bash
git add src/lib/components/removals/OverviewTab.svelte
git commit -m "feat(frontend): add overview tab component

Display batch statistics, progress bar, and recent activity feed
for removal submission monitoring.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Create CAPTCHA Queue Tab Component

**Files:**
- Create: `src/lib/components/removals/CaptchaQueueTab.svelte`

**Step 1: Create CAPTCHA queue tab component**

Create `src/lib/components/removals/CaptchaQueueTab.svelte`:

```svelte
<script lang="ts">
	import type { RemovalAttempt } from '$lib/api/removal';
	import { open } from '@tauri-apps/plugin-shell';

	interface Props {
		captchaQueue: RemovalAttempt[];
	}

	let { captchaQueue }: Props = $props();

	function extractCaptchaUrl(errorMessage: string): string {
		// Format: "CAPTCHA_REQUIRED:https://..."
		const parts = errorMessage.split('CAPTCHA_REQUIRED:');
		return parts.length > 1 ? parts[1] : '';
	}

	async function openInBrowser(url: string) {
		try {
			await open(url);
		} catch (err) {
			console.error('Failed to open browser:', err);
			alert('Failed to open URL in browser');
		}
	}

	function formatTime(isoString: string) {
		const date = new Date(isoString);
		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffMins = Math.floor(diffMs / 60000);

		if (diffMins < 1) return 'Just now';
		if (diffMins < 60) return `${diffMins}m ago`;
		const diffHours = Math.floor(diffMins / 60);
		if (diffHours < 24) return `${diffHours}h ago`;
		const diffDays = Math.floor(diffHours / 24);
		return `${diffDays}d ago`;
	}
</script>

<div class="space-y-4">
	{#if captchaQueue.length === 0}
		<!-- Empty State -->
		<div class="bg-white rounded-lg border border-gray-200 p-12 text-center">
			<div class="inline-flex items-center justify-center w-16 h-16 bg-green-100 rounded-full mb-4">
				<span class="text-3xl text-green-600">✓</span>
			</div>
			<h3 class="text-lg font-semibold text-gray-900 mb-2">No CAPTCHAs to solve</h3>
			<p class="text-sm text-gray-600">All removals processed without CAPTCHA blocks</p>
		</div>
	{:else}
		<!-- CAPTCHA Queue List -->
		<div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
			<div class="px-6 py-4 border-b border-gray-200 bg-gray-50">
				<h2 class="text-lg font-semibold text-gray-900">
					CAPTCHA Queue ({captchaQueue.length})
				</h2>
				<p class="text-sm text-gray-600 mt-1">
					These removals require CAPTCHA solving to continue
				</p>
			</div>

			<div class="divide-y divide-gray-200">
				{#each captchaQueue as attempt}
					{@const captchaUrl = extractCaptchaUrl(attempt.error_message || '')}
					<div class="p-6 hover:bg-gray-50">
						<div class="flex items-start justify-between">
							<div class="flex-1">
								<div class="flex items-center gap-2 mb-2">
									<span class="text-sm font-semibold text-gray-900">{attempt.broker_id}</span>
									<span class="px-2 py-1 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800">
										CAPTCHA Required
									</span>
								</div>

								<div class="text-sm text-gray-600 mb-2">
									<span class="font-medium">Listing URL:</span>
									<span class="ml-2 break-all">{captchaUrl || 'Unknown'}</span>
								</div>

								<div class="text-xs text-gray-500">
									Blocked {formatTime(attempt.created_at)}
								</div>
							</div>

							<button
								onclick={() => openInBrowser(captchaUrl)}
								disabled={!captchaUrl}
								class="ml-4 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm font-medium"
							>
								Open in Browser
							</button>
						</div>

						<div class="mt-3 p-3 bg-blue-50 rounded-lg">
							<p class="text-xs text-blue-900">
								<strong>Instructions:</strong> Click "Open in Browser" to solve the CAPTCHA manually.
								After solving, the removal may be retried from the Failed queue if needed.
							</p>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
```

**Step 2: Verify component compiles**

Run: `npm run check`
Expected: No TypeScript errors

**Step 3: Commit**

```bash
git add src/lib/components/removals/CaptchaQueueTab.svelte
git commit -m "feat(frontend): add CAPTCHA queue tab component

Display CAPTCHA-blocked removal attempts with "Open in Browser"
action for manual CAPTCHA solving.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 6: Create Failed Queue Tab Component

**Files:**
- Create: `src/lib/components/removals/FailedQueueTab.svelte`

**Step 1: Create failed queue tab component**

Create `src/lib/components/removals/FailedQueueTab.svelte`:

```svelte
<script lang="ts">
	import type { RemovalAttempt } from '$lib/api/removal';

	interface Props {
		failedQueue: RemovalAttempt[];
		onRetry: (attemptId: string) => void;
		onRetryAll: () => void;
	}

	let { failedQueue, onRetry, onRetryAll }: Props = $props();

	let expandedErrors = $state<Set<string>>(new Set());
	let retrying = $state<Set<string>>(new Set());

	function toggleError(attemptId: string) {
		const newSet = new Set(expandedErrors);
		if (newSet.has(attemptId)) {
			newSet.delete(attemptId);
		} else {
			newSet.add(attemptId);
		}
		expandedErrors = newSet;
	}

	async function handleRetry(attemptId: string) {
		retrying.add(attemptId);
		try {
			await onRetry(attemptId);
		} finally {
			const newSet = new Set(retrying);
			newSet.delete(attemptId);
			retrying = newSet;
		}
	}

	function formatTime(isoString: string) {
		const date = new Date(isoString);
		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffMins = Math.floor(diffMs / 60000);

		if (diffMins < 1) return 'Just now';
		if (diffMins < 60) return `${diffMins}m ago`;
		const diffHours = Math.floor(diffMins / 60);
		if (diffHours < 24) return `${diffHours}h ago`;
		const diffDays = Math.floor(diffHours / 24);
		return `${diffDays}d ago`;
	}
</script>

<div class="space-y-4">
	{#if failedQueue.length === 0}
		<!-- Empty State -->
		<div class="bg-white rounded-lg border border-gray-200 p-12 text-center">
			<div class="inline-flex items-center justify-center w-16 h-16 bg-green-100 rounded-full mb-4">
				<span class="text-3xl text-green-600">✓</span>
			</div>
			<h3 class="text-lg font-semibold text-gray-900 mb-2">No failed attempts</h3>
			<p class="text-sm text-gray-600">
				All removals succeeded or need manual attention (CAPTCHA)
			</p>
		</div>
	{:else}
		<!-- Failed Queue List -->
		<div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
			<div class="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center justify-between">
				<div>
					<h2 class="text-lg font-semibold text-gray-900">Failed Queue ({failedQueue.length})</h2>
					<p class="text-sm text-gray-600 mt-1">These removals failed after all retry attempts</p>
				</div>
				{#if failedQueue.length > 1}
					<button
						onclick={onRetryAll}
						class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm font-medium"
					>
						Retry All
					</button>
				{/if}
			</div>

			<div class="divide-y divide-gray-200">
				{#each failedQueue as attempt}
					{@const isExpanded = expandedErrors.has(attempt.id)}
					{@const isRetrying = retrying.has(attempt.id)}
					<div class="p-6 hover:bg-gray-50">
						<div class="flex items-start justify-between">
							<div class="flex-1">
								<div class="flex items-center gap-2 mb-2">
									<span class="text-sm font-semibold text-gray-900">{attempt.broker_id}</span>
									<span class="px-2 py-1 rounded-full text-xs font-medium bg-red-100 text-red-800">
										Failed
									</span>
								</div>

								<div class="text-sm text-gray-600 mb-2">
									<button
										onclick={() => toggleError(attempt.id)}
										class="text-blue-600 hover:text-blue-800 font-medium"
									>
										{isExpanded ? 'Hide' : 'Show'} Error Details
									</button>
								</div>

								{#if isExpanded && attempt.error_message}
									<div class="mt-2 p-3 bg-red-50 border border-red-200 rounded-lg">
										<p class="text-sm text-red-900 font-mono break-all">
											{attempt.error_message}
										</p>
									</div>
								{/if}

								<div class="text-xs text-gray-500 mt-2">Failed {formatTime(attempt.created_at)}</div>
							</div>

							<button
								onclick={() => handleRetry(attempt.id)}
								disabled={isRetrying}
								class="ml-4 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm font-medium flex items-center gap-2"
							>
								{#if isRetrying}
									<span class="inline-block w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"></span>
									Retrying...
								{:else}
									Retry
								{/if}
							</button>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
```

**Step 2: Verify component compiles**

Run: `npm run check`
Expected: No TypeScript errors

**Step 3: Commit**

```bash
git add src/lib/components/removals/FailedQueueTab.svelte
git commit -m "feat(frontend): add failed queue tab component

Display failed removal attempts with expandable error messages
and individual/batch retry functionality.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 7: Create Progress Dashboard Page

**Files:**
- Create: `src/routes/removals/progress/[jobId]/+page.svelte`

**Step 1: Create progress page with tab navigation**

Create `src/routes/removals/progress/[jobId]/+page.svelte`:

```svelte
<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { vaultStore, removalStore } from '$lib/stores';
	import OverviewTab from '$lib/components/removals/OverviewTab.svelte';
	import CaptchaQueueTab from '$lib/components/removals/CaptchaQueueTab.svelte';
	import FailedQueueTab from '$lib/components/removals/FailedQueueTab.svelte';

	const scanJobId = $derived($page.params.jobId);

	let activeTab = $state<'overview' | 'captcha' | 'failed'>('overview');

	onMount(async () => {
		// Validate scan job ID
		if (!scanJobId) {
			goto('/');
			return;
		}

		// Validate vault is unlocked
		if (!vaultStore.currentVaultId) {
			goto('/');
			return;
		}

		// Load removal attempts
		await removalStore.loadRemovalAttempts(vaultStore.currentVaultId, scanJobId);

		// Set up event listeners
		await removalStore.setupEventListeners();
	});

	onDestroy(async () => {
		// Clean up event listeners
		await removalStore.cleanupEventListeners();
	});

	async function handleRetry(attemptId: string) {
		if (!vaultStore.currentVaultId) return;

		try {
			await removalStore.retryRemoval(vaultStore.currentVaultId, attemptId);
		} catch (err) {
			console.error('Retry failed:', err);
		}
	}

	async function handleRetryAll() {
		if (!vaultStore.currentVaultId) return;

		for (const attempt of removalStore.failedQueue) {
			try {
				await removalStore.retryRemoval(vaultStore.currentVaultId, attempt.id);
			} catch (err) {
				console.error('Retry failed for', attempt.id, err);
			}
		}
	}
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 p-4">
	<div class="max-w-6xl mx-auto">
		<div class="bg-white rounded-lg shadow-xl overflow-hidden">
			<!-- Header -->
			<div class="px-8 py-6 border-b border-gray-200">
				<h1 class="text-3xl font-bold text-gray-900">Removal Progress</h1>
				<p class="text-gray-600 mt-2">
					Monitoring {removalStore.removalAttempts.length} removal request{removalStore.removalAttempts.length !== 1 ? 's' : ''}
				</p>
			</div>

			<!-- Tab Navigation -->
			<div class="border-b border-gray-200 px-8">
				<nav class="flex gap-8">
					<button
						onclick={() => (activeTab = 'overview')}
						class="py-4 border-b-2 font-medium text-sm transition-colors {activeTab === 'overview'
							? 'border-blue-600 text-blue-600'
							: 'border-transparent text-gray-600 hover:text-gray-900 hover:border-gray-300'}"
					>
						Overview
					</button>
					<button
						onclick={() => (activeTab = 'captcha')}
						class="py-4 border-b-2 font-medium text-sm transition-colors flex items-center gap-2 {activeTab === 'captcha'
							? 'border-blue-600 text-blue-600'
							: 'border-transparent text-gray-600 hover:text-gray-900 hover:border-gray-300'}"
					>
						CAPTCHA Queue
						{#if removalStore.captchaQueue.length > 0}
							<span class="px-2 py-0.5 bg-yellow-100 text-yellow-800 rounded-full text-xs font-semibold">
								{removalStore.captchaQueue.length}
							</span>
						{/if}
					</button>
					<button
						onclick={() => (activeTab = 'failed')}
						class="py-4 border-b-2 font-medium text-sm transition-colors flex items-center gap-2 {activeTab === 'failed'
							? 'border-blue-600 text-blue-600'
							: 'border-transparent text-gray-600 hover:text-gray-900 hover:border-gray-300'}"
					>
						Failed Queue
						{#if removalStore.failedQueue.length > 0}
							<span class="px-2 py-0.5 bg-red-100 text-red-800 rounded-full text-xs font-semibold">
								{removalStore.failedQueue.length}
							</span>
						{/if}
					</button>
				</nav>
			</div>

			<!-- Tab Content -->
			<div class="p-8">
				{#if removalStore.loading}
					<div class="text-center py-12">
						<div class="inline-block w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
						<p class="text-gray-600 mt-4">Loading removal attempts...</p>
					</div>
				{:else if removalStore.error}
					<div class="bg-red-50 border border-red-200 rounded-lg p-6">
						<p class="text-red-900 font-medium">{removalStore.error}</p>
						<button
							onclick={() => {
								if (vaultStore.currentVaultId && scanJobId) {
									removalStore.loadRemovalAttempts(vaultStore.currentVaultId, scanJobId);
								}
							}}
							class="mt-4 px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors"
						>
							Retry
						</button>
					</div>
				{:else if activeTab === 'overview'}
					<OverviewTab
						removalAttempts={removalStore.removalAttempts}
						allComplete={removalStore.allComplete}
					/>
				{:else if activeTab === 'captcha'}
					<CaptchaQueueTab captchaQueue={removalStore.captchaQueue} />
				{:else if activeTab === 'failed'}
					<FailedQueueTab
						failedQueue={removalStore.failedQueue}
						onRetry={handleRetry}
						onRetryAll={handleRetryAll}
					/>
				{/if}
			</div>

			<!-- Footer -->
			<div class="px-8 py-6 border-t border-gray-200 bg-gray-50">
				<button
					onclick={() => goto('/')}
					class="px-6 py-3 bg-gray-600 text-white rounded-lg hover:bg-gray-700 transition-colors font-medium"
				>
					Return to Dashboard
				</button>
			</div>
		</div>
	</div>
</div>
```

**Step 2: Verify page compiles**

Run: `npm run check`
Expected: No TypeScript errors

**Step 3: Commit**

```bash
git add src/routes/removals/progress/[jobId]/+page.svelte
git commit -m "feat(frontend): add progress dashboard page

Main removal progress page with tab navigation for overview,
CAPTCHA queue, and failed queue. Includes real-time event
listeners and state management.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 8: Update Scan Review Flow Integration

**Files:**
- Modify: `src/routes/scan/review/[id]/+page.svelte`

**Step 1: Update handleSubmit to navigate to progress page**

Modify `src/routes/scan/review/[id]/+page.svelte`, find the `handleSubmit` function (around line 58-71) and replace it:

```typescript
async function handleSubmit() {
	if (confirmedCount === 0 || !scanJobId || !vaultStore.currentVaultId) {
		return;
	}

	actionError = null;
	try {
		// Step 1: Create removal attempts (existing command)
		const removalAttemptIds = await scanStore.submitRemovals(vaultStore.currentVaultId, scanJobId);

		// Step 2: Start batch processing
		const result = await removalAPI.processBatch(vaultStore.currentVaultId, removalAttemptIds);

		// Step 3: Navigate to progress page
		goto(`/removals/progress/${scanJobId}`);
	} catch (err) {
		actionError = 'Failed to submit removals. Please try again.';
		console.error('Submission failed:', err);
	}
}
```

**Step 2: Add import**

Add at top of script section:

```typescript
import { removalAPI } from '$lib/api/removal';
```

**Step 3: Verify page compiles**

Run: `npm run check`
Expected: No TypeScript errors

**Step 4: Test navigation flow**

Run: `npm run dev`
- Navigate to scan review page
- Click "Submit Removals"
- Should navigate to `/removals/progress/[jobId]`

**Step 5: Commit**

```bash
git add src/routes/scan/review/[id]/+page.svelte
git commit -m "feat(frontend): integrate progress dashboard with scan review

Update scan review submit flow to start batch processing and
navigate to progress dashboard instead of stub page.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 9: Update Removals Stub Page (Optional)

**Files:**
- Modify: `src/routes/removals/+page.svelte`

**Step 1: Convert stub to redirect or history page**

For MVP, simply redirect to dashboard. Replace content of `src/routes/removals/+page.svelte`:

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	onMount(() => {
		// For MVP, redirect to dashboard
		// In future, could show removal history/job list
		goto('/');
	});
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 flex items-center justify-center">
	<div class="text-center">
		<div class="inline-block w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
		<p class="text-gray-600 mt-4">Redirecting...</p>
	</div>
</div>
```

**Step 2: Commit**

```bash
git add src/routes/removals/+page.svelte
git commit -m "feat(frontend): convert removals stub to redirect

Replace stub page with redirect to dashboard. Future enhancement
could show removal job history.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 10: Manual End-to-End Testing

**Files:**
- N/A (testing only)

**Step 1: Build and run application**

Run: `npm run tauri dev`

**Step 2: Complete scan flow**

1. Create/unlock vault
2. Create profile
3. Start scan (use test broker if available)
4. Review findings, confirm some
5. Click "Submit Removals"

**Step 3: Verify progress dashboard**

Expected behavior:
- ✓ Navigates to `/removals/progress/[jobId]`
- ✓ Shows loading state initially
- ✓ Loads removal attempts
- ✓ Sets up event listeners
- ✓ Shows overview tab with counts
- ✓ Progress bar displays correctly

**Step 4: Test tab navigation**

- ✓ Click CAPTCHA Queue tab - should show empty or populated queue
- ✓ Click Failed Queue tab - should show empty or populated queue
- ✓ Click Overview tab - should return to overview
- ✓ Tab counts update in navigation badges

**Step 5: Test CAPTCHA flow**

If any CAPTCHAs exist:
- ✓ CAPTCHA Queue tab shows attempts
- ✓ "Open in Browser" button works
- ✓ Browser opens with correct URL

**Step 6: Test retry flow**

If any failures exist:
- ✓ Failed Queue tab shows attempts
- ✓ "Show Error Details" expands error message
- ✓ "Retry" button triggers retry command
- ✓ "Retry All" button retries all failures
- ✓ Retry status updates in real-time

**Step 7: Test real-time updates**

Simulate or trigger events:
- ✓ removal:started - Status updates to "Processing"
- ✓ removal:success - Status updates to "Submitted"
- ✓ removal:captcha - Moves to CAPTCHA queue
- ✓ removal:failed - Moves to Failed queue
- ✓ Counts update automatically
- ✓ Progress bar updates

**Step 8: Test error handling**

- Navigate away from page, return
  - ✓ State reloads from database
  - ✓ Event listeners re-setup

- Disconnect vault, try action
  - ✓ Shows appropriate error message

**Step 9: Test completion state**

When all removals complete:
- ✓ "Batch Complete" badge shows on overview tab
- ✓ All counts reflect final state
- ✓ Progress bar shows 100%

**Step 10: Document any issues**

Create GitHub issues for:
- Bugs found during testing
- UX improvements needed
- Performance issues
- Edge cases not handled

**No commit for this task** - testing only

---

## Next Steps (Phase 6 - Not in This Plan)

The following features are deferred to Phase 6:

1. **Guided CAPTCHA Solving** - In-app browser with automation hints
2. **Email Verification Monitoring** - Auto-detect and verify confirmation emails
3. **Job History Page** - View past removal batches at `/removals`
4. **Batch Scheduling** - Schedule removals for specific times
5. **Advanced Analytics** - Success rates, broker performance tracking

**Phase 5 Frontend: Complete** ✅

---

## Success Criteria Checklist

- ✅ Real-time progress dashboard with event-driven updates
- ✅ Tab navigation (Overview, CAPTCHA Queue, Failed Queue)
- ✅ Batch statistics display with progress bar
- ✅ CAPTCHA queue with "Open in Browser" functionality
- ✅ Failed queue with retry functionality
- ✅ Integration with scan review flow
- ✅ Error handling and loading states
- ✅ Empty states for queues
- ✅ State persistence (reload from database on mount)
- ✅ Clean event listener management (setup/cleanup)
