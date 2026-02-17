# Phase 6 Design: Automation, Verification & History

**Date:** 2026-02-17
**Status:** Approved (updated to include outpaced features)
**Builds on:** Phase 5 (Removal Form Submission + Progress Dashboard)

---

## Overview

Phase 6 completes all deferred and outpaced work across Phases 1–5, plus the original Phase 2 automation features. It delivers twelve user-visible capabilities in user-journey order: vault management, settings foundation, job history, privacy score, dashboard enhancement, browser removal, email removal, email verification, scheduling, broker explorer, proactive scanning tiers, and local PII discovery.

**Design principle:** Each feature is independently shippable. Features 1–2 are foundations that unblock later features. Features 3–5 use only data already in the database. Features 6–9 introduce new backend infrastructure. Features 10–12 add discovery and informational surfaces.

---

## Feature Order (User Journey)

| # | Feature | New Infrastructure | Previously |
|---|---|---|---|
| 1 | Multi-Vault UI | None — backend already complete | Task 1.5 (never built) |
| 2 | Settings Page | None — structural frontend | Architecture doc (never built) |
| 3 | Job History Page | DB aggregate queries | Phase 6 original Feature 1 |
| 4 | Privacy Score Page | Score calculation command | Phase 6 original Feature 2 |
| 5 | Dashboard Enhancement | None — uses existing data | Architecture doc (partial) |
| 6 | Browser-based Removal | Wire `spectral-browser` into worker | Phase 6 original Feature 3 |
| 7 | Email Removal Flow | `spectral-mail` crate | Phase 6 original Feature 4 |
| 8 | Email Verification Monitoring | IMAP polling in `spectral-mail` | Phase 6 original Feature 5 |
| 9 | Scan Scheduler + Tray | `spectral-scheduler` crate | Phase 6 original Feature 6 |
| 10 | Broker Explorer | None — reads existing broker definitions | Architecture doc (never built) |
| 11 | Proactive Scanning Tiers | Broker definition schema extension | Architecture doc (never built) |
| 12 | Local PII Discovery | `spectral-discovery` crate | Architecture doc (never built) |

---

## Shared Foundation: Broker Method Classification

Features 6 and 7 require brokers to declare how their removal is performed. Each broker definition TOML gains a `removal_method` field:

```toml
removal_method = "HttpForm"  # | "BrowserForm" | "Email"
```

- `HttpForm` — direct HTTP form submission (current behaviour, default when field absent)
- `BrowserForm` — headless Chromium via `spectral-browser`
- `Email` — opt-out email via `spectral-mail`

The broker loader deserialises `removal_method` with a default of `HttpForm` so all existing definitions continue to work unchanged.

Email-method brokers additionally require:

```toml
removal_method = "Email"
removal_email = "optout@broker.com"
email_template = """Dear Sir/Madam,

I am writing to request removal of my personal information...

Name: {{name}}
Address: {{address}}
Email: {{email}}"""
requires_email_verification = false
```

Brokers requiring post-submission email confirmation set `requires_email_verification = true`.

---

## Feature 1: Multi-Vault UI

**Status of backend:** Complete. `list_vaults`, `vault_create`, `vault_unlock`, `vault_lock`, `vault_status` all implemented. `AppState` uses `HashMap<String, Arc<Vault>>` supporting multiple simultaneously unlocked vaults. Tests for multiple vaults exist and pass.

**What is missing:** A vault switcher/selector in the frontend. The vault store already has `availableVaults`, `setCurrentVault()`, and all API calls. This is a pure UI gap.

### Vault Switcher (Navigation)

A vault indicator in the top navigation bar shows the current vault's display name and an expand icon. Clicking opens a dropdown listing all available vaults with:

- Display name and last-accessed timestamp
- Lock/unlock icon indicating current state
- "Switch" button for unlocked vaults (calls `vaultStore.setCurrentVault()` and reloads profile/scan/removal stores)
- "Unlock" button for locked vaults (opens password modal)
- "Add Vault" option at the bottom

On vault switch, all downstream stores (`profileStore`, `scanStore`, `removalStore`) reset and reload for the new vault context.

### Vault Management (Settings → Vaults)

A "Vaults" section in the Settings page (Feature 2) provides:

- List of all vaults with display name, created date, last accessed date
- Rename vault (updates `metadata.json` display name via new `rename_vault` command)
- Change password (calls new `change_vault_password` command)
- Delete vault (confirmation prompt; removes vault directory after locking)
- Create new vault (same flow as initial onboarding)

### New Tauri Commands

