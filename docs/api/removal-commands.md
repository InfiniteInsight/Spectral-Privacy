# Removal Submission API

This document describes the Tauri commands and events for the removal submission system (Phase 5 backend).

## Overview

The removal submission API provides:
- Batch processing of removal attempts with concurrent execution
- Queue management for CAPTCHAs and failed attempts
- Retry logic with exponential backoff
- Real-time progress events for UI updates

**Status:** Phase 5 Backend Complete (Frontend integration pending)

## Commands

### `process_removal_batch`

Process a batch of removal attempts concurrently (max 3 at a time).

**Signature:**
```rust
#[tauri::command]
pub async fn process_removal_batch(
    state: State<'_, AppState>,
    vault_id: String,
    removal_attempt_ids: Vec<String>,
) -> Result<BatchSubmissionResult, String>
```

**Parameters:**
- `vault_id` (string): The vault ID containing the removal attempts
- `removal_attempt_ids` (string[]): Array of removal attempt IDs to process

**Returns:**
```typescript
interface BatchSubmissionResult {
  job_id: string;      // Unique identifier for this batch job
  total_count: number; // Total number of attempts to process
  queued_count: number; // Number successfully queued for processing
}
```

**Behavior:**
- Spawns async workers for each removal attempt (max 3 concurrent)
- Returns immediately after queueing tasks
- Emits events as tasks progress (see Events section)
- Automatically retries failures up to 3 times with backoff (30s, 2m, 5m)
- Routes CAPTCHAs to CAPTCHA queue (status remains Pending)
- Routes final failures to Failed queue

**Example:**
```typescript
const result = await invoke<BatchSubmissionResult>('process_removal_batch', {
  vaultId: 'vault123',
  removalAttemptIds: ['attempt1', 'attempt2', 'attempt3']
});

console.log(`Processing ${result.total_count} removals (Job: ${result.job_id})`);
```

---

### `get_captcha_queue`

Retrieve all removal attempts that require CAPTCHA solving.

**Signature:**
```rust
#[tauri::command]
pub async fn get_captcha_queue(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<RemovalAttempt>, String>
```

**Parameters:**
- `vault_id` (string): The vault ID to query

**Returns:**
```typescript
interface RemovalAttempt {
  id: string;
  vault_id: string;
  finding_id: string;
  broker_id: string;
  status: 'Pending' | 'Processing' | 'Submitted' | 'Completed' | 'Failed';
  error_message: string | null;  // Contains "CAPTCHA_REQUIRED:<url>" for queue items
  retry_count: number;
  submitted_at: string | null;
  completed_at: string | null;
  created_at: string;
  updated_at: string;
}
```

**Query Logic:**
Returns attempts where `error_message` contains `"CAPTCHA_REQUIRED"` prefix.

**Example:**
```typescript
const captchaQueue = await invoke<RemovalAttempt[]>('get_captcha_queue', {
  vaultId: 'vault123'
});

if (captchaQueue.length > 0) {
  console.log(`${captchaQueue.length} removals need CAPTCHA solving`);
}
```

---

### `get_failed_queue`

Retrieve all removal attempts that failed after retries.

**Signature:**
```rust
#[tauri::command]
pub async fn get_failed_queue(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<RemovalAttempt>, String>
```

**Parameters:**
- `vault_id` (string): The vault ID to query

**Returns:**
```typescript
RemovalAttempt[]  // Same structure as get_captcha_queue
```

**Query Logic:**
Returns attempts where `status = 'Failed'`.

**Example:**
```typescript
const failedQueue = await invoke<RemovalAttempt[]>('get_failed_queue', {
  vaultId: 'vault123'
});

failedQueue.forEach(attempt => {
  console.log(`Failed: ${attempt.broker_id} - ${attempt.error_message}`);
});
```

---

### `retry_removal`

Retry a single failed removal attempt.

**Signature:**
```rust
#[tauri::command]
pub async fn retry_removal(
    state: State<'_, AppState>,
    removal_attempt_id: String,
) -> Result<(), String>
```

**Parameters:**
- `removal_attempt_id` (string): The ID of the removal attempt to retry

**Behavior:**
- Resets status to `Pending`
- Clears `error_message`
- Spawns new worker task to process the attempt
- Emits progress events as task executes

**Example:**
```typescript
try {
  await invoke('retry_removal', {
    removalAttemptId: 'attempt_xyz'
  });
  console.log('Retry initiated');
} catch (error) {
  console.error('Retry failed:', error);
}
```

