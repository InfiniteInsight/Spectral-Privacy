# Phase 5: Removal Form Submission - Design Document

**Date:** 2026-02-15
**Status:** Approved
**Dependencies:** Phase 4 (Findings Storage and Verification)

## Overview

Phase 5 implements automated removal form submission to data brokers. Building on Phase 4's findings verification workflow, this phase automates the actual opt-out request submission using browser automation with intelligent queue management for CAPTCHAs, email verification, and failures.

## Goals

1. **Automate removal submission** - Fill and submit opt-out forms to data brokers
2. **Handle async workflows** - Manage CAPTCHA solving and email verification
3. **Parallel processing** - Submit multiple removals concurrently while allowing user interaction
4. **Graceful degradation** - Route problematic submissions to manual queues
5. **Transparent progress** - Real-time UI updates during batch processing

## User Workflow

```
1. User verifies findings (Phase 4)
   ↓
2. Clicks "Submit Removals" → Creates pending removal attempts
   ↓
3. Reviews batch on confirmation screen
   ↓
4. Clicks "Start Submission" → Parallel processing begins
   ↓
5. System processes removals in background (3 concurrent)
   - Success: Marks as "Submitted"
   - CAPTCHA: Adds to CAPTCHA queue
   - Error: Retries, then adds to Failed queue
   ↓
6. User solves CAPTCHAs (if any) while others process
   ↓
7. Email verification handling (optional automation)
   ↓
8. Review completion summary
```

## Architecture

### Pattern: Tokio Task Queue with Workers

**Core Components:**
1. **Tauri Command Layer** - Entry points for UI
2. **Task Spawner** - Creates async worker tasks
3. **Worker Pool** - Semaphore-limited (max 3 concurrent)
4. **Queue Router** - Directs outcomes to appropriate queues
5. **Status Tracker** - Updates database, emits events

**Concurrency Model:**
- Each removal attempt runs as independent Tokio task
- Semaphore limits concurrent browser instances (3 max)
- Tasks don't block each other - failures are isolated
- User can solve CAPTCHAs while others continue processing

**State Management:**
- Database is source of truth (RemovalStatus enum)
- Queues implemented via database queries + status filters
- UI subscribes to Tauri events for real-time updates
- Stateless workers - no complex in-memory coordination

## Components

### 1. Tauri Commands

#### `process_removal_batch`
```rust
#[tauri::command]
pub async fn process_removal_batch(
    state: State<'_, AppState>,
    vault_id: String,
    removal_attempt_ids: Vec<String>,
) -> Result<BatchSubmissionResult, String>
```

**Responsibilities:**
- Validates vault is unlocked
- Spawns worker tasks for each removal attempt
- Returns immediately with batch job ID
- Emits progress events as tasks complete

**Returns:**
```rust
struct BatchSubmissionResult {
    job_id: String,
    total_count: usize,
    queued_count: usize,
}
```

#### `solve_captcha_guided`
```rust
#[tauri::command]
pub async fn solve_captcha_guided(
    state: State<'_, AppState>,
    removal_attempt_id: String,
) -> Result<(), String>
```

**Responsibilities:**
- Opens browser at CAPTCHA URL
- Waits for user to solve (shows browser window)
- Resumes submission after completion
- Runs as background task (non-blocking)

#### `get_captcha_queue`
```rust
#[tauri::command]
pub async fn get_captcha_queue(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<RemovalAttempt>, String>
```

**Returns:** All removal attempts with `error_message` containing "CAPTCHA_REQUIRED"

#### `get_failed_queue`
```rust
#[tauri::command]
pub async fn get_failed_queue(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<Vec<RemovalAttempt>, String>
```

**Returns:** All removal attempts with `status = Failed`

#### `retry_removal`
```rust
#[tauri::command]
pub async fn retry_removal(
    state: State<'_, AppState>,
    removal_attempt_id: String,
) -> Result<(), String>
```

**Responsibilities:**
- Resets status to Pending
- Clears error_message
- Spawns new worker task for retry

### 2. Worker Task

```rust
async fn submit_removal_task(
    db: Arc<Database>,
    removal_attempt_id: String,
    event_emitter: EventEmitter,
    semaphore: Arc<Semaphore>,
) -> RemovalOutcome
```

**Process:**
1. Acquire semaphore (wait if 3 tasks active)
2. Load removal attempt from database
3. Load associated finding and profile data
4. Map profile fields to form fields:
   - `listing_url` ← finding.listing_url
   - `email` ← profile.email
   - `first_name` ← profile.first_name
   - `last_name` ← profile.last_name
   - Additional fields as needed by broker
5. Load broker definition
6. Call `WebFormSubmitter::submit(broker_def, field_values)`
7. Handle outcome and update database
8. Emit event to UI
9. Release semaphore

### 3. Queue Management

**CAPTCHA Queue:**
- Query: `status = 'Pending' AND error_message LIKE 'CAPTCHA_REQUIRED%'`
- UI displays: "X removals need CAPTCHA"
- User clicks "Solve CAPTCHAs" → Guided workflow

