# Phase 6 Design: Automation, Verification & History

**Date:** 2026-02-17
**Status:** Approved
**Builds on:** Phase 5 (Removal Form Submission + Progress Dashboard)

---

## Overview

Phase 6 completes the unfinished Phase 2 automation work and adds the deferred Phase 5 features. It delivers six user-visible capabilities in order of user impact: job history, privacy score, browser-based removal, email removal, email verification monitoring, and background scheduling.

**Design principle:** Each feature is independently shippable. Features 1 and 2 use only data already in the database. Features 3–6 introduce new backend infrastructure, built incrementally.

---

## Feature Order (User Journey)

| # | Feature | New Infrastructure |
|---|---|---|
| 1 | Job History Page | DB aggregate queries |
| 2 | Privacy Score Page | Score calculation command |
| 3 | Browser-based Removal | Wire `spectral-browser` into worker |
| 4 | Email Removal Flow | `spectral-mail` crate |
| 5 | Email Verification Monitoring | IMAP polling in `spectral-mail` |
| 6 | Scan Scheduler + Tray | `spectral-scheduler` crate |

---

## Shared Foundation: Broker Method Classification

Features 3 and 4 require brokers to declare how their removal is performed. Each broker definition JSON gains a `removal_method` field:

```json
{
  "removal_method": "HttpForm" | "BrowserForm" | "Email"
}
```

- `HttpForm` — direct HTTP form submission (current behaviour, default when field absent)
- `BrowserForm` — headless Chromium via `spectral-browser`
- `Email` — opt-out email via `spectral-mail`

The broker loader deserialises `removal_method` with a default of `HttpForm` so all existing definitions continue to work unchanged.

Email-method brokers additionally require:

```json
{
  "removal_method": "Email",
  "removal_email": "optout@broker.com",
  "email_template": "Dear Sir/Madam,\n\nI am writing to request removal of my personal information...\n\nName: {{name}}\nAddress: {{address}}\nEmail: {{email}}",
  "requires_email_verification": false
}
```

Brokers requiring post-submission email confirmation set `"requires_email_verification": true`.

---

## Feature 1: Job History Page

**Route:** `/removals` (currently a redirect to `/`)

### List View

Replaces the redirect with a page listing all past removal batches. Each scan job that has associated removal attempts appears as a card showing:

- Date submitted
- Total attempts
- Status breakdown: submitted / failed / CAPTCHA pending / awaiting verification
- Link to the existing `/removals/progress/[jobId]` progress dashboard for that job

Cards are sorted newest-first. Empty state shown when no jobs exist.

### Drill-Down View

Clicking a job card expands an inline per-broker breakdown (or navigates to a detail route). Shows each broker attempted with:

- Final status with badge (Submitted / Failed / CAPTCHA / Verified / Reappeared)
- Timestamps (submitted\_at, completed\_at)
- Error message (expandable) if failed
- Retry button for failed attempts (calls existing `retry_removal` command)

Reuses `OverviewTab` and `FailedQueueTab` components from Phase 5 in read-only mode.

### Backend

Two new Tauri commands in `commands/scan.rs`:

```rust
// Returns one entry per scan job that has removal attempts
get_removal_job_history(vault_id: String) -> Vec<RemovalJobSummary>

// RemovalJobSummary includes scan_job_id, submitted_at, total, counts by status
```

`get_removal_attempts_by_scan_job` (already exists) serves the drill-down.

New DB query in `spectral-db/src/removal_attempts.rs`:

```sql
SELECT scan_job_id,
       MIN(created_at) as submitted_at,
       COUNT(*) as total,
       SUM(status = 'Submitted') as submitted_count,
       SUM(status = 'Completed') as completed_count,
       SUM(status = 'Failed') as failed_count,
       SUM(status = 'Pending') as pending_count
FROM removal_attempts
WHERE vault_id = ?
GROUP BY scan_job_id
ORDER BY submitted_at DESC
```

No new database tables required.

---

## Feature 2: Privacy Score Page

**Route:** `/score`
**Navigation:** "Privacy Health" card added to the home dashboard

### Score Calculation

A 0–100 integer derived on-demand from existing scan and removal data. Not stored — recalculated each page load.

```
score = 100
      - (unresolved_findings × weight_per_broker_category)
      + (confirmed_removals × confirmation_bonus)
      - (failed_removals × failure_penalty)
      - (reappeared_listings × reappear_penalty)
```

