# Email Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Make Spectral act as a fully autonomous email agent for data removal: persist SMTP/IMAP/CC credentials in the vault, send removal request emails directly via SMTP (with user CC), fall back to `mailto:` when unconfigured, poll IMAP on a schedule to detect broker responses, and use the LLM to draft and send replies when brokers require follow-up.

**Architecture Summary:**
- Credentials live in the existing `settings` KV table (same pattern as LLM API keys — no new migration needed)
- A new `spectral-mail` settings module holds `get/set` helpers following the `spectral-privacy` pattern
- New Tauri commands `get_email_settings` / `save_email_settings` replace the current stateless UI
- `send_removal_email` command loads SMTP config from vault and sends directly (or falls back to `mailto:`)
- `RemovalActionButton` calls the Tauri command instead of building a `mailto:` URL itself
- The removal worker `submit_via_email` loads SMTP config from vault so batch removals also use SMTP
- The `PollImap` scheduler job is fully implemented: loads IMAP config, finds pending email-removal attempts, calls `poll_for_verifications`, marks matching ones complete
- LLM reply loop: broker responses that require action are detected, drafted via `draft_email`, and sent via SMTP

**Tech Stack:** Rust/SQLite/sqlx (`spectral-db` settings KV), Tauri commands (`src-tauri`), `spectral-mail` crate, Svelte 5 runes (frontend), Tailwind CSS

---

## Phase Tracking

| Phase | Title | Status |
|-------|-------|--------|
| Phase 1 | Credential Persistence — Save & Load Email Settings | ✅ Complete |
| Phase 2 | Settings UI — Wiring, Save Button, CC Field, Fixed Test | ✅ Complete |
| Phase 3 | SMTP Send Path — Use Real Email When Configured | ✅ Complete |
| Phase 4 | IMAP Polling Loop — Detect & Process Broker Responses | ✅ Complete |

> **Instructions for Claude:** Update the Status column above as you complete phases. Use: ⬜ Not Started → 🔄 In Progress → ✅ Complete → ❌ Blocked

---

## Architecture: Settings Keys

Email settings use the same KV pattern as LLM API keys, stored in the encrypted `settings` table:

| Key | Type | Description |
|-----|------|-------------|
| `email.smtp.enabled` | bool | Whether SMTP sending is active |
| `email.smtp.host` | String | SMTP server hostname |
| `email.smtp.port` | u16 | SMTP port (default 587) |
| `email.smtp.username` | String | SMTP login |
| `email.smtp.password` | String | SMTP password (vault-encrypted at rest) |
| `email.imap.enabled` | bool | Whether IMAP monitoring is active |
| `email.imap.host` | String | IMAP server hostname |
| `email.imap.port` | u16 | IMAP port (default 993) |
| `email.imap.username` | String | IMAP login |
| `email.imap.password` | String | IMAP password (vault-encrypted at rest) |
| `email.cc_address` | String | Address to CC on all outbound removal emails (empty = no CC) |

---

## File Map

### Modified Files

| File | Change |
|------|--------|
| `crates/spectral-mail/src/lib.rs` | Add `pub mod settings` |
| `crates/spectral-mail/src/settings.rs` | **NEW** — get/set helpers for email credentials |
| `crates/spectral-mail/src/sender.rs` | Add optional `cc` field to `send_smtp` |
| `src-tauri/src/commands/settings.rs` | Add `get_email_settings`, `save_email_settings`; implement `test_smtp_connection` |
| `src-tauri/src/commands/scan.rs` | Update `send_removal_email` to load SMTP from vault and send directly |
| `src-tauri/src/removal_worker.rs` | Load SMTP config in `submit_removal_task` and pass to `submit_via_email` |
| `src-tauri/src/commands/scheduler.rs` | Implement `PollImap` job |
| `src-tauri/src/lib.rs` | Register new commands |
| `src/lib/api/settings.ts` | Add `getEmailSettings`, `saveEmailSettings` TS wrappers |
| `src/routes/settings/+page.svelte` | Wire email tab: load/save, CC field, fix test buttons |
| `src/lib/components/removals/RemovalActionButton.svelte` | Email branch: call Tauri cmd instead of building mailto |

---

## Phase 1 — Credential Persistence

**Goal:** Implement the settings storage layer and Tauri commands. No UI changes yet.

### Task 1 — `spectral-mail` settings module

**File:** `crates/spectral-mail/src/settings.rs` (new) + `crates/spectral-mail/src/lib.rs` (add mod)

**Pattern to follow:** `crates/spectral-privacy/src/llm_settings.rs` — use `spectral_db::settings::{get_setting, set_setting}` with JSON values.