**Failed Queue:**
- Query: `status = 'Failed'`
- UI displays: Error message + "Retry" button
- User can retry individually or in batch

**Success Queue:**
- Query: `status = 'Submitted' OR status = 'Completed'`
- UI displays: Success count + completion stats

### 4. Email Monitor (Optional Feature)

**Configuration:**
```rust
struct EmailMonitorSettings {
    enabled: bool,
    mode: EmailMode,  // Assisted | Auto
    imap_config: Option<ImapConfig>,
}

enum EmailMode {
    Assisted,  // Notify user, user clicks link
    Auto,      // Auto-click with warning about false positives
}
```

**Detection Logic:**
- Monitor inbox for emails matching patterns:
  - From: Known broker domains
  - Subject: Contains "verify", "confirm", "opt-out"
- Extract verification link from email body
- Mode Assisted: Emit event with link
- Mode Auto: Click link, update status to Completed

**Implementation Note:** Email monitoring runs as separate background service, not in submission worker tasks.

## Data Flow

### Removal Attempt State Machine

```
┌─────────┐
│ Pending │ ← Created by submit_removals_for_confirmed (Phase 4)
└────┬────┘
     │
     ▼
┌──────────────┐
│ Processing   │ ← Worker task spawned
└──────┬───────┘
       │
       ├─► Success ──────┐
       │                 ▼
       │         ┌───────────┐
       │         │ Submitted │ ← Form submitted, may need email verification
       │         └─────┬─────┘
       │               │
       │               ▼
       │         ┌───────────┐
       │         │ Completed │ ← Email verified (final state)
       │         └───────────┘
       │
       ├─► CAPTCHA ─────┐
       │                ▼
       │         ┌─────────────┐
       │         │ CAPTCHA     │ ← User solves → Retry from Processing
       │         │ Queue       │
       │         └─────────────┘
       │
       └─► Error ───────┐
                        ▼
                 ┌────────────┐
                 │ Retry      │ ← 3 attempts with backoff
                 │ (1, 2, 3)  │
                 └──────┬─────┘
                        │
                        ├─► Success → Submitted
                        │
                        └─► Failed ──┐
                                     ▼
                              ┌────────────┐
                              │ Failed     │ ← Final state (can manual retry)
                              │ Queue      │
                              └────────────┘
```

### Outcome Mapping

| WebFormSubmitter Outcome | RemovalStatus | Additional Actions |
|--------------------------|---------------|-------------------|
| `Submitted` | `Submitted` | Set `submitted_at` timestamp |
| `RequiresEmailVerification` | `Submitted` | Set `submitted_at`, may auto-verify later |
| `RequiresCaptcha` | `Pending` | Set `error_message = "CAPTCHA_REQUIRED:<url>"` |
| `Failed` | (Retry logic) | Retry 3x, then `Failed` + `error_message` |

### Retry Logic