```rust
rename_vault(vault_id: String, new_name: String) -> Result<(), CommandError>
change_vault_password(vault_id: String, old_password: String, new_password: String) -> Result<(), CommandError>
delete_vault(vault_id: String, password: String) -> Result<(), CommandError>
```

No new database tables required — uses existing filesystem structure.

---

## Feature 2: Settings Page

**Route:** `/settings`
**Navigation:** Gear icon in the top navigation bar

The Settings page is a prerequisite for Features 7, 8, and 9 (email SMTP/IMAP credentials, scheduler configuration). It is built as a tabbed layout with sections added incrementally as each feature is implemented.

### Sections

**Vaults** (Feature 1)
- Vault list with management actions (see Feature 1)

**Profile**
- Link to existing `/profile/setup` flow for editing PII

**Privacy Level** (Permission Presets)
Four preset cards: Paranoid / Local Privacy (recommended) / Balanced / Custom. Selecting a preset updates the active permission set via a new `set_permission_preset(preset: String)` command. Changing preset shows a confirmation dialog. Current active preset shown with a badge.

**Email** (Feature 7)
- SMTP toggle (off by default)
- Host / Port / Username / Password (shown when toggled on)
- "Test connection" button
- IMAP subsection (Feature 8): Toggle, Host / Port / Username / Password, "Test connection"

Credentials stored encrypted in vault via existing `spectral-vault` APIs.

**Scheduling** (Feature 9)
- Per-job toggles and interval selectors
- Tray mode toggle
- Upcoming jobs list

**Privacy Audit Log**
Read-only log of all PII access events. Columns: Timestamp, Action, Subject, Fields Accessed, Destination. Sourced from `audit_log` table (new — see below). Filter by date range and event type.

### New Database Table

```sql
CREATE TABLE audit_log (
    id TEXT PRIMARY KEY,
    vault_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    subject TEXT NOT NULL,
    pii_fields TEXT,            -- JSON array of field names (never values)
    data_destination TEXT NOT NULL,  -- 'LocalOnly' | 'ExternalSite:domain' | 'CloudLlm:provider'
    outcome TEXT NOT NULL       -- 'Allowed' | 'Denied'
);
```

### New Tauri Commands

```rust
set_permission_preset(vault_id: String, preset: String) -> Result<(), CommandError>
get_permission_preset(vault_id: String) -> Result<String, CommandError>
get_audit_log(vault_id: String, limit: u32, offset: u32) -> Result<Vec<AuditEntry>, CommandError>
test_smtp_connection(host: String, port: u16, username: String, password: String) -> Result<(), CommandError>
test_imap_connection(host: String, port: u16, username: String, password: String) -> Result<(), CommandError>
```

---

## Feature 3: Job History Page

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

## Feature 4: Privacy Score Page

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

## Feature 5: Dashboard Enhancement

**Route:** `/` (existing home page)

Upgrades the home page from its current minimal state (vault status + "Start Scan" button) to a proper privacy dashboard using data already in the database.

### Dashboard Cards

**Privacy Score Card** — Current score (0–100) with grade badge and descriptor. Clicking navigates to `/score`. Score delta from previous week shown if previous scan data exists.

**Scan Coverage Card** — "X of Y known brokers scanned." Last scan date. "Scan Now" button. If never scanned: "No scan yet — start your first scan."

**Active Removals Card** — Count of in-progress removals by status (Submitted / Pending / Failed). Clicking navigates to `/removals`. If no removals: "No removals started yet."

**Recent Activity Feed** — Last 10 events across all vault activity: scans completed, findings found, removals submitted, removals confirmed/failed, verifications completed. Each item shows timestamp, icon, and one-line description. Sourced from `scan_jobs` and `removal_attempts` — no new tables.

### Backend

One new Tauri command:

```rust
get_dashboard_summary(vault_id: String) -> DashboardSummary
// Returns: privacy_score (Option<u8>), brokers_scanned (u32), brokers_total (u32),
//          last_scan_at (Option<String>), active_removals (RemovalCounts),
//          recent_events (Vec<ActivityEvent>)
```

Network monitoring metrics (broker contact trends, tracker activity) are explicitly deferred to Phase 7 — they require `spectral-netmon` infrastructure not yet built.

---

## Feature 6: Browser-based Removal

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

5–10 known JS-heavy brokers updated to `removal_method = "BrowserForm"` to validate the new path. Remaining existing definitions stay as `HttpForm`.

---

## Feature 7: Email Removal Flow

### `spectral-mail` Crate

New crate at `crates/spectral-mail/` with four modules:

**`templates.rs`** — generates email body and subject from a broker's `email_template` field with variable substitution (`{{name}}`, `{{address}}`, `{{email}}` from the user's profile). Subject: `"Opt-Out Request — {{name}}"`.

**`sender.rs`** — two sending paths:
- `mailto:` — constructs `mailto:recipient?subject=...&body=...` URL, calls `tauri-plugin-shell`'s `open()`. Default path, requires no configuration.
- SMTP — uses the `lettre` crate with host/port/username/password stored encrypted in the vault. Used when SMTP is configured in Settings (Feature 2).

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

**`imap.rs`** — IMAP poller (see Feature 8).

### Worker Integration

`Email` arm in the worker routing calls `spectral-mail` to generate then send (SMTP) or open (`mailto:`) the email. Attempt status is set to `Submitted` immediately after the send call. If `broker.requires_email_verification` is true, status is set to `Pending` with a note in `error_message`: `"AWAITING_VERIFICATION"`.

### Email Preview Modal

Before sending (both paths), a modal shows the generated email — to, subject, editable body — with "Send" / "Cancel" buttons. Present in the progress dashboard when the user opens a `Pending` email attempt.

---

## Feature 8: Email Verification Monitoring

### Manual Path (Default)

When an attempt has status `Pending` and `error_message = "AWAITING_VERIFICATION"`, the progress dashboard renders a fourth tab **"Pending Verification"** (alongside Overview, CAPTCHA Queue, Failed Queue). Each item shows:

- Broker name
- "Check your inbox for a confirmation email from `broker.com`"
- Expected sender address (from broker definition)
- "Mark as Verified" button → sets attempt to `Completed`, emits `removal:verified`

### IMAP Path (Optional)

When IMAP credentials are configured in Settings (Feature 2), `imap.rs` runs a poller:

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

**Poller cadence:** Every 5 minutes while the app is open. Integrated with the scheduler (Feature 9) for tray mode.

### New Tauri Event

`removal:verified` — mirrors existing event pattern, triggers store update in real-time:

```typescript
interface RemovalVerifiedEvent {
  attempt_id: string;
  broker_id: string;
}
```

---

## Feature 9: Scan Scheduler & Tray Mode

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

---

## Feature 10: Broker Explorer

**Route:** `/brokers`
**Navigation:** "Browse Brokers" link in the main navigation

A searchable, filterable table of all broker definitions loaded at startup. No new backend infrastructure — reads from the existing broker registry already loaded into `AppState`.

### List View

Table with columns: Name, Category, Region, Removal Method, Difficulty, Status (Not Scanned / Found / Not Found / Removed).