**Steps:**
- [x] Create `crates/spectral-mail/src/settings.rs`
- [x] Define `EmailSettings` struct:
  ```rust
  pub struct EmailSettings {
      pub smtp_enabled: bool,
      pub smtp_host: String,
      pub smtp_port: u16,
      pub smtp_username: String,
      pub smtp_password: String,   // plaintext in memory; encrypted at rest by vault pool
      pub imap_enabled: bool,
      pub imap_host: String,
      pub imap_port: u16,
      pub imap_username: String,
      pub imap_password: String,
      pub cc_address: String,       // empty string = no CC
  }
  ```
- [x] Implement `get_email_settings(pool: &SqlitePool) -> Result<EmailSettings>`
  - Read each key individually with fallback defaults (empty strings, ports 587/993, enabled = false)
- [x] Implement `set_email_settings(pool: &SqlitePool, settings: &EmailSettings) -> Result<()>`
  - Write each key individually (same pattern as `set_api_key`)
- [x] Implement `get_smtp_config(pool: &SqlitePool) -> Result<Option<SmtpConfig>>`
  - Returns `None` if `smtp_enabled = false` or host is empty
  - Returns `Some(SmtpConfig { host, port, username, password })` when configured
- [x] Implement `get_imap_config(pool: &SqlitePool) -> Result<Option<ImapConfig>>`
  - Returns `None` if `imap_enabled = false` or host is empty
- [x] Implement `get_cc_address(pool: &SqlitePool) -> Result<Option<String>>`
  - Returns `None` if cc_address is empty
- [x] Add `pub mod settings;` to `crates/spectral-mail/src/lib.rs` and re-export `EmailSettings`
- [x] Add `spectral-db` as a dependency in `crates/spectral-mail/Cargo.toml` if not already present
- [x] Write unit tests for get/set round-trip (use in-memory SQLite like other crate tests)
- [x] Run `cargo test -p spectral-mail` — must pass

**Passing criteria:** `cargo test -p spectral-mail` passes with new tests green.

---

### Task 2 — Tauri commands for email settings

**File:** `src-tauri/src/commands/settings.rs`

**Steps:**
- [x] Read the full current `settings.rs` file first
- [x] Define `EmailSettingsPayload` struct (mirrors `EmailSettings`, with `#[derive(serde::Serialize, serde::Deserialize)]`)
- [x] Add `get_email_settings` command:
  ```rust
  #[tauri::command]
  pub async fn get_email_settings(
      state: State<'_, AppState>,
      vault_id: String,
  ) -> Result<EmailSettingsPayload, CommandError>
  ```
  - Use `get_vault` + `get_db` helpers
  - Call `spectral_mail::settings::get_email_settings(pool)`
  - Map to payload (omit passwords from response — return empty strings, include `has_smtp_password: bool` and `has_imap_password: bool` flags instead)