```rust
async fn retry_with_backoff(task_fn, max_attempts: u32) -> Result<T, Error> {
    for attempt in 1..=max_attempts {
        match task_fn().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt < max_attempts => {
                let delay = match attempt {
                    1 => Duration::from_secs(30),      // 30 seconds
                    2 => Duration::from_secs(120),     // 2 minutes
                    _ => Duration::from_secs(300),     // 5 minutes
                };
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Error Handling

### Browser/Network Errors

| Error Type | Handling |
|------------|----------|
| Navigation timeout (>30s) | Retry with backoff |
| Selector not found | Log error, retry once, fail with "Form changed" |
| Network disconnection | Retry with backoff |
| Browser crash | Restart browser engine, retry |
| Rate limiting (429) | Wait 5 minutes, retry |

### Broker-Specific Issues

| Issue | Handling |
|-------|----------|
| Site down (5xx) | Retry later, fail after 3 attempts |
| Form validation error | Capture message, mark failed with details |
| Unexpected redirect | Log URL, mark failed with "Unexpected redirect" |
| Success indicator not found | Mark failed: "Success confirmation not detected" |

### Data Issues

| Issue | Handling |
|-------|----------|
| Missing required field (email) | Skip, mark failed: "Missing required field: email" |
| Invalid listing URL | Mark failed: "Invalid listing URL" |
| Finding already processed | Skip with warning |

### Edge Cases

**Race Conditions:**
- Multiple tasks updating same attempt → Database transactions handle it
- App closes mid-submission → Tasks continue until completion (Tokio runtime)
- Vault locked during submission → Fail gracefully, save progress

**CAPTCHA Scenarios:**
- User closes CAPTCHA browser → Task waits, shows "Awaiting CAPTCHA" status
- CAPTCHA expires → User can retry CAPTCHA flow
- Wrong CAPTCHA solution → Retry CAPTCHA (max 3 times)

**Email Verification:**
- Email never arrives → Stays "Submitted", user can manually mark complete/failed
- Multiple verification emails → Process first, ignore duplicates
- Expired link → Show error, user requests new email from broker

**Concurrency:**
- Max 3 concurrent browsers (semaphore)
- If all waiting on CAPTCHA → New tasks queue
- Task timeout: >5 minutes → Kill and retry

## Events

Real-time UI updates via Tauri events:

| Event | Payload | Description |
|-------|---------|-------------|
| `removal:started` | `{ attempt_id, broker_id }` | Task begins processing |
| `removal:progress` | `{ attempt_id, stage }` | Form navigation/submission stage |
| `removal:success` | `{ attempt_id, status }` | Successfully submitted |
| `removal:captcha` | `{ attempt_id, url }` | CAPTCHA detected, added to queue |
| `removal:failed` | `{ attempt_id, error }` | Failed after retries |
| `removal:retry` | `{ attempt_id, attempt_num }` | Retry attempt starting |
| `removal:batch_complete` | `{ total, succeeded, captcha, failed }` | All tasks finished |

## Testing Strategy

### Unit Tests

- **Field mapping** - Profile data → Form fields conversion
- **Outcome routing** - RemovalOutcome → RemovalStatus mapping
- **Retry logic** - Verify backoff timing and attempt counting
- **Queue queries** - Database filters for CAPTCHA/Failed queues

### Integration Tests

- **End-to-end with mock broker** - Submit form, verify status updates
- **CAPTCHA detection** - Mock page with CAPTCHA, verify routing
- **Concurrent processing** - Spawn 5 tasks, verify all complete independently
- **Database state transitions** - Pending → Submitted → Completed
- **Retry behavior** - Force errors, verify 3 attempts with delays

### Manual Testing

1. **Happy path** - Submit 3 removals, all succeed
2. **CAPTCHA flow** - Submit batch, solve CAPTCHA mid-process, verify others continue
3. **Mixed outcomes** - 2 succeed, 1 CAPTCHA, 1 failed
4. **Retry behavior** - Simulate network error, verify retries
5. **Email verification** - Submit, receive email, click link, verify completion
6. **Failed queue** - Force failures, verify retry option works

### Browser Testing

- Real broker forms (Spokeo, BeenVerified, Radaris)
- Different CAPTCHA types (reCAPTCHA, hCaptcha)
- Form variations (different required fields)

### Performance Testing

- Batch of 20 removals - UI stays responsive
- Concurrent limit - No more than 3 browsers open
- Memory usage - No leaks during long batches

## UI Components

### Batch Review Screen
- List of pending removals (broker name, finding details)
- Summary: "X removals ready to submit"
- "Start Submission" button
- Option to exclude specific removals

### Progress Dashboard
- Real-time progress bar
- Status breakdown:
  - ✓ Submitted: X
  - ⏳ Processing: X
  - 🧩 CAPTCHA Needed: X
  - ❌ Failed: X
- "View CAPTCHA Queue" button (if any)
- "View Failed Queue" button (if any)

### CAPTCHA Queue Screen
- List of removals needing CAPTCHA
- "Solve Next CAPTCHA" button → Opens guided workflow
- Shows broker name and listing URL
- Progress: "2 of 5 CAPTCHAs solved"

### Failed Queue Screen
- List of failed removals with error messages
- "Retry" button per item
- "Retry All Failed" batch button
- Error details expandable

### Email Verification
- **Assisted Mode**: Notification with "Click to Verify" button
- **Auto Mode**: Silent processing + confirmation notification
- **Manual Mode**: "Mark as Verified" button in removal details

## Out of Scope (Phase 6+)

- Advanced CAPTCHA solving (3rd party services like 2captcha)
- Batch scheduling (submit removals at specific times)
- Multi-broker form optimization (reuse sessions)
- Machine learning for form field detection
- Automatic retry scheduling (background retry failed attempts)

## Success Criteria

1. ✅ User can submit batch of verified findings in one click
2. ✅ Multiple forms submit concurrently (3 at a time)
3. ✅ CAPTCHAs don't block entire batch - queue for later
4. ✅ Failed submissions retry automatically (3x with backoff)
5. ✅ Real-time progress updates visible in UI
6. ✅ Email verification can be automated (opt-in)
7. ✅ User can retry failed submissions manually
8. ✅ All state persists in database (survives app restart)
9. ✅ Comprehensive error messages for failures
10. ✅ Integration tests cover concurrent task processing

## Dependencies

**Phase 4 (Complete):**
- Findings storage and verification
- `removal_attempts` table and CRUD operations
- `submit_removals_for_confirmed` command

**Existing Code (In Master):**
- `WebFormSubmitter` - Browser automation for forms
- `RemovalOutcome` enum - Submission outcome states
- CAPTCHA detection logic
- Browser engine (spectral-browser)

**New Dependencies:**
- None - uses existing Tokio, async/await in stack

## Implementation Notes

1. **Semaphore creation**: Use `Arc<Semaphore>` from `tokio::sync`
2. **Event emission**: Use Tauri's `app.emit()` API
3. **Database access in tasks**: Clone `Arc<Database>` handle
4. **Browser lifecycle**: Create one engine per task, dispose after completion
5. **Email monitoring**: Separate service, not coupled to submission tasks
