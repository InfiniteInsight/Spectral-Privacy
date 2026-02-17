# Phase 5 Frontend: Removal Submission UI - Design Document

**Date:** 2026-02-16
**Status:** Approved
**Dependencies:** Phase 5 Backend (Complete)

## Overview

Phase 5 Frontend implements the user interface for automated data broker removal form submission. Building on the Phase 5 backend (batch processing, queue management, real-time events), this phase adds the UI components that allow users to monitor removal progress, handle CAPTCHAs, and retry failed submissions.

## Goals

1. **Real-time progress tracking** - Show live updates as removals process
2. **Queue management UI** - View and handle CAPTCHA and failed queues
3. **Manual CAPTCHA solving** - Open broker URLs in browser for user to solve
4. **Retry functionality** - Allow users to retry failed submissions
5. **Seamless integration** - Connect existing scan review flow to new progress dashboard

## User Workflow

```
1. User reviews findings (existing scan/review page)
   ↓
2. Clicks "Submit Removals" → Calls submit_removals_for_confirmed
   ↓
3. Navigates to Progress Dashboard (/removals/progress/[jobId])
   ↓
4. Backend calls process_removal_batch → Workers start processing
   ↓
5. Real-time updates via Tauri events:
   - Overview tab: Shows progress, counts, activity feed
   - CAPTCHA Queue tab: Lists CAPTCHA-blocked attempts
   - Failed Queue tab: Lists failed attempts with errors
   ↓
6. User actions:
   - Open CAPTCHA URLs in browser (manual solving)
   - Retry failed attempts
   - Monitor until batch completes
   ↓
7. When complete: "Return to Dashboard" or review queues
```

## Architecture

### Pattern: Single-Page Tabbed Component

**Approach:** One route with local tab state, three tab panels sharing event listeners and state.

**Rationale:** Simplest approach that provides instant tab switching, shared state across tabs, and matches existing scan progress pattern.

### File Structure

```
src/
├── routes/
│   ├── scan/review/[id]/+page.svelte          (MODIFY - update submit flow)
│   └── removals/
│       ├── +page.svelte                        (MODIFY - remove stub, redirect)
│       └── progress/[jobId]/
│           └── +page.svelte                    (NEW - main progress page)
├── lib/
│   ├── api/
│   │   └── removal.ts                          (NEW - removal command wrappers)
│   ├── stores/
│   │   └── removal.svelte.ts                   (NEW - removal state + events)
│   └── components/
│       └── removals/
│           ├── OverviewTab.svelte              (NEW - overview panel)
│           ├── CaptchaQueueTab.svelte          (NEW - CAPTCHA queue panel)
│           └── FailedQueueTab.svelte           (NEW - failed queue panel)
```

## Components

### 1. Progress Dashboard Page (`/removals/progress/[jobId]/+page.svelte`)

**Responsibilities:**
- Parse jobId from URL params
- Load initial removal attempts from backend
- Set up Tauri event listeners
- Manage tab state (overview/captcha/failed)
- Orchestrate child tab components
- Handle user actions (retry, open browser)

**State:**
```typescript
let activeTab = $state<'overview' | 'captcha' | 'failed'>('overview')
```

**On Mount:**
1. Validate jobId and vaultId
2. Query backend for removal attempts
3. Set up event listeners (removal:started, success, captcha, failed, retry)
4. Start listening for real-time updates

**Layout:**
```
┌─────────────────────────────────────────────┐
│ Progress Dashboard                          │
│ ┌─────────────────────────────────────────┐ │
│ │ [Overview] [CAPTCHA (3)] [Failed (2)]   │ │ ← Tab Navigation
│ └─────────────────────────────────────────┘ │
│                                             │
│ [Active Tab Panel Component]                │
│                                             │
│ [Return to Dashboard]                       │
└─────────────────────────────────────────────┘
```

### 2. Overview Tab Component (`OverviewTab.svelte`)