Filters:
- Category (People Search / Background Check / Data Aggregator / Marketing / Platform / Other)
- Region (US / EU / UK / Canada / Australia / Global)
- Status (based on current vault's scan data)
- Removal Method (HttpForm / BrowserForm / Email)

Search box filters by broker name or domain.

### Broker Detail Page

**Route:** `/brokers/[brokerId]`

Shows:
- Broker name, domain, category, region tags
- Opt-out method with a direct link to the removal URL
- Privacy policy URL
- Typical response time (if known in definition)
- User's scan status for this broker: last scanned, result, removal status
- "Scan this broker" button → triggers single-broker scan
- "Request removal" button (if finding exists) → adds to removal queue

### New Tauri Commands

```rust
list_brokers() -> Vec<BrokerSummary>
// BrokerSummary: id, name, domain, category, region_relevance, removal_method, typical_response_days

get_broker_detail(broker_id: String, vault_id: String) -> BrokerDetail
// BrokerDetail: BrokerSummary + user's scan/removal status for this broker
```

---

## Feature 11: Proactive Scanning Tiers

Upgrades the scan orchestrator from flat user-initiated scans to a tiered, region-aware model. Broker definitions gain two new TOML fields:

```toml
scan_priority = "AutoScanTier1"  # | "AutoScanTier2" | "OnRequest" | "ManualOnly"
region_relevance = ["US", "Global"]  # | "EU" | "UK" | "Canada" | "Australia"
```

### Tier Definitions

**Tier 1** (`AutoScanTier1`) — Highest-traffic people-search brokers for the user's region. Auto-scanned on first run and on scheduled `ScanAll` jobs. ~20 US brokers, ~10 EU brokers.

Examples (US): Spokeo, BeenVerified, WhitePages, FastPeopleSearch, TruePeopleSearch, Intelius, Radaris, MyLife, Instant Checkmate.

**Tier 2** (`AutoScanTier2`) — Secondary aggregators. Auto-scanned on second scheduled pass or explicit "Full Scan". ~20 additional brokers.

Examples: Acxiom, LexisNexis, TowerData, Truthfinder, ZabaSearch, FamilyTreeNow.

**On Request** (`OnRequest`) — Scanned only when user explicitly selects them in the Broker Explorer.

**Manual Only** (`ManualOnly`) — Phone/postal-mail-only opt-out, no automation possible. Shown in Broker Explorer with manual instructions only.

### First-Run Auto-Scan

On first app use (detected by absence of any `scan_jobs` record), after profile setup completes:
- A prompt offers "Start your first scan — checks the ~20 most common data brokers for your region (~15–30 min)"
- Accepting triggers a `ScanAll` scoped to Tier 1 brokers matching `user.jurisdiction.region`
- Progress shown on the existing scan progress page

### Region Filtering

`start_scan()` command gains an optional `tier` parameter. When `tier` is `Tier1` or `Tier2`, the orchestrator filters broker list by `scan_priority` and `region_relevance` matching the user's jurisdiction stored in their profile.

### Broker Definition Migration

Existing broker TOML files gain `scan_priority` and `region_relevance` fields. A migration script (`scripts/migrate_broker_definitions.py`) assigns sensible defaults to all existing definitions based on broker category.

### New Tauri Commands

```rust
start_scan(vault_id: String, tier: Option<ScanTier>, broker_ids: Option<Vec<String>>) -> Result<String, CommandError>
// ScanTier enum: Tier1, Tier2, All, Custom
```

---

## Feature 12: Local PII Discovery

**Route:** `/discovery`
**Navigation:** "Local Discovery" link in the main navigation

Scans the local filesystem, browser data, and (if IMAP configured) email headers for PII matching the user's profile. Helps users understand where their personal data exists on their own machine.

### `spectral-discovery` Crate

New crate at `crates/spectral-discovery/` with three modules:

**`filesystem.rs`** — Scans configured filesystem paths for PII patterns:
- Regex-based detection: email addresses, phone numbers, SSN patterns, name+address co-occurrence
- File types: `.txt`, `.pdf`, `.docx`, `.csv`, `.json`, `.md`
- Configurable paths (defaults: Documents, Downloads, Desktop)
- Max depth: 5 levels

**`browser.rs`** — Reads browser profile data (with user permission):
- Saved passwords: extract domains, flag if domain is a known broker
- Browser history: flag visits to known broker domains
- Browsers supported: Chrome/Chromium, Firefox, Safari (macOS only)

**`email_headers.rs`** — When IMAP is configured, scans email headers (sender, subject) from the last 90 days:
- Flags emails from known broker domains
- Does not read email bodies

### Findings Model

```sql
CREATE TABLE discovery_findings (
    id TEXT PRIMARY KEY,
    vault_id TEXT NOT NULL,
    source TEXT NOT NULL,        -- 'filesystem' | 'browser' | 'email'
    source_detail TEXT NOT NULL, -- file path, browser name, or email folder
    finding_type TEXT NOT NULL,  -- 'pii_exposure' | 'broker_contact' | 'broker_account'
    risk_level TEXT NOT NULL,    -- 'critical' | 'medium' | 'informational'
    description TEXT NOT NULL,
    recommended_action TEXT,
    remediated INTEGER NOT NULL DEFAULT 0,
    found_at TEXT NOT NULL
);
```

### Page Layout

**Top section:** Summary counts — Critical / Medium / Informational findings. "Run Discovery Scan" button.

**Middle section:** Findings grouped by source (Filesystem / Browser / Email). Each finding shows:
- Risk badge
- Description ("File `~/Documents/tax2023.pdf` contains name, address, and SSN")
- Recommended action ("Review file — consider moving to encrypted storage")
- "Mark Resolved" button

**Bottom section:** Scan history — when each discovery scan ran, how many findings were found.

### New Tauri Commands

```rust
start_discovery_scan(vault_id: String) -> Result<String, CommandError>
// Returns discovery_job_id

get_discovery_findings(vault_id: String) -> Result<Vec<DiscoveryFinding>, CommandError>
mark_finding_remediated(finding_id: String) -> Result<(), CommandError>
```

### Permissions Required

Discovery scan requests `FileSystemRead` permission on first run. The permission prompt shows:
- "Spectral wants to scan your Documents, Downloads, and Desktop folders for personal information"
- "This scan runs locally — no data leaves your device"
- Allow / Allow Once / Deny

---

## New Crates Summary

| Crate | Purpose | Key Dependencies |
|---|---|---|
| `spectral-mail` | Email generation, sending, IMAP monitoring | `lettre`, `async-imap` |
| `spectral-scheduler` | Job scheduling, tray mode, background dispatch | `tauri-plugin-system-tray`, `tauri-plugin-autostart` |
| `spectral-discovery` | Local PII discovery across filesystem, browser, email | `pdf-extract`, `regex` |

`spectral-browser` already exists — wired in, not created.

---

## New Database Tables

| Table | Purpose | Feature |
|---|---|---|
| `audit_log` | Privacy audit trail for Settings viewer | 2 |
| `removal_evidence` | Screenshot BLOBs for browser-submitted attempts | 6 |
| `email_removals` | Log of all email-based removal actions | 7 |
| `scheduled_jobs` | Scheduler state and job configuration | 9 |
| `discovery_findings` | Local PII discovery results | 12 |

---

## New Tauri Commands

| Command | Feature |
|---|---|
| `rename_vault(vault_id, new_name)` | 1 — Multi-Vault |
| `change_vault_password(vault_id, old_password, new_password)` | 1 — Multi-Vault |
| `delete_vault(vault_id, password)` | 1 — Multi-Vault |
| `set_permission_preset(vault_id, preset)` | 2 — Settings |
| `get_permission_preset(vault_id)` | 2 — Settings |
| `get_audit_log(vault_id, limit, offset)` | 2 — Settings |
| `test_smtp_connection(host, port, username, password)` | 2 — Settings |
| `test_imap_connection(host, port, username, password)` | 2 — Settings |
| `get_removal_job_history(vault_id)` | 3 — Job History |
| `get_privacy_score(vault_id)` | 4 — Privacy Score |
| `get_dashboard_summary(vault_id)` | 5 — Dashboard |
| `get_removal_evidence(attempt_id)` | 6 — Browser Removal |
| `send_removal_email(attempt_id)` | 7 — Email Removal |
| `mark_attempt_verified(attempt_id)` | 8 — Email Verification |
| `get_scheduled_jobs(vault_id)` | 9 — Scheduler |
| `update_scheduled_job(job_id, interval, enabled)` | 9 — Scheduler |
| `run_job_now(job_type)` | 9 — Scheduler |
| `list_brokers()` | 10 — Broker Explorer |
| `get_broker_detail(broker_id, vault_id)` | 10 — Broker Explorer |
| `start_scan(vault_id, tier, broker_ids)` | 11 — Proactive Scanning |
| `start_discovery_scan(vault_id)` | 12 — Local Discovery |
| `get_discovery_findings(vault_id)` | 12 — Local Discovery |
| `mark_finding_remediated(finding_id)` | 12 — Local Discovery |

---

## New Tauri Events

| Event | Payload | Trigger |
|---|---|---|
| `removal:verified` | `{ attempt_id, broker_id }` | Manual or IMAP auto-verification |
| `scan:scheduled_complete` | `{ job_type, findings_count }` | Scheduler-triggered scan finishes |
| `discovery:complete` | `{ findings_count }` | Discovery scan finishes |

---

## Out of Scope (Phase 7+)

- Advanced CAPTCHA solving (vision LLM or 3rd party services like 2captcha)
- LLM-assisted email drafting / Chat interface
- Network monitoring telemetry (`spectral-netmon`) and dashboard network cards
- Multi-broker session reuse
- Mobile / iOS / Android support
- Community broker definition contributions
- Report export (PDF / Markdown / JSON)
- Plugin system

---

## Success Criteria

1. Users with multiple vaults can switch between them from the navigation bar
2. Vault management (rename, change password, delete, create) works from Settings
3. Settings page exists with Vaults, Privacy Level, Email, Scheduling, and Audit Log sections
4. Users can see all past removal jobs in a history page with per-broker detail
5. Users can view a privacy score with category breakdown and event timeline
6. Dashboard shows scan coverage, active removal counts, and recent activity feed
7. Brokers marked `BrowserForm` submit successfully via headless Chrome
8. Brokers marked `Email` send removal emails via `mailto:` or SMTP
9. Email verification can be handled manually or automatically via IMAP
10. Scans and verification checks run automatically on app open; tray mode enables background scheduling
11. Tray mode works on macOS, Windows, and supported Linux DEs; degrades gracefully elsewhere
12. Broker Explorer shows all brokers with filtering and per-broker detail pages
13. Proactive scan tiers auto-scan Tier 1 brokers for the user's region on first run
14. Local Discovery scan finds PII in filesystem, browser, and email headers; findings are actionable