- [x] Add `save_email_settings` command:
  ```rust
  #[tauri::command]
  pub async fn save_email_settings(
      state: State<'_, AppState>,
      vault_id: String,
      payload: EmailSettingsPayload,
  ) -> Result<(), CommandError>
  ```
  - If password field is empty string in payload, preserve existing password (don't overwrite)
  - Otherwise save new password
- [x] Implement `test_smtp_connection` (currently a stub — replace with real test):
  - Build `SmtpConfig` from params
  - Call `spectral_mail::sender::test_smtp(&config)` (add this function to sender.rs — just attempt connection without sending)
- [x] Register all three commands in `src-tauri/src/lib.rs`
- [x] Run `cargo build -p spectral-app` — must compile

**Passing criteria:** `cargo build -p spectral-app` succeeds.

---

### Phase 1 Checkpoint

Update Phase 1 status to ✅ in the Phase Tracking table above.
Commit: `feat(email): persist SMTP/IMAP/CC credentials in vault settings`

---

## Phase 2 — Settings UI

**Goal:** Wire the email settings tab to load/save from vault, add CC field, fix test buttons.

### Task 3 — TypeScript API wrappers

**File:** `src/lib/api/settings.ts`

**Steps:**
- [x] Read the current `src/lib/api/settings.ts`
- [x] Add `EmailSettings` interface matching the Rust payload (no password fields, has `has_smtp_password: bool`, `has_imap_password: bool`)
- [x] Add `getEmailSettings(vaultId: string): Promise<EmailSettings>` → invoke `get_email_settings`
- [x] Add `saveEmailSettings(vaultId: string, settings: Partial<EmailSettings> & { smtp_password?: string; imap_password?: string }): Promise<void>` → invoke `save_email_settings`
- [x] Verify TypeScript compiles: `npx tsc --noEmit`

**Passing criteria:** No TypeScript errors.

---

### Task 4 — Settings page email tab

**File:** `src/routes/settings/+page.svelte`

This is a large file (~1754 lines). Read it fully before editing.

**Changes needed:**
- [x] Import `getEmailSettings`, `saveEmailSettings` from `$lib/api/settings`
- [x] Add `ccAddress` state variable alongside existing SMTP/IMAP state
- [x] Add `hasSMTPPassword`, `hasIMAPPassword` state (from loaded settings — show "••••••••" placeholder)
- [x] Add `emailSaveResult` state: `'idle' | 'saving' | 'saved' | 'error'`
- [x] Add `$effect`: when email tab becomes active and vault is set, call `loadEmailSettings()`
- [x] Implement `loadEmailSettings()`: calls `getEmailSettings`, populates all SMTP/IMAP state, sets CC
- [x] Implement `handleSaveEmailSettings()`: calls `saveEmailSettings` with all fields; only includes password if user typed in the password field (non-empty)
- [x] In the email tab template:
  - Add "CC Address" input field below IMAP section with label "CC me on all removal emails"
  - Add "Save Email Settings" button with loading/saved/error states
  - Fix test SMTP button to use currently-loaded/entered values
  - Show `hasSMTPPassword` / `hasIMAPPassword` as placeholder "••••••••" when password is saved but not shown
- [x] Run `npx tsc --noEmit` and `npx prettier --write` — must pass

**Passing criteria:** TypeScript clean, prettier clean, UI loads and saves without errors.

---

### Phase 2 Checkpoint

Update Phase 2 status to ✅ in the Phase Tracking table above.
Commit: `feat(settings): wire email settings UI with save/load and CC field`

---

## Phase 3 — SMTP Send Path

**Goal:** When SMTP is configured, all outbound removal emails are sent by Spectral automatically. `RemovalActionButton` delegates to the Tauri `send_removal_email` command. The removal worker also uses SMTP.

### Task 5 — Add CC support to `send_smtp`

**File:** `crates/spectral-mail/src/sender.rs`

**Steps:**
- [x] Read current `sender.rs`
- [x] Add `cc: Option<String>` parameter to `send_smtp` (add to the Message builder if Some)
- [x] Update all callers of `send_smtp` (check `removal_worker.rs`) to pass `None` for CC for now (Task 6 will wire the real value)
- [x] `cargo test -p spectral-mail` must still pass

---

### Task 6 — `send_removal_email` uses SMTP + CC

**File:** `src-tauri/src/commands/scan.rs`

**Goal:** Update the `send_removal_email` command to:
1. Load email settings from vault
2. If SMTP enabled: send via SMTP (with CC if set), return success
3. If SMTP not enabled: fall back to opening `mailto:` via `app.shell().open()`

**Steps:**
- [x] Read the current `send_removal_email` command in `scan.rs`
- [x] After loading removal context and rendering the template, call `spectral_mail::settings::get_smtp_config(pool)` and `get_cc_address(pool)`
- [x] If `smtp_config` is Some: call `send_smtp(&email_template, &user_email, &config, cc.as_deref())`, return Ok
- [x] If `smtp_config` is None: build mailto URL and call `app.shell().open()` (existing behavior)
- [x] Update `email_removals` table insert: set `method = "smtp"` or `"mailto"` based on which path was used
- [x] `cargo build -p spectral-app` must succeed

---

### Task 7 — `RemovalActionButton` delegates to Tauri command

**File:** `src/lib/components/removals/RemovalActionButton.svelte`

Currently the email branch builds a `mailto:` URL on the frontend. Replace with a call to the Tauri `send_removal_email` command, which handles SMTP-vs-mailto internally.

**Steps:**
- [x] Read current `RemovalActionButton.svelte`
- [x] Import `invoke` from `@tauri-apps/api/core` (or use `removalAPI` — add a new `sendRemovalEmail(vaultId, attemptId)` wrapper in `removal.ts`)
- [x] The component needs a `removalAttemptId` prop — this is needed to call the command. Add `removalAttemptId?: string` prop.
- [x] For email-method brokers: replace the `<a href={buildMailtoUrl()}>` with a button that calls `invoke('send_removal_email', { vaultId, attemptId: removalAttemptId })` (with loading/submitted states)
- [x] Keep the substitution logic (`substituteTemplateVars`) as a fallback for when no `removalAttemptId` is provided (render mailto: link in that case)
- [x] Update the adtech detail page to pass a `removalAttemptId` if one exists (it won't for pre-removal state; that's fine — the fallback handles it)
- [x] `npx tsc --noEmit` must pass

---

### Task 8 — Removal worker loads SMTP config

**File:** `src-tauri/src/removal_worker.rs`

The `submit_removal_task` function calls `submit_via_email(..., smtp_config: None, ...)`. Fix this.

**Steps:**
- [x] Read the `submit_removal_task` function in `removal_worker.rs`
- [x] After loading the database, call `spectral_mail::settings::get_smtp_config(db.pool()).await` (make it async or use `spawn_blocking` if the function is sync)
- [x] Also load CC address: `get_cc_address(db.pool()).await`
- [x] Pass both to `submit_via_email` (update its signature to accept `cc: Option<&str>`)
- [x] `cargo build -p spectral-app` must succeed
- [x] `cargo test` must pass

---

### Phase 3 Checkpoint

Update Phase 3 status to ✅ in the Phase Tracking table above.
Commit: `feat(email): use SMTP for automatic removal email sending when configured`

---

## Phase 4 — IMAP Polling Loop

**Goal:** The `PollImap` scheduled job becomes fully functional: it loads IMAP credentials, finds pending email removal attempts, polls the inbox, and marks confirmed ones complete. If a broker response requires a reply (and LLM is configured), it drafts and sends one.

### Task 9 — Implement `PollImap` scheduler job

**File:** `src-tauri/src/commands/scheduler.rs`

**Steps:**
- [x] Read the current `scheduler.rs` fully
- [x] In the `PollImap` match arm, replace the stub with:
  1. Load IMAP config: `spectral_mail::settings::get_imap_config(db.pool()).await` — if None, return early with a helpful message
  2. Query `email_removals` table for all rows where the linked `removal_attempt` has status `Submitted` — collect `(broker_email, attempt_id)` pairs into a `HashMap<String, String>`
  3. If map is empty, return Ok (nothing to check)
  4. Spawn blocking task: `tokio::task::spawn_blocking(move || poll_for_verifications(&config, &broker_map))`
  5. For each `(broker_email, attempt_id)` in `result.verified`:
     - Update `removal_attempts` status to `Completed` where id = attempt_id
     - Set `completed_at = now`
     - Emit Tauri event `removal:verified` with `{ removal_attempt_id }`
  6. Log any `result.errors` as warnings
- [x] `cargo build -p spectral-app` must succeed

---

### Task 10 — LLM reply detection

**File:** `src-tauri/src/commands/scheduler.rs` (continued) or a new helper in `removal_worker.rs`

Some broker responses require action (e.g. "please confirm your request by replying"). Detect and handle these.

**Steps:**
- [x] Extend the IMAP polling to also fetch message bodies (not just headers) for matched messages
  - Fetch `RFC822` instead of `RFC822.HEADER` for verified messages only
  - Parse the text body from the message
- [x] After marking an attempt as verified, check if LLM is configured (`spectral_privacy::get_primary_provider` or check if any provider is set)
- [x] If LLM available: call the `draft_email` logic to analyze the response body and determine if a reply is needed
  - Prompt: "You received the following email from a data broker in response to a removal request. Does it require a reply? If yes, draft a short professional reply confirming the request. If no, just say NO_REPLY_NEEDED."
  - Parse response: if starts with `NO_REPLY_NEEDED`, skip; otherwise use the drafted reply as body
- [x] If reply needed and SMTP configured: send reply via `send_smtp` (To = broker email, from = user email, body = drafted reply)
- [x] Log reply to `email_removals` table with method `smtp_reply`
- [x] `cargo build -p spectral-app` must succeed

---

### Phase 4 Checkpoint

Update Phase 4 status to ✅ in the Phase Tracking table above.
Commit: `feat(email): IMAP polling loop with LLM-assisted reply handling`

---

## Completion Criteria

- [x] SMTP/IMAP/CC credentials are saved in the vault and survive app restart
- [x] "Test SMTP" button actually attempts a connection
- [x] Removal emails are sent via SMTP when configured (no mail client popup)
- [x] User receives a CC copy of every email sent on their behalf
- [x] `mailto:` link still works as fallback when SMTP is not configured
- [x] `PollImap` scheduled job runs and marks broker confirmations as complete
- [x] If a broker reply requires action and LLM is configured, a reply is sent automatically
- [x] All Rust tests pass, TypeScript compiles clean

---

## Key Architecture Notes

**Why KV table and not a dedicated table?**
The `settings` table already holds LLM API keys using exactly this pattern. Keeping email credentials in the same table means they get the same vault encryption, backup behavior, and access control automatically. No migration needed.

**Password handling:**
- `get_email_settings` response omits password values — returns `has_smtp_password: bool` and `has_imap_password: bool` flags instead (same as `has_openai_key` pattern)
- `save_email_settings` only overwrites a password if the field is non-empty in the payload — preserves existing credential if user didn't retype it

**CC behavior:**
- Empty string = no CC (default)
- CC is added to every email sent via SMTP on the user's behalf
- CC address is stored in settings, not per-email

**IMAP reply detection:**
- Only triggered when LLM is configured AND SMTP is configured (need both to close the loop)
- Conservative prompt — prefers `NO_REPLY_NEEDED` to avoid spam
- All sent replies logged to `email_removals` for auditability