Broker category weights (higher = more sensitive data):
- People-search / background check: 8 points per unresolved finding
- Data aggregator: 5 points
- Marketing list: 3 points
- Other: 2 points

Clamped to [0, 100]. Descriptors: 0–39 "At Risk", 40–69 "Improving", 70–89 "Good", 90–100 "Well Protected".

### Page Layout

**Top section:** Circular SVG gauge (0–100) with colour band (red → amber → green), numeric score, and plain-English descriptor.

**Middle section:** Per-category breakdown table — columns: Category, Found, Removal Submitted, Confirmed Removed. Rows for each broker category with data.

**Bottom section:** Chronological event timeline — scan runs, removals submitted, removals confirmed/failed/reappeared. Pulled from existing `scan_jobs` and `removal_attempts` tables. Filter chips: All / Scans / Removals / Verified.

### Backend

One new Tauri command:

```rust
get_privacy_score(vault_id: String) -> PrivacyScoreResult
// Returns: score (u8), descriptor (String), category_breakdown (Vec<CategoryStats>)
```

Timeline events assembled from existing scan\_job and removal\_attempt queries — no new DB queries needed.

---

## Feature 3: Browser-based Removal

Upgrades the removal worker to support `BrowserForm` brokers using the existing `spectral-browser` crate (chromiumoxide).

### Worker Routing

`process_removal_batch` currently calls `submit_via_http()` for all brokers. Gains a match on `broker.removal_method`:

```rust
match broker.removal_method {
    RemovalMethod::HttpForm => submit_via_http(&broker, &attempt, &profile).await,
    RemovalMethod::BrowserForm => submit_via_browser(&broker, &attempt, &profile, &browser).await,
    RemovalMethod::Email => submit_via_email(&broker, &attempt, &profile, &mailer).await,
}
```

### Browser Submission (`submit_via_browser`)

1. Acquire the shared `BrowserEngine` from `AppState`
2. Navigate to `broker.removal_url`
3. Fill each field defined in `broker.field_mappings` using `BrowserActions::fill_field()`
4. Check for CAPTCHA presence (detect common CAPTCHA selectors)
5. If CAPTCHA detected: emit `removal:captcha` event (same as today), return `Pending`
6. Submit the form, wait for success selector or timeout (30s)
7. On success: capture screenshot, emit `removal:success` event, return `Submitted`
8. On timeout/error: emit `removal:failed` event, return `Failed`

CAPTCHA handling is identical to the HTTP path — the CAPTCHA URL is opened in the system browser via `tauri-plugin-shell`.

### Browser Lifecycle

A single `BrowserEngine` is held in `AppState` (wrapped in `Arc<Mutex<Option<BrowserEngine>>>`). It is lazily initialised on the first `BrowserForm` attempt and shut down cleanly on app exit via Tauri's `on_window_event`. Avoids spawning multiple Chrome processes.

### Evidence Capture

On successful browser submission, a screenshot is stored:

```sql
CREATE TABLE removal_evidence (
    id TEXT PRIMARY KEY,
    attempt_id TEXT NOT NULL REFERENCES removal_attempts(id),
    screenshot_bytes BLOB NOT NULL,
    captured_at TEXT NOT NULL
);
```

Tauri command `get_removal_evidence(attempt_id)` returns the screenshot bytes for display in the job history drill-down.

### Broker Definition Updates

5–10 known JS-heavy brokers updated to `"removal_method": "BrowserForm"` to validate the new path. Remaining existing definitions stay as `HttpForm`.

---

## Feature 4: Email Removal Flow

### `spectral-mail` Crate

New crate at `crates/spectral-mail/` with four modules:

**`templates.rs`** — generates email body and subject from a broker's `email_template` field with variable substitution (`{{name}}`, `{{address}}`, `{{email}}` from the user's profile). Subject: `"Opt-Out Request — {{name}}"`.

**`sender.rs`** — two sending paths:
- `mailto:` — constructs `mailto:recipient?subject=...&body=...` URL, calls `tauri-plugin-shell`'s `open()`. Default path, requires no configuration.
- SMTP — uses the `lettre` crate with host/port/username/password stored encrypted in the vault. Used when SMTP is configured in settings.

**`log.rs`** — records every sent email to a new `email_removals` table:

```sql
CREATE TABLE email_removals (
    id TEXT PRIMARY KEY,
    attempt_id TEXT REFERENCES removal_attempts(id),
    broker_id TEXT NOT NULL,
    sent_at TEXT NOT NULL,
    method TEXT NOT NULL,       -- 'mailto' | 'smtp'
    recipient TEXT NOT NULL,
    subject TEXT NOT NULL,
    body_hash TEXT NOT NULL     -- SHA-256 of body, not the body itself
);
```

**`imap.rs`** — IMAP poller (see Feature 5).

### Worker Integration

`Email` arm in the worker routing calls `spectral-mail` to generate then send (SMTP) or open (`mailto:`) the email. Attempt status is set to `Submitted` immediately after the send call. If `broker.requires_email_verification` is true, status is set to `Pending` with a note in `error_message`: `"AWAITING_VERIFICATION"`.

### Email Preview Modal

Before sending (both paths), a modal shows the generated email — to, subject, editable body — with "Send" / "Cancel" buttons. Present in the progress dashboard when the user opens a `Pending` email attempt.

### Settings UI

New "Email" section in settings:
- SMTP toggle (off by default)
- Host / Port / Username / Password fields (shown when toggled on)
- "Test connection" button
- IMAP subsection (see Feature 5)

Credentials stored encrypted in vault via existing `spectral-vault` APIs.

---

## Feature 5: Email Verification Monitoring

### Manual Path (Default)

When an attempt has status `Pending` and `error_message = "AWAITING_VERIFICATION"`, the progress dashboard renders a fourth tab **"Pending Verification"** (alongside Overview, CAPTCHA Queue, Failed Queue). Each item shows:

- Broker name
- "Check your inbox for a confirmation email from `broker.com`"
- Expected sender address (from broker definition)
- "Mark as Verified" button → sets attempt to `Completed`, emits `removal:verified`

### IMAP Path (Optional)

When IMAP credentials are configured, `imap.rs` runs a poller:

1. Connects to the configured IMAP server (using `async-imap` crate)
2. Searches `UNSEEN` messages in INBOX from the last 7 days
3. For each message, checks if the sender address matches any broker's `removal_email` value
4. If matched, extracts the confirmation link from the body using a regex defined in the broker definition: `"verification_link_pattern": "https://broker\\.com/confirm/[a-z0-9]+")`
5. Calls `tauri-plugin-shell`'s `open()` to load the link in the system browser
6. Marks the matching attempt `Completed`, emits `removal:verified`
7. Logs the action to `email_removals`

**Safety guardrails:**
- Only acts on senders matching known broker addresses — no fuzzy matching
- `MAX_VERIFICATION_AGE`: 7 days — ignores older emails
- Read-only IMAP access — never sends or modifies messages
- All actions logged to `email_removals`

**Poller cadence:** Every 5 minutes while the app is open. Integrated with the scheduler (Feature 6) for tray mode.

### New Tauri Event

`removal:verified` — mirrors existing event pattern, triggers store update in real-time:

```typescript
interface RemovalVerifiedEvent {
  attempt_id: string;
  broker_id: string;
}
```

### Settings UI

IMAP subsection under Email settings:
- Toggle to enable
- Host / Port / Username / Password
- "Test connection" button

---

## Feature 6: Scan Scheduler & Tray Mode

### `spectral-scheduler` Crate

New crate at `crates/spectral-scheduler/` with three modules:

**`scheduler.rs`** — reads the `scheduled_jobs` table on app start, dispatches any jobs due:

```sql
CREATE TABLE scheduled_jobs (
    id TEXT PRIMARY KEY,
    job_type TEXT NOT NULL,         -- 'ScanAll' | 'VerifyRemovals' | 'PollImap'
    interval_days INTEGER NOT NULL,
    next_run_at TEXT NOT NULL,
    last_run_at TEXT,
    enabled INTEGER NOT NULL DEFAULT 1
);
```

On completion of each job, `next_run_at` is updated to `now + interval_days`.

**`jobs.rs`** — three job types:
- `ScanAll` — triggers a full broker scan (calls existing scan orchestrator)
- `VerifyRemovals` — re-scans brokers with `Submitted` or `Completed` attempts older than 3 days; if listing is gone → `Confirmed`; if listing reappears → `Reappeared` (creates new finding)
- `PollImap` — kicks the IMAP poller if IMAP is configured