**Props:**
- `removalAttempts: RemovalAttempt[]`
- `jobId: string`

**Displays:**
- **Batch Statistics Card**
  - Total removals: X
  - Submitted: Y (✓)
  - In Progress: Z (⏳)
  - Need CAPTCHA: N (🧩)
  - Failed: M (❌)

- **Progress Bar**
  - Visual completion indicator
  - Shows percentage of processed vs total

- **Recent Activity Feed**
  - Last 10 events (started, success, captcha, failed)
  - Timestamp, broker name, status
  - Auto-scrolls as events arrive

**Completion Indicator:**
When all attempts are in final states (Submitted, Completed, or in queues):
- Show "Batch Complete" badge
- Highlight next actions (check queues, retry failures)

### 3. CAPTCHA Queue Tab Component (`CaptchaQueueTab.svelte`)

**Props:**
- `captchaQueue: RemovalAttempt[]` (filtered: error_message starts with "CAPTCHA_REQUIRED")

**Displays:**
- List of CAPTCHA-blocked attempts
- Each item shows:
  - Broker name
  - Listing URL (truncated)
  - "Open in Browser" button
  - Time blocked

**Actions:**
- **Open in Browser:** Extract URL from `error_message` (format: `CAPTCHA_REQUIRED:<url>`), open in external browser
- After solving externally, user can retry from failed queue (attempt will likely fail again, but allows them to try)

**Empty State:**
```
┌─────────────────────────────────┐
│     No CAPTCHAs to solve        │
│                                 │
│  ✓ All removals processed       │
│    without CAPTCHA blocks       │
└─────────────────────────────────┘
```

### 4. Failed Queue Tab Component (`FailedQueueTab.svelte`)

**Props:**
- `failedQueue: RemovalAttempt[]` (filtered: status === 'Failed')

**Displays:**
- List of failed attempts
- Each item shows:
  - Broker name
  - Listing URL
  - Error message (expandable if long)
  - Retry count
  - "Retry" button

**Actions:**
- **Retry:** Call `retry_removal(vaultId, attemptId)` → Backend resets to Pending and spawns worker
- **Retry All:** Batch retry all failed attempts

**Empty State:**
```
┌─────────────────────────────────┐
│      No failed attempts         │
│                                 │
│  ✓ All removals succeeded       │
│    or need manual attention     │
└─────────────────────────────────┘
```

## Data Flow

### State Management

**Removal Store (`removal.svelte.ts`):**

```typescript
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

class RemovalStore {
  // State
  removalAttempts = $state<RemovalAttempt[]>([])
  jobId = $state<string | null>(null)
  loading = $state(false)
  error = $state<string | null>(null)

  // Derived counts
  submitted = $derived(this.removalAttempts.filter(r =>
    r.status === 'Submitted' || r.status === 'Completed'
  ))

  captchaQueue = $derived(this.removalAttempts.filter(r =>
    r.status === 'Pending' && r.error_message?.startsWith('CAPTCHA_REQUIRED')
  ))

  failedQueue = $derived(this.removalAttempts.filter(r =>
    r.status === 'Failed'
  ))

  inProgress = $derived(this.removalAttempts.filter(r =>
    r.status === 'Processing'
  ))

  // Methods
  async loadRemovalAttempts(vaultId: string, jobId: string) { ... }
  async retryRemoval(vaultId: string, attemptId: string) { ... }
  handleEvent(event) { ... }
}

export const removalStore = new RemovalStore();
```

### Event Handling

**Event Listeners (set up on page mount):**