---

## Events

Subscribe to Tauri events for real-time progress updates:

### `removal:started`

Emitted when a worker task begins processing an attempt.

**Payload:**
```typescript
interface RemovalStartedEvent {
  attempt_id: string;
  broker_id: string;
}
```

**Example:**
```typescript
await listen<RemovalStartedEvent>('removal:started', (event) => {
  console.log(`Started processing ${event.payload.broker_id}`);
});
```

---

### `removal:success`

Emitted when a removal is successfully submitted.

**Payload:**
```typescript
interface RemovalSuccessEvent {
  attempt_id: string;
  status: 'Submitted' | 'Completed';  // Submitted if email verification needed
}
```

**Example:**
```typescript
await listen<RemovalSuccessEvent>('removal:success', (event) => {
  console.log(`Success: ${event.payload.attempt_id}`);
  updateProgressUI();
});
```

---

### `removal:captcha`

Emitted when a CAPTCHA is detected and the attempt is routed to CAPTCHA queue.

**Payload:**
```typescript
interface RemovalCaptchaEvent {
  attempt_id: string;
  captcha_url: string;  // URL where CAPTCHA was encountered
}
```

**Example:**
```typescript
await listen<RemovalCaptchaEvent>('removal:captcha', (event) => {
  showCaptchaNotification(event.payload.attempt_id);
  refreshCaptchaQueue();
});
```

---

### `removal:failed`

Emitted when a removal fails after all retry attempts.

**Payload:**
```typescript
interface RemovalFailedEvent {
  attempt_id: string;
  error: string;  // Human-readable error message
}
```

**Example:**
```typescript
await listen<RemovalFailedEvent>('removal:failed', (event) => {
  console.error(`Failed: ${event.payload.error}`);
  refreshFailedQueue();
});
```

---

### `removal:retry`

Emitted when a retry attempt is starting (informational).

**Payload:**
```typescript
interface RemovalRetryEvent {
  attempt_id: string;
  attempt_num: number;  // 1, 2, or 3
}
```

**Example:**
```typescript
await listen<RemovalRetryEvent>('removal:retry', (event) => {
  console.log(`Retry ${event.payload.attempt_num}/3 for ${event.payload.attempt_id}`);
});
```

---

## Complete Workflow Example

```typescript
import { invoke, listen } from '@tauri-apps/api';

// 1. Set up event listeners
await listen<RemovalSuccessEvent>('removal:success', (event) => {
  successCount++;
  updateProgress();
});

await listen<RemovalCaptchaEvent>('removal:captcha', (event) => {
  captchaCount++;
  updateProgress();
});

await listen<RemovalFailedEvent>('removal:failed', (event) => {
  failedCount++;
  updateProgress();
});

// 2. Start batch processing
const result = await invoke<BatchSubmissionResult>('process_removal_batch', {
  vaultId: currentVaultId,
  removalAttemptIds: selectedAttempts.map(a => a.id)
});

console.log(`Processing ${result.total_count} removals`);

// 3. Events will fire as tasks complete
// UI updates automatically via event handlers

// 4. After completion, check queues
const captchaQueue = await invoke<RemovalAttempt[]>('get_captcha_queue', {
  vaultId: currentVaultId
});

const failedQueue = await invoke<RemovalAttempt[]>('get_failed_queue', {
  vaultId: currentVaultId
});

// 5. Handle user retry action
async function retryFailed(attemptId: string) {
  try {
    await invoke('retry_removal', { removalAttemptId: attemptId });
  } catch (error) {
    showError(error);
  }
}
```

---

## Implementation Status

**Complete:**
- ✅ All commands implemented and tested
- ✅ Event emission for real-time updates
- ✅ Concurrent processing with semaphore limiting
- ✅ Automatic retry with exponential backoff
- ✅ Queue routing (CAPTCHA, Failed)
- ✅ Database state persistence
- ✅ Integration tests

**Pending (Frontend):**
- ⏳ Progress dashboard UI
- ⏳ CAPTCHA queue screen
- ⏳ Failed queue screen
- ⏳ Event listener integration

---

## Notes

- **Concurrency Limit:** Max 3 browser instances run concurrently (controlled by semaphore)
- **Retry Backoff:** 30 seconds, 2 minutes, 5 minutes between attempts
- **CAPTCHA Handling:** Currently requires manual solving (Phase 6 will add automation)
- **State Persistence:** All state stored in database, safe across app restarts
- **Error Messages:** Stored in `error_message` field for debugging and user display