**`tray.rs`** — when tray mode is enabled:
- Registers system tray icon via `tauri-plugin-system-tray` with a context menu: "Open Spectral", "Run Scan Now", "Quit"
- Registers autostart via `tauri-plugin-autostart`
- Keeps app alive when window is closed (intercepts `CloseRequested` event)
- Runs a tokio interval (30-minute tick) that calls `scheduler.rs`'s dispatch function

### Default Schedule

| Job | Default Interval | Default State |
|---|---|---|
| ScanAll | 7 days | Enabled |
| VerifyRemovals | 3 days | Enabled |
| PollImap | — | Runs with scheduler tick if IMAP configured |

### Cross-Platform Compatibility

| Platform | Tray Support | Autostart |
|---|---|---|
| macOS | `NSStatusItem` — reliable | `LaunchAgent` plist in `~/Library/LaunchAgents` |
| Windows | `Shell_NotifyIcon` — reliable | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` registry key |
| Linux | Requires `libappindicator3` or `libayatana-appindicator`; not all DEs support it | XDG `~/.config/autostart/<app>.desktop` |

**Linux fallback:** At startup, Spectral attempts to initialise the tray icon. If initialisation fails (missing library or unsupported desktop environment), tray mode silently degrades to on-open-only. The settings UI shows a warning: _"Background scheduling requires a system tray-compatible desktop environment (e.g. XFCE, KDE, MATE)."_ The scheduler, all job types, and the `scheduled_jobs` table are unaffected — only the 30-minute background tick is skipped.

**Packaging:** `libappindicator3` added as `Recommends:` (not `Depends:`) in the `.deb` manifest. CI tests tray initialisation on Ubuntu with appindicator and in a minimal headless environment.

### Settings UI

"Scheduling" section in settings:
- Per-job toggles and interval selectors (3 / 7 / 14 / 30 days)
- Tray mode toggle (with platform caveat note on Linux)
- "Upcoming jobs" list: each job type, last run, next due

---

## New Crates Summary

| Crate | Purpose | Key Dependencies |
|---|---|---|
| `spectral-mail` | Email generation, sending, IMAP monitoring | `lettre`, `async-imap` |
| `spectral-scheduler` | Job scheduling, tray mode, background dispatch | `tauri-plugin-system-tray`, `tauri-plugin-autostart` |

`spectral-browser` already exists — wired in, not created.

---

## New Database Tables

| Table | Purpose |
|---|---|
| `removal_evidence` | Screenshot BLOBs for browser-submitted attempts |
| `email_removals` | Log of all email-based removal actions |
| `scheduled_jobs` | Scheduler state and job configuration |

---

## New Tauri Commands

| Command | Feature |
|---|---|
| `get_removal_job_history(vault_id)` | Job History Page |
| `get_privacy_score(vault_id)` | Privacy Score Page |
| `get_removal_evidence(attempt_id)` | Job History drill-down |
| `send_removal_email(attempt_id)` | Email Removal |
| `mark_attempt_verified(attempt_id)` | Email Verification (manual) |
| `get_scheduled_jobs(vault_id)` | Scheduler settings |
| `update_scheduled_job(job_id, interval, enabled)` | Scheduler settings |
| `run_job_now(job_type)` | Scheduler settings |

---

## New Tauri Events

| Event | Payload | Trigger |
|---|---|---|
| `removal:verified` | `{ attempt_id, broker_id }` | Manual or IMAP auto-verification |
| `scan:scheduled_complete` | `{ job_type, findings_count }` | Scheduler-triggered scan finishes |

---

## Out of Scope (Phase 7+)

- Advanced CAPTCHA solving (vision LLM or 3rd party services like 2captcha)
- LLM-assisted email drafting
- Multi-broker session reuse
- Mobile / iOS / Android support
- Conversational interface

---

## Success Criteria

1. Users can see all past removal jobs in a history page with per-broker detail
2. Users can view a privacy score with category breakdown and event timeline
3. Brokers marked `BrowserForm` submit successfully via headless Chrome
4. Brokers marked `Email` send removal emails via `mailto:` or SMTP
5. Email verification can be handled manually or automatically via IMAP
6. Scans and verification checks run automatically on app open
7. Tray mode enables background scheduling on macOS, Windows, and supported Linux DEs
8. All features work cross-platform (Linux degradation is graceful, documented, not silent failure)