```typescript
// removal:started - Worker task begins
listen('removal:started', (event) => {
  const { attempt_id } = event.payload;
  removalStore.updateAttempt(attempt_id, { status: 'Processing' });
});

// removal:success - Form submitted successfully
listen('removal:success', (event) => {
  const { attempt_id, outcome } = event.payload;
  removalStore.updateAttempt(attempt_id, {
    status: 'Submitted',
    submitted_at: new Date().toISOString()
  });
});

// removal:captcha - CAPTCHA detected, routed to queue
listen('removal:captcha', (event) => {
  const { attempt_id, outcome } = event.payload;
  // Extract URL from outcome: "RequiresCaptcha { captcha_url: '...' }"
  const url = extractCaptchaUrl(outcome);
  removalStore.updateAttempt(attempt_id, {
    status: 'Pending',
    error_message: `CAPTCHA_REQUIRED:${url}`
  });
});

// removal:failed - Failed after all retries
listen('removal:failed', (event) => {
  const { attempt_id, error } = event.payload;
  removalStore.updateAttempt(attempt_id, {
    status: 'Failed',
    error_message: error
  });
});

// removal:retry - Retry attempt starting
listen('removal:retry', (event) => {
  const { attempt_id } = event.payload;
  removalStore.updateAttempt(attempt_id, {
    status: 'Processing',
    error_message: null
  });
});
```

### Loading Strategy

**Initial Load (on mount):**
1. Parse `jobId` from URL params
2. Get `vaultId` from vault store
3. Query backend for removal attempts for this job (need new backend query command)
4. Set up event listeners
5. If batch hasn't started yet, call `process_removal_batch`

**Real-time Updates:**
- Events update store state
- Svelte reactivity updates all tab components automatically
- No polling needed (events provide instant updates)

**Refresh/Return:**
- If user navigates away and returns, re-query backend for current state
- Events may have been missed, database is source of truth

### Integration with Scan Review Flow

**Modify `/scan/review/[id]/+page.svelte`:**

Current flow:
```typescript
async function handleSubmit() {
  const count = await scanStore.submitRemovals(vaultId, scanJobId);
  goto(`/removals?count=${count}`);  // ← Goes to stub page
}
```

New flow:
```typescript
async function handleSubmit() {
  // Step 1: Create removal attempts (existing)
  const removalAttemptIds = await scanStore.submitRemovals(vaultId, scanJobId);

  // Step 2: Start batch processing
  const result = await invoke('process_removal_batch', {
    vaultId,
    removalAttemptIds
  });

  // Step 3: Navigate to progress page
  goto(`/removals/progress/${result.job_id}`);
}
```

## API Layer

### Removal Commands (`src/lib/api/removal.ts`)

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
   * Process a batch of removal attempts
   */
  async processBatch(
    vaultId: string,
    removalAttemptIds: string[]
  ): Promise<BatchSubmissionResult> {
    return await invoke('process_removal_batch', {
      vaultId,
      removalAttemptIds
    });
  },

  /**
   * Get CAPTCHA queue
   */
  async getCaptchaQueue(vaultId: string): Promise<RemovalAttempt[]> {
    return await invoke('get_captcha_queue', { vaultId });
  },

  /**
   * Get failed queue
   */
  async getFailedQueue(vaultId: string): Promise<RemovalAttempt[]> {
    return await invoke('get_failed_queue', { vaultId });
  },

  /**
   * Retry a failed removal
   */
  async retry(vaultId: string, removalAttemptId: string): Promise<void> {
    return await invoke('retry_removal', { vaultId, removalAttemptId });
  },

  /**
   * Get removal attempts by job ID (need backend command)
   */
  async getByJobId(vaultId: string, jobId: string): Promise<RemovalAttempt[]> {
    return await invoke('get_removal_attempts_by_job', { vaultId, jobId });
  }
};
```

**Note:** `get_removal_attempts_by_job` is a new backend command needed for initial load.

## Error Handling

### Error Scenarios

| Scenario | Handling |
|----------|----------|
| Invalid jobId in URL | Redirect to dashboard with toast "Invalid removal job" |
| Initial load fails | Show error banner "Unable to load removal status", "Retry" button |
| retry_removal fails | Show inline error on retry button "Retry failed, try again" |
| Event listener disconnects | Log error, continue (database query on refresh recovers state) |
| CAPTCHA URL extraction fails | Show "Invalid CAPTCHA URL" in queue item, disable "Open" button |
| Vault not unlocked | Redirect to unlock screen |

### User Feedback

**Loading States:**
- Skeleton loaders while initial data loads
- Spinner on retry button during retry
- Tab badges show "..." while counts loading

**Action Feedback:**
- Toast notifications for success/errors
- Inline error messages on retry failures
- Disabled buttons during processing

**Empty States:**
- "No CAPTCHAs to solve" when CAPTCHA queue empty
- "No failed attempts" when failed queue empty
- "Processing..." when no attempts loaded yet

## Testing Strategy

### Component Tests

**OverviewTab:**
- Renders correct counts from props
- Progress bar calculates percentage correctly
- Activity feed displays recent events
- "Batch Complete" shows when all attempts finished

**CaptchaQueueTab:**
- Displays queue items correctly
- "Open in Browser" extracts URL correctly
- Opens external browser on click
- Empty state shows when queue empty

**FailedQueueTab:**
- Displays error messages
- Retry button calls correct command
- Retry All button processes all items
- Empty state shows when queue empty

### Integration Tests

**Full Flow:**
1. Navigate to progress page with valid jobId
2. Verify initial state loads from backend
3. Simulate Tauri events, verify UI updates
4. Click retry, verify command called and UI updates
5. Switch tabs, verify data persists

### Manual Testing Scenarios

1. **All succeed:** No CAPTCHA or failures
2. **Mixed outcomes:** Some succeed, some CAPTCHA, some fail
3. **All CAPTCHA:** Every attempt hits CAPTCHA
4. **All fail:** Network errors, form changes, etc.
5. **Navigation:** Leave page, return, verify state restored
6. **Concurrent batches:** Multiple scan jobs processing simultaneously

## Success Criteria

1. ✅ User sees real-time progress updates during batch processing
2. ✅ CAPTCHA queue displays blocked attempts with "Open in Browser" action
3. ✅ Failed queue displays errors with "Retry" action
4. ✅ Tab navigation is instant (no page loads)
5. ✅ State persists when user navigates away and returns
6. ✅ Empty states display when queues are empty
7. ✅ Error handling provides clear user feedback
8. ✅ Integration with scan review flow is seamless

## Dependencies

**Phase 5 Backend (Complete):**
- `process_removal_batch` command
- `get_captcha_queue` command
- `get_failed_queue` command
- `retry_removal` command
- Tauri events: removal:started, success, captcha, failed, retry

**New Backend Requirement:**
- `get_removal_attempts_by_job` command (query removal attempts by job_id)

**Existing Frontend:**
- SvelteKit routing
- Svelte 5 runes ($state, $derived)
- Tailwind CSS
- Tauri invoke/listen APIs
- Vault store (for vaultId)
- Scan store (for submitRemovals)

## Out of Scope (Phase 6+)

- Guided CAPTCHA solving (in-app browser with automation)
- Email verification monitoring UI
- Batch scheduling interface
- Multi-broker session optimization UI
- Historical removal tracking (view past jobs)

## Implementation Notes

1. **Job ID Source:** Backend's `process_removal_batch` returns `job_id`, use this for routing
2. **CAPTCHA URL Extraction:** Parse from `error_message` format: `"CAPTCHA_REQUIRED:<url>"`
3. **Event Cleanup:** Unlisten from events on component unmount to prevent memory leaks
4. **Tab State Persistence:** Could store active tab in URL hash (#overview, #captcha, #failed) for deep linking
5. **Browser Opening:** Use Tauri's `shell::open` API to open external browser

## Migration Plan

Since `/removals/+page.svelte` currently exists as a stub:

1. Delete existing stub content
2. Add redirect logic: `goto('/removals/progress/' + lastJobId)` or `goto('/')` if no job
3. Alternatively, turn it into a "Removal History" page listing all past jobs (Phase 6 feature)
