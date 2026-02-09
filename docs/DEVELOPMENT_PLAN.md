# Spectral Development Plan

> **Version:** 1.0.0
> **Last Updated:** 2026-02-09
> **Total Duration:** ~24 weeks across 4 phases

This document breaks down the Spectral development roadmap into manageable tasks. Each task is designed to fit within a single AI context window and includes tracking, testing, and commit instructions.

---

## Task Tracking Legend

| Status | Meaning |
|--------|---------|
| `[ ]` | Not started |
| `[~]` | In progress |
| `[x]` | Completed |
| `[!]` | Blocked |

---

## Phase 1: Foundation (v0.1) — ~8 weeks

### Overview
Build the core infrastructure: encrypted vault, Tauri shell, LLM abstraction, and initial broker definitions. By the end of Phase 1, users can unlock the app, configure their profile, manually scan 5 brokers, and view results.

---

### Task 1.1: Core Crate Setup
**Status:** `[ ]`
**Estimated Scope:** Create spectral-core with shared types, errors, and configuration

#### Objective
Establish the foundation crate that all other crates depend on. This includes common error types, configuration structures, and shared utilities.

#### Implementation Steps

1. **Create spectral-core crate structure**
   ```
   crates/spectral-core/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── error.rs
       ├── config.rs
       ├── types.rs
       └── capabilities.rs
   ```

2. **Implement core error types** (`src/error.rs`)
   - `SpectralError` enum with variants for each subsystem
   - Implement `std::error::Error` and `Display`
   - Use `thiserror` for derivation

3. **Implement configuration** (`src/config.rs`)
   - `AppConfig` struct with sections for each module
   - TOML-based config file loading
   - XDG-compliant paths via `directories` crate
   - Environment variable overrides

4. **Implement shared types** (`src/types.rs`)
   - `ProfileId`, `BrokerId` newtypes with validation
   - `PiiField` enum for PII categories
   - `Timestamp` wrapper around `chrono::DateTime<Utc>`

5. **Implement capability registry stub** (`src/capabilities.rs`)
   - `CapabilityRegistry` struct (placeholder for Phase 1)
   - `FeatureId` enum with all feature identifiers
   - `is_feature_available()` method (returns false for LLM features initially)

#### Acceptance Criteria
- [ ] `cargo build -p spectral-core` succeeds
- [ ] `cargo test -p spectral-core` passes
- [ ] `cargo clippy -p spectral-core -- -D warnings` clean
- [ ] Config loads from `~/.config/spectral/config.toml`
- [ ] Error types implement proper Display formatting

#### Test Commands
```bash
cargo test -p spectral-core
cargo clippy -p spectral-core -- -D warnings
```

#### Commit Instructions
```bash
git add crates/spectral-core/
git commit -m "feat(core): add spectral-core crate with shared types, errors, and config

- Add SpectralError enum with thiserror derivation
- Implement AppConfig with TOML loading and XDG paths
- Add ProfileId, BrokerId newtypes with validation
- Stub CapabilityRegistry for LLM-optional architecture

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.2: Database Layer**

---

### Task 1.2: Database Layer
**Status:** `[ ]`
**Estimated Scope:** Implement spectral-db with SQLCipher integration

#### Objective
Create the database abstraction layer using SQLCipher for at-rest encryption. This provides the foundation for all persistent storage.

#### Prerequisites
- Task 1.1 (spectral-core) completed

#### Implementation Steps

1. **Create spectral-db crate structure**
   ```
   crates/spectral-db/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── connection.rs
       ├── migrations.rs
       └── error.rs
   ```

2. **Add dependencies to Cargo.toml**
   ```toml
   [dependencies]
   spectral-core = { path = "../spectral-core" }
   sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
   sqlcipher = "0.1"  # or rusqlite with bundled-sqlcipher
   thiserror = "1.0"
   tokio = { version = "1", features = ["sync"] }
   ```

3. **Implement encrypted connection** (`src/connection.rs`)
   - `EncryptedPool` wrapper around sqlx pool
   - Key derivation integration point (accepts derived key)
   - Connection string with SQLCipher pragmas
   - Auto-lock on drop via `Zeroizing`

4. **Implement migrations** (`src/migrations.rs`)
   - Embed migrations using `sqlx::migrate!`
   - Create initial schema (profiles, broker_results, audit_log)
   - Migration version tracking table

5. **Create initial migration** (`migrations/001_initial.sql`)
   ```sql
   CREATE TABLE profiles (
       id TEXT PRIMARY KEY,
       data BLOB NOT NULL,
       nonce BLOB NOT NULL,
       created_at TEXT NOT NULL,
       updated_at TEXT NOT NULL
   );

   CREATE TABLE broker_results (
       id TEXT PRIMARY KEY,
       profile_id TEXT NOT NULL REFERENCES profiles(id),
       broker_id TEXT NOT NULL,
       status TEXT NOT NULL,
       found_data_hash TEXT,
       first_seen TEXT NOT NULL,
       last_checked TEXT NOT NULL,
       removal_requested_at TEXT,
       removal_confirmed_at TEXT
   );

   CREATE TABLE audit_log (
       id TEXT PRIMARY KEY,
       timestamp TEXT NOT NULL,
       event_type TEXT NOT NULL,
       detail TEXT,
       source TEXT NOT NULL
   );
   ```

#### Acceptance Criteria
- [ ] `cargo build -p spectral-db` succeeds
- [ ] Database creates encrypted file on disk
- [ ] Migrations run successfully on first connection
- [ ] Connection fails with wrong key
- [ ] Basic CRUD operations work

#### Test Commands
```bash
cargo test -p spectral-db
cargo clippy -p spectral-db -- -D warnings
```

#### Commit Instructions
```bash
git add crates/spectral-db/
git commit -m "feat(db): add spectral-db with SQLCipher encryption

- Implement EncryptedPool with SQLCipher integration
- Add migration system with initial schema
- Create profiles, broker_results, audit_log tables
- Ensure key zeroization on connection drop

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.3: Encrypted Vault**

---

### Task 1.3: Encrypted Vault
**Status:** `[ ]`
**Estimated Scope:** Implement spectral-vault with Argon2id KDF and ChaCha20-Poly1305

#### Objective
Build the vault that manages the master password, key derivation, and field-level encryption for PII data.

#### Prerequisites
- Task 1.1 (spectral-core) completed
- Task 1.2 (spectral-db) completed

#### Implementation Steps

1. **Create spectral-vault crate structure**
   ```
   crates/spectral-vault/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── kdf.rs
       ├── cipher.rs
       ├── profile.rs
       ├── locked.rs
       └── error.rs
   ```

2. **Implement KDF** (`src/kdf.rs`)
   - Argon2id with recommended parameters (19MB memory, 2 iterations)
   - Salt generation and storage
   - `derive_key(password: &str, salt: &[u8]) -> Zeroizing<[u8; 32]>`

3. **Implement field cipher** (`src/cipher.rs`)
   - `EncryptedField<T>` struct with ciphertext and nonce
   - ChaCha20-Poly1305 encryption/decryption
   - Serialize/deserialize via serde

4. **Implement UserProfile** (`src/profile.rs`)
   - All PII fields as `EncryptedField<T>`
   - CRUD operations via spectral-db
   - Decryption only when vault is unlocked

5. **Implement vault lifecycle** (`src/lib.rs`)
   - `Vault::create(password)` - first-time setup
   - `Vault::unlock(password)` - derive key, verify, open db
   - `Vault::lock()` - zeroize key, close connections
   - Auto-lock timer (configurable)

#### Acceptance Criteria
- [ ] `cargo build -p spectral-vault` succeeds
- [ ] New vault creation with master password works
- [ ] Unlock with correct password succeeds
- [ ] Unlock with wrong password fails cleanly
- [ ] Profile encryption/decryption round-trips correctly
- [ ] Key is zeroized from memory after lock

#### Test Commands
```bash
cargo test -p spectral-vault
cargo clippy -p spectral-vault -- -D warnings
# Verify zeroization (check with valgrind in CI)
```

#### Commit Instructions
```bash
git add crates/spectral-vault/
git commit -m "feat(vault): implement encrypted vault with Argon2id and ChaCha20-Poly1305

- Add Argon2id KDF with secure parameters (19MB, 2 iterations)
- Implement EncryptedField<T> for field-level PII encryption
- Create UserProfile with all encrypted PII fields
- Add vault lifecycle: create, unlock, lock with auto-lock timer
- Ensure key zeroization via zeroize crate

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.4: Tauri Commands - Vault**

---

### Task 1.4: Tauri Commands - Vault
**Status:** `[ ]`
**Estimated Scope:** Expose vault operations to frontend via Tauri commands

#### Objective
Create the IPC layer between the Rust backend and SvelteKit frontend for vault operations.

#### Prerequisites
- Task 1.3 (spectral-vault) completed

#### Implementation Steps

1. **Create commands module structure**
   ```
   src-tauri/src/
   ├── lib.rs
   ├── state.rs
   └── commands/
       ├── mod.rs
       └── vault.rs
   ```

2. **Implement app state** (`src/state.rs`)
   - `AppState` struct with `RwLock<Option<Vault>>`
   - State initialization in Tauri setup

3. **Implement vault commands** (`src/commands/vault.rs`)
   ```rust
   #[tauri::command]
   async fn vault_create(password: String, state: State<'_, AppState>) -> Result<(), CommandError>;

   #[tauri::command]
   async fn vault_unlock(password: String, state: State<'_, AppState>) -> Result<(), CommandError>;

   #[tauri::command]
   async fn vault_lock(state: State<'_, AppState>) -> Result<(), CommandError>;

   #[tauri::command]
   async fn vault_status(state: State<'_, AppState>) -> Result<VaultStatus, CommandError>;
   ```

4. **Implement CommandError** type
   - Serializable error type for frontend
   - Convert from internal errors
   - User-friendly messages

5. **Register commands in lib.rs**
   - Add to `invoke_handler`
   - Manage app state

#### Acceptance Criteria
- [ ] Frontend can create new vault
- [ ] Frontend can unlock/lock vault
- [ ] Frontend can query vault status
- [ ] Errors are properly serialized to frontend
- [ ] State persists across command calls

#### Test Commands
```bash
cargo build -p spectral-app
cargo test -p spectral-app
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add src-tauri/
git commit -m "feat(tauri): add vault IPC commands

- Implement vault_create, vault_unlock, vault_lock, vault_status commands
- Add AppState with RwLock<Vault> for shared state
- Create CommandError type for frontend-friendly errors
- Register commands in Tauri invoke handler

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.5: Unlock Screen UI**

---

### Task 1.5: Unlock Screen UI
**Status:** `[ ]`
**Estimated Scope:** Create the unlock/create vault screen in SvelteKit

#### Objective
Build the first screen users see - either creating a new vault or unlocking an existing one.

#### Prerequisites
- Task 1.4 (Tauri vault commands) completed

#### Implementation Steps

1. **Create unlock route and components**
   ```
   src/
   ├── routes/
   │   ├── +layout.svelte
   │   ├── +page.svelte        # Redirects based on vault status
   │   └── unlock/
   │       └── +page.svelte
   ├── lib/
   │   ├── api/
   │   │   └── vault.ts        # Tauri invoke wrappers
   │   ├── stores/
   │   │   └── vault.ts        # Vault state store
   │   └── components/
   │       └── unlock/
   │           ├── UnlockForm.svelte
   │           ├── CreateVaultForm.svelte
   │           └── PasswordStrength.svelte
   ```

2. **Implement vault API wrapper** (`src/lib/api/vault.ts`)
   ```typescript
   import { invoke } from '@tauri-apps/api/core';

   export async function createVault(password: string): Promise<void>;
   export async function unlockVault(password: string): Promise<void>;
   export async function lockVault(): Promise<void>;
   export async function getVaultStatus(): Promise<VaultStatus>;
   ```

3. **Implement vault store** (`src/lib/stores/vault.ts`)
   - Svelte 5 runes: `$state`, `$derived`
   - Track locked/unlocked status
   - Auto-check status on app load

4. **Create UnlockForm component**
   - Password input with show/hide toggle
   - Submit button with loading state
   - Error display for wrong password

5. **Create CreateVaultForm component**
   - Password input with confirmation
   - Password strength indicator
   - Minimum requirements validation

6. **Implement routing logic**
   - Check vault existence on load
   - Redirect to /unlock if locked
   - Redirect to /dashboard if unlocked

#### Acceptance Criteria
- [ ] New users see vault creation screen
- [ ] Existing users see unlock screen
- [ ] Password validation works
- [ ] Error messages display correctly
- [ ] Successful unlock redirects to dashboard

#### Test Commands
```bash
npm run check
npm run lint
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add src/
git commit -m "feat(ui): add unlock and vault creation screens

- Create vault API wrapper with typed invoke calls
- Implement vault store with Svelte 5 runes
- Add UnlockForm with password visibility toggle
- Add CreateVaultForm with strength indicator
- Implement routing logic for vault state

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.6: Profile Setup UI**

---

### Task 1.6: Profile Setup UI
**Status:** `[ ]`
**Estimated Scope:** Create the profile setup wizard for entering PII

#### Objective
Build the wizard that collects user PII (name, email, addresses, etc.) after vault creation.

#### Prerequisites
- Task 1.5 (Unlock Screen) completed

#### Implementation Steps

1. **Add profile Tauri commands** (`src-tauri/src/commands/profile.rs`)
   ```rust
   #[tauri::command]
   async fn profile_create(profile: ProfileInput) -> Result<ProfileId, CommandError>;

   #[tauri::command]
   async fn profile_get() -> Result<Option<Profile>, CommandError>;

   #[tauri::command]
   async fn profile_update(updates: ProfileUpdate) -> Result<(), CommandError>;
   ```

2. **Create profile API wrapper** (`src/lib/api/profile.ts`)
   - Type definitions matching Rust structs
   - Create, get, update functions

3. **Create wizard components**
   ```
   src/lib/components/profile/
   ├── ProfileWizard.svelte      # Multi-step container
   ├── steps/
   │   ├── BasicInfo.svelte      # Name, DOB
   │   ├── ContactInfo.svelte    # Email, phone
   │   ├── Addresses.svelte      # Physical addresses
   │   ├── Aliases.svelte        # Previous names, nicknames
   │   └── Review.svelte         # Summary before save
   └── common/
       ├── AddressInput.svelte
       └── PhoneInput.svelte
   ```

4. **Implement wizard state management**
   - Step tracking with progress indicator
   - Form validation per step
   - Back/next navigation
   - Final submission to backend

5. **Create profile route** (`src/routes/profile/setup/+page.svelte`)
   - Render ProfileWizard
   - Redirect to dashboard on completion

#### Acceptance Criteria
- [ ] Wizard progresses through all steps
- [ ] Form validation prevents bad data
- [ ] Back button preserves entered data
- [ ] Profile saves successfully to vault
- [ ] Completion redirects to dashboard

#### Test Commands
```bash
npm run check
npm run lint
cargo test -p spectral-app
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add src-tauri/src/commands/profile.rs src/
git commit -m "feat(ui): add profile setup wizard

- Add profile_create, profile_get, profile_update commands
- Create multi-step ProfileWizard component
- Implement BasicInfo, ContactInfo, Addresses, Aliases steps
- Add form validation and progress tracking
- Save encrypted profile to vault on completion

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.7: Broker Definitions Crate**

---

### Task 1.7: Broker Definitions Crate
**Status:** `[ ]`
**Estimated Scope:** Create spectral-broker with TOML definition loading

#### Objective
Build the broker engine that loads and validates broker definition files.

#### Prerequisites
- Task 1.1 (spectral-core) completed

#### Implementation Steps

1. **Create spectral-broker crate structure**
   ```
   crates/spectral-broker/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── definition.rs
       ├── loader.rs
       ├── registry.rs
       └── error.rs
   ```

2. **Define broker types** (`src/definition.rs`)
   ```rust
   pub struct BrokerDefinition {
       pub id: BrokerId,
       pub name: String,
       pub url: String,
       pub category: BrokerCategory,
       pub search_method: SearchMethod,
       pub removal_method: RemovalMethod,
       pub difficulty: RemovalDifficulty,
       pub typical_removal_days: u32,
       pub recheck_interval_days: u32,
       pub last_verified: DateTime<Utc>,
   }
   ```

3. **Implement TOML loader** (`src/loader.rs`)
   - Load from `broker-definitions/` directory
   - Validate required fields
   - Parse search/removal methods

4. **Implement broker registry** (`src/registry.rs`)
   - In-memory cache of loaded definitions
   - Query by ID, category, difficulty
   - Hot-reload support (watch for file changes)

5. **Create initial broker definitions**
   ```
   broker-definitions/
   ├── people-search/
   │   ├── spokeo.toml
   │   ├── beenverified.toml
   │   ├── whitepages.toml
   │   ├── fastpeoplesearch.toml
   │   └── truepeoplesearch.toml
   └── README.md
   ```

#### Acceptance Criteria
- [ ] TOML files parse without errors
- [ ] All 5 initial brokers load successfully
- [ ] Registry queries work correctly
- [ ] Invalid definitions produce clear errors
- [ ] Definition schema is documented

#### Test Commands
```bash
cargo test -p spectral-broker
cargo clippy -p spectral-broker -- -D warnings
```

#### Commit Instructions
```bash
git add crates/spectral-broker/ broker-definitions/
git commit -m "feat(broker): add broker definition system

- Create BrokerDefinition with search/removal method types
- Implement TOML loader with validation
- Add broker registry with query methods
- Create initial 5 broker definitions (Spokeo, BeenVerified, etc.)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.8: LLM Abstraction Layer**

---

### Task 1.8: LLM Abstraction Layer
**Status:** `[ ]`
**Estimated Scope:** Create spectral-llm with provider trait and routing

#### Objective
Build the LLM abstraction that supports multiple backends with privacy-aware routing.

#### Prerequisites
- Task 1.1 (spectral-core) completed

#### Implementation Steps

1. **Create spectral-llm crate structure**
   ```
   crates/spectral-llm/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── provider.rs
       ├── router.rs
       ├── pii_filter.rs
       ├── providers/
       │   ├── mod.rs
       │   ├── anthropic.rs
       │   └── ollama.rs
       └── error.rs
   ```

2. **Define provider trait** (`src/provider.rs`)
   ```rust
   #[async_trait]
   pub trait LlmProvider: Send + Sync {
       async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
       async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream>;
       fn capabilities(&self) -> ProviderCapabilities;
       fn provider_id(&self) -> &str;
   }
   ```

3. **Implement PII filter** (`src/pii_filter.rs`)
   - Regex patterns for common PII (email, phone, SSN, etc.)
   - Redaction strategy (replace with [REDACTED_TYPE])
   - Tokenization strategy (reversible placeholders)

4. **Implement router** (`src/router.rs`)
   - Route based on `RoutingPreference`
   - Apply PII filter before sending to cloud
   - Fallback handling

5. **Implement Anthropic provider** (`src/providers/anthropic.rs`)
   - Claude API integration
   - Tool use support
   - Rate limiting

6. **Implement Ollama provider** (`src/providers/ollama.rs`)
   - Local Ollama API
   - Model listing
   - Streaming support

#### Acceptance Criteria
- [ ] Provider trait is extensible
- [ ] PII filter catches common patterns
- [ ] Anthropic provider connects successfully
- [ ] Ollama provider works with local instance
- [ ] Router respects routing preferences

#### Test Commands
```bash
cargo test -p spectral-llm
cargo clippy -p spectral-llm -- -D warnings
# Integration tests require API keys / local Ollama
```

#### Commit Instructions
```bash
git add crates/spectral-llm/
git commit -m "feat(llm): add LLM abstraction with privacy-aware routing

- Define LlmProvider trait with complete/stream methods
- Implement PII filter with regex patterns and redaction
- Add LlmRouter with local-preferred routing
- Implement Anthropic and Ollama providers
- Support tool use and streaming

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.9: Manual Scan Engine**

---

### Task 1.9: Manual Scan Engine
**Status:** `[ ]`
**Estimated Scope:** Implement basic URL-template scanning without browser automation

#### Objective
Create the scanning engine for brokers that use simple URL templates (no JavaScript required).

#### Prerequisites
- Task 1.3 (spectral-vault) completed
- Task 1.7 (spectral-broker) completed

#### Implementation Steps

1. **Extend spectral-broker with scan engine**
   ```
   crates/spectral-broker/src/
   ├── scanner/
   │   ├── mod.rs
   │   ├── url_template.rs
   │   └── result.rs
   ```

2. **Implement URL template scanner** (`src/scanner/url_template.rs`)
   - Variable substitution from profile
   - HTTP GET with proper headers
   - HTML response parsing
   - Result detection (found/not found)

3. **Define scan result types** (`src/scanner/result.rs`)
   ```rust
   pub struct ScanResult {
       pub broker_id: BrokerId,
       pub found: bool,
       pub listing_url: Option<String>,
       pub confidence: f32,
       pub scanned_at: DateTime<Utc>,
   }
   ```

4. **Implement result storage**
   - Save to `broker_results` table
   - Track scan history

5. **Add scan Tauri commands** (`src-tauri/src/commands/scan.rs`)
   ```rust
   #[tauri::command]
   async fn scan_broker(broker_id: String) -> Result<ScanResult, CommandError>;

   #[tauri::command]
   async fn scan_all() -> Result<Vec<ScanResult>, CommandError>;

   #[tauri::command]
   async fn get_scan_results() -> Result<Vec<ScanResult>, CommandError>;
   ```

#### Acceptance Criteria
- [ ] URL template substitution works
- [ ] HTTP requests include proper headers
- [ ] Found/not-found detection works for test brokers
- [ ] Results persist to database
- [ ] Frontend can trigger scans

#### Test Commands
```bash
cargo test -p spectral-broker
cargo test -p spectral-app
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-broker/ src-tauri/
git commit -m "feat(scan): implement URL template scanning engine

- Add URL template variable substitution
- Implement HTTP-based broker scanning
- Create ScanResult type with confidence scoring
- Persist results to broker_results table
- Add scan_broker, scan_all Tauri commands

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.10: Scan Results UI**

---

### Task 1.10: Scan Results UI
**Status:** `[ ]`
**Estimated Scope:** Display scan results in the dashboard

#### Objective
Build the UI for viewing scan results with status indicators.

#### Prerequisites
- Task 1.9 (Manual Scan Engine) completed

#### Implementation Steps

1. **Create scan API wrapper** (`src/lib/api/scan.ts`)
   ```typescript
   export async function scanBroker(brokerId: string): Promise<ScanResult>;
   export async function scanAll(): Promise<ScanResult[]>;
   export async function getScanResults(): Promise<ScanResult[]>;
   ```

2. **Create scan store** (`src/lib/stores/scan.ts`)
   - Track active scans
   - Cache results
   - Polling for updates

3. **Create dashboard components**
   ```
   src/lib/components/dashboard/
   ├── ScanDashboard.svelte
   ├── BrokerCard.svelte
   ├── ScanProgress.svelte
   └── ResultsTable.svelte
   ```

4. **Implement ScanDashboard**
   - Grid of broker cards
   - Scan all button
   - Progress indicators during scan

5. **Implement BrokerCard**
   - Broker name and category
   - Status badge (not scanned, found, not found, error)
   - Last scanned timestamp
   - Individual scan button

6. **Create dashboard route** (`src/routes/dashboard/+page.svelte`)

#### Acceptance Criteria
- [ ] Dashboard shows all configured brokers
- [ ] Scan all triggers scanning
- [ ] Progress updates in real-time
- [ ] Results display correctly
- [ ] Status badges are color-coded

#### Test Commands
```bash
npm run check
npm run lint
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add src/
git commit -m "feat(ui): add scan results dashboard

- Create scan API wrapper and store
- Implement ScanDashboard with broker grid
- Add BrokerCard with status badges
- Show progress indicators during scanning
- Display last scanned timestamps

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.11: Permission System**

---

### Task 1.11: Permission System
**Status:** `[ ]`
**Estimated Scope:** Implement granular permissions with first-run wizard

#### Objective
Create the permission system that controls what actions Spectral can take automatically.

#### Prerequisites
- Task 1.1 (spectral-core) completed

#### Implementation Steps

1. **Create spectral-permissions crate**
   ```
   crates/spectral-permissions/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── manager.rs
       ├── presets.rs
       ├── prompts.rs
       └── audit.rs
   ```

2. **Define permission types** (`src/lib.rs`)
   ```rust
   pub enum Permission {
       // Network
       ScanBrokers,
       SubmitRemovalForms,
       SendEmails,

       // LLM
       UseLlmCloud,
       UseLlmLocal,
       LlmGuidedBrowsing,

       // Local
       ScanFilesystem,
       ScanBrowserData,
       ScanEmails,

       // Automation
       AutoScheduleScans,
       AutoSubmitRemovals,
   }
   ```

3. **Implement presets** (`src/presets.rs`)
   - `Minimal`: manual everything
   - `Balanced`: auto-scan, manual removal
   - `Maximum`: full automation

4. **Implement permission manager** (`src/manager.rs`)
   - Check permission before action
   - Prompt user if not set
   - Persist decisions

5. **Add Tauri commands and first-run wizard UI**
   - Permission wizard steps
   - Explain each permission
   - Save preferences

#### Acceptance Criteria
- [ ] All permissions have clear descriptions
- [ ] First-run wizard covers all permissions
- [ ] Permissions persist across restarts
- [ ] Permission checks block unauthorized actions
- [ ] Presets apply correctly

#### Test Commands
```bash
cargo test -p spectral-permissions
npm run check
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-permissions/ src-tauri/ src/
git commit -m "feat(permissions): add granular permission system

- Define Permission enum covering all sensitive actions
- Create Minimal, Balanced, Maximum presets
- Implement PermissionManager with persistence
- Add first-run wizard UI for permission setup
- Ensure permission checks gate all automated actions

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.12: Command Palette**

---

### Task 1.12: Command Palette
**Status:** `[ ]`
**Estimated Scope:** Create keyboard-accessible command interface (LLM fallback)

#### Objective
Build the command palette that allows users to perform actions without LLM when it's unavailable.

#### Prerequisites
- Task 1.10 (Scan Results UI) completed

#### Implementation Steps

1. **Create command palette components**
   ```
   src/lib/components/command/
   ├── CommandPalette.svelte
   ├── CommandInput.svelte
   ├── CommandList.svelte
   └── CommandItem.svelte
   ```

2. **Define command registry** (`src/lib/commands/registry.ts`)
   ```typescript
   interface Command {
       id: string;
       name: string;
       shortcut?: string;
       keywords: string[];
       action: () => void | Promise<void>;
   }
   ```

3. **Implement commands**
   - `scan:all` - Scan all brokers
   - `scan:<broker>` - Scan specific broker
   - `profile:edit` - Open profile editor
   - `vault:lock` - Lock the vault
   - `settings:open` - Open settings

4. **Add keyboard shortcuts**
   - `Cmd/Ctrl+K` - Open palette
   - `Escape` - Close palette
   - Arrow keys - Navigate
   - Enter - Execute

5. **Integrate with layout**
   - Global keyboard listener
   - Modal overlay
   - Fuzzy search filtering

#### Acceptance Criteria
- [ ] Cmd/Ctrl+K opens palette
- [ ] Fuzzy search filters commands
- [ ] Keyboard navigation works
- [ ] Commands execute correctly
- [ ] Palette closes after execution

#### Test Commands
```bash
npm run check
npm run lint
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add src/
git commit -m "feat(ui): add command palette for keyboard-driven access

- Create CommandPalette with fuzzy search
- Implement command registry with actions
- Add scan, profile, vault, settings commands
- Enable Cmd/Ctrl+K global shortcut
- Support full keyboard navigation

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.13: Chat Interface (Basic)**

---

### Task 1.13: Chat Interface (Basic)
**Status:** `[ ]`
**Estimated Scope:** Create chat UI for status queries with LLM integration

#### Objective
Build the conversational interface that allows natural language interactions with Spectral.

#### Prerequisites
- Task 1.8 (LLM Abstraction Layer) completed
- Task 1.10 (Scan Results UI) completed

#### Implementation Steps

1. **Create spectral-chat crate**
   ```
   crates/spectral-chat/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── engine.rs
       ├── tools.rs
       └── prompts.rs
   ```

2. **Define chat tools** (`src/tools.rs`)
   ```rust
   pub enum ChatTool {
       GetStatus { broker_id: Option<String> },
       ScanBroker { broker_id: String },
       ScanAll,
       ExplainBroker { broker_id: String },
       GetTimeline,
   }
   ```

3. **Implement conversation engine** (`src/engine.rs`)
   - System prompt with tool descriptions
   - Parse tool calls from LLM response
   - Execute tools and format results

4. **Create chat UI components**
   ```
   src/lib/components/chat/
   ├── ChatContainer.svelte
   ├── MessageList.svelte
   ├── Message.svelte
   ├── ChatInput.svelte
   └── ToolResult.svelte
   ```

5. **Add Tauri commands** (`src-tauri/src/commands/chat.rs`)
   ```rust
   #[tauri::command]
   async fn chat_send(message: String) -> Result<ChatResponse, CommandError>;
   ```

#### Acceptance Criteria
- [ ] Chat input sends messages to backend
- [ ] LLM responds with natural language
- [ ] Tool calls are executed automatically
- [ ] Results are formatted nicely
- [ ] Graceful degradation without LLM configured

#### Test Commands
```bash
cargo test -p spectral-chat
npm run check
npm run tauri dev  # Manual testing (requires LLM config)
```

#### Commit Instructions
```bash
git add crates/spectral-chat/ src-tauri/ src/
git commit -m "feat(chat): add conversational interface with tool use

- Create spectral-chat crate with conversation engine
- Define ChatTool enum for available actions
- Implement tool calling and result formatting
- Add chat UI with message display
- Graceful fallback when LLM unavailable

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.14: Settings UI**

---

### Task 1.14: Settings UI
**Status:** `[ ]`
**Estimated Scope:** Create settings page for LLM configuration and preferences

#### Objective
Build the settings interface for configuring LLM providers, permissions, and app preferences.

#### Prerequisites
- Task 1.8 (LLM Abstraction Layer) completed
- Task 1.11 (Permission System) completed

#### Implementation Steps

1. **Create settings components**
   ```
   src/lib/components/settings/
   ├── SettingsLayout.svelte
   ├── sections/
   │   ├── LlmSettings.svelte
   │   ├── PermissionSettings.svelte
   │   ├── VaultSettings.svelte
   │   └── AboutSection.svelte
   └── common/
       ├── SettingsSection.svelte
       └── SettingsToggle.svelte
   ```

2. **Implement LLM settings**
   - Provider selection (Anthropic, Ollama, none)
   - API key input (encrypted storage)
   - Ollama URL configuration
   - Test connection button

3. **Implement permission settings**
   - List all permissions with toggles
   - Preset selection dropdown
   - Reset to preset button

4. **Implement vault settings**
   - Auto-lock timeout slider
   - Change master password
   - Export/import profile

5. **Add Tauri commands for settings**
   - Get/set config values
   - Test LLM connection
   - Change password

6. **Create settings route** (`src/routes/settings/+page.svelte`)

#### Acceptance Criteria
- [ ] LLM provider can be configured
- [ ] API key is stored securely
- [ ] Connection test works
- [ ] Permissions can be modified
- [ ] Settings persist across restarts

#### Test Commands
```bash
npm run check
npm run lint
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add src-tauri/ src/
git commit -m "feat(ui): add settings page with LLM and permission config

- Create settings layout with sections
- Implement LLM provider configuration
- Add API key encrypted storage
- Create permission toggles with presets
- Add vault settings (auto-lock, password change)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 1.15: Phase 1 Integration Testing**

---

### Task 1.15: Phase 1 Integration Testing
**Status:** `[ ]`
**Estimated Scope:** End-to-end testing of all Phase 1 features

#### Objective
Verify all Phase 1 features work together correctly before moving to Phase 2.

#### Prerequisites
- All previous Phase 1 tasks completed

#### Implementation Steps

1. **Create integration test suite**
   ```
   tests/
   ├── integration/
   │   ├── mod.rs
   │   ├── vault_flow.rs
   │   ├── scan_flow.rs
   │   └── chat_flow.rs
   ```

2. **Implement vault flow tests**
   - Create vault → unlock → lock cycle
   - Wrong password rejection
   - Profile CRUD operations

3. **Implement scan flow tests**
   - Load broker definitions
   - Execute scan against test fixtures
   - Verify result persistence

4. **Implement chat flow tests** (with mock LLM)
   - Send status query
   - Verify tool execution
   - Test without LLM configured

5. **Create test fixtures**
   - Mock HTTP responses for brokers
   - Test profile data
   - Mock LLM responses

6. **Document known limitations**
   - Update README with Phase 1 capabilities
   - Note what's coming in Phase 2

#### Acceptance Criteria
- [ ] All integration tests pass
- [ ] Test coverage > 60% for core crates
- [ ] No clippy warnings
- [ ] Documentation is updated
- [ ] Release notes drafted

#### Test Commands
```bash
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
cargo tarpaulin --out Html  # Coverage report
```

#### Commit Instructions
```bash
git add tests/ docs/ README.md
git commit -m "test(phase1): add integration tests and documentation

- Create vault, scan, chat integration test suites
- Add test fixtures for HTTP mocking
- Achieve >60% test coverage
- Update README with Phase 1 capabilities
- Draft v0.1 release notes

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
git tag -a v0.1.0 -m "Phase 1: Foundation release"
git push origin v0.1.0
```

#### Next Task
→ Proceed to **Phase 2: Automation**

---

## Phase 2: Automation (v0.2) — ~6 weeks

### Overview
Add browser automation for opt-out form submission, email-based removals, scheduling, and a status dashboard with privacy score.

---

### Task 2.1: Browser Automation Engine
**Status:** `[ ]`
**Estimated Scope:** Create spectral-browser with headless Chromium

#### Objective
Build the browser automation engine using chromiumoxide for JavaScript-heavy broker sites.

#### Prerequisites
- Phase 1 completed

#### Implementation Steps

1. **Create spectral-browser crate**
   ```
   crates/spectral-browser/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── engine.rs
       ├── fingerprint.rs
       ├── actions.rs
       └── error.rs
   ```

2. **Add chromiumoxide dependency**
   ```toml
   [dependencies]
   chromiumoxide = { version = "0.7", features = ["tokio-runtime"] }
   ```

3. **Implement browser engine** (`src/engine.rs`)
   - Launch headless Chrome/Chromium
   - Configure anti-fingerprinting
   - Rate limiting per domain
   - Screenshot capture

4. **Implement fingerprinting protection** (`src/fingerprint.rs`)
   - Randomized user agent
   - Viewport variation
   - WebGL noise injection
   - Timezone matching

5. **Implement browser actions** (`src/actions.rs`)
   - Navigate to URL
   - Fill form field
   - Click element
   - Wait for selector
   - Extract text

#### Acceptance Criteria
- [ ] Browser launches in headless mode
- [ ] Navigation works reliably
- [ ] Form interactions succeed
- [ ] Screenshots capture correctly
- [ ] Rate limiting prevents abuse

#### Test Commands
```bash
cargo test -p spectral-browser
cargo clippy -p spectral-browser -- -D warnings
```

#### Commit Instructions
```bash
git add crates/spectral-browser/
git commit -m "feat(browser): add headless browser automation engine

- Integrate chromiumoxide for headless Chrome
- Implement anti-fingerprinting measures
- Add rate limiting per domain
- Create action primitives (navigate, fill, click)
- Support screenshot capture for evidence

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 2.2: Form-Based Removal**

---

### Task 2.2: Form-Based Removal
**Status:** `[ ]`
**Estimated Scope:** Automate opt-out form submission for Tier 1 brokers

#### Objective
Implement automated form submission for brokers with WebForm removal method.

#### Prerequisites
- Task 2.1 (Browser Automation Engine) completed

#### Implementation Steps

1. **Extend spectral-broker with removal execution**
   ```
   crates/spectral-broker/src/
   ├── removal/
   │   ├── mod.rs
   │   ├── web_form.rs
   │   └── result.rs
   ```

2. **Implement form submission** (`src/removal/web_form.rs`)
   - Navigate to opt-out URL
   - Fill fields from profile + listing URL
   - Handle CAPTCHA detection (pause for user)
   - Submit and verify success

3. **Define removal result types**
   ```rust
   pub enum RemovalOutcome {
       Submitted,
       RequiresEmailVerification { email: String },
       RequiresCaptcha,
       RequiresAccountCreation,
       Failed { reason: String },
   }
   ```

4. **Update broker definitions**
   - Add form selectors for initial 5 brokers
   - Define field mappings

5. **Add Tauri commands**
   ```rust
   #[tauri::command]
   async fn submit_removal(broker_result_id: String) -> Result<RemovalOutcome, CommandError>;
   ```

#### Acceptance Criteria
- [ ] Form submission works for test broker
- [ ] CAPTCHA is detected and paused
- [ ] Email verification flow initiated
- [ ] Results stored in database
- [ ] UI shows removal status

#### Test Commands
```bash
cargo test -p spectral-broker
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-broker/ broker-definitions/ src-tauri/
git commit -m "feat(removal): automate web form opt-out submission

- Implement form-filling automation
- Add CAPTCHA detection and user pause
- Handle email verification requirements
- Update broker definitions with selectors
- Add submit_removal Tauri command

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 2.3: Email Removal Flow**

---

### Task 2.3: Email Removal Flow
**Status:** `[ ]`
**Estimated Scope:** Generate and send opt-out emails

#### Objective
Implement email-based removal for brokers that require email requests.

#### Prerequisites
- Task 1.3 (Encrypted Vault) completed

#### Implementation Steps

1. **Create spectral-mail crate**
   ```
   crates/spectral-mail/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── templates.rs
       ├── sender.rs
       └── error.rs
   ```

2. **Implement email templates** (`src/templates.rs`)
   - Generic opt-out request template
   - Broker-specific templates
   - Variable substitution

3. **Implement email sending** (`src/sender.rs`)
   - SMTP configuration (user-provided server)
   - Copy-to-clipboard fallback
   - Send via user's email client (mailto:)

4. **Add email Tauri commands**
   ```rust
   #[tauri::command]
   async fn generate_removal_email(broker_id: String) -> Result<EmailDraft, CommandError>;

   #[tauri::command]
   async fn send_removal_email(email: EmailDraft) -> Result<(), CommandError>;

   #[tauri::command]
   async fn copy_email_to_clipboard(email: EmailDraft) -> Result<(), CommandError>;
   ```

5. **Create email preview UI**
   - Show generated email
   - Edit before sending
   - Send or copy options

#### Acceptance Criteria
- [ ] Email templates generate correctly
- [ ] SMTP sending works when configured
- [ ] Copy-to-clipboard works
- [ ] mailto: link opens email client
- [ ] Sent emails are logged

#### Test Commands
```bash
cargo test -p spectral-mail
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-mail/ src-tauri/ src/
git commit -m "feat(email): add email-based removal flow

- Create email templates with variable substitution
- Implement SMTP sending (user-configured)
- Add copy-to-clipboard and mailto fallbacks
- Create email preview and edit UI
- Log sent emails for tracking

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 2.4: Scan Scheduler**

---

### Task 2.4: Scan Scheduler
**Status:** `[ ]`
**Estimated Scope:** Background task scheduling for periodic scans

#### Objective
Create the scheduler for automated periodic broker scans.

#### Prerequisites
- Task 1.9 (Manual Scan Engine) completed
- Task 1.11 (Permission System) completed

#### Implementation Steps

1. **Create spectral-scheduler crate**
   ```
   crates/spectral-scheduler/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── scheduler.rs
       ├── jobs.rs
       └── persistence.rs
   ```

2. **Implement scheduler** (`src/scheduler.rs`)
   - Cron-like scheduling expressions
   - Job queue with priority
   - Retry with exponential backoff

3. **Define job types** (`src/jobs.rs`)
   ```rust
   pub enum Job {
       ScanBroker { broker_id: BrokerId },
       ScanAll,
       VerifyRemoval { broker_result_id: String },
       RecheckBroker { broker_id: BrokerId },
   }
   ```

4. **Implement persistence** (`src/persistence.rs`)
   - Store scheduled jobs in database
   - Survive app restart
   - Track execution history

5. **Add Tauri commands**
   ```rust
   #[tauri::command]
   async fn schedule_scan(schedule: ScanSchedule) -> Result<(), CommandError>;

   #[tauri::command]
   async fn get_scheduled_jobs() -> Result<Vec<ScheduledJob>, CommandError>;
   ```

6. **Create schedule UI**
   - Schedule configuration in settings
   - View upcoming jobs
   - Pause/resume scheduling

#### Acceptance Criteria
- [ ] Jobs execute on schedule
- [ ] Jobs persist across restart
- [ ] Retry logic works correctly
- [ ] UI shows upcoming jobs
- [ ] Respects permission settings

#### Test Commands
```bash
cargo test -p spectral-scheduler
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-scheduler/ src-tauri/ src/
git commit -m "feat(scheduler): add background task scheduling

- Implement cron-like job scheduler
- Define ScanBroker, ScanAll, VerifyRemoval jobs
- Persist jobs across app restarts
- Add retry with exponential backoff
- Create schedule configuration UI

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 2.5: Dashboard with Privacy Score**

---

### Task 2.5: Dashboard with Privacy Score
**Status:** `[ ]`
**Estimated Scope:** Enhanced dashboard with privacy metrics and timeline

#### Objective
Build an enhanced dashboard showing privacy score, removal timeline, and status tracking.

#### Prerequisites
- Task 1.10 (Scan Results UI) completed
- Task 2.2 (Form-Based Removal) completed

#### Implementation Steps

1. **Implement privacy score calculation**
   - Score based on: brokers found, removals pending, removals confirmed
   - Weight by broker category and data sensitivity
   - Trend over time

2. **Create dashboard components**
   ```
   src/lib/components/dashboard/
   ├── PrivacyScore.svelte
   ├── ScoreGauge.svelte
   ├── Timeline.svelte
   ├── TimelineEvent.svelte
   ├── StatusSummary.svelte
   └── BrokerBreakdown.svelte
   ```

3. **Implement PrivacyScore**
   - Circular gauge visualization
   - Score explanation tooltip
   - Trend indicator (up/down)

4. **Implement Timeline**
   - Chronological event list
   - Event types: scan, found, removal requested, removal confirmed
   - Filter by type

5. **Implement StatusSummary**
   - Cards showing: Total scanned, Found, Pending removal, Removed
   - Click to filter broker list

#### Acceptance Criteria
- [ ] Privacy score calculates correctly
- [ ] Gauge displays score visually
- [ ] Timeline shows all events
- [ ] Status cards are accurate
- [ ] Dashboard updates in real-time

#### Test Commands
```bash
npm run check
npm run lint
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add src/
git commit -m "feat(dashboard): add privacy score and removal timeline

- Implement privacy score calculation algorithm
- Create circular gauge visualization
- Build chronological event timeline
- Add status summary cards
- Show broker breakdown by category

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 2.6: Additional LLM Providers**

---

### Task 2.6: Additional LLM Providers
**Status:** `[ ]`
**Estimated Scope:** Add OpenAI, LM Studio, llama.cpp providers

#### Objective
Expand LLM support with additional cloud and local providers.

#### Prerequisites
- Task 1.8 (LLM Abstraction Layer) completed

#### Implementation Steps

1. **Add OpenAI provider** (`src/providers/openai.rs`)
   - GPT-4o integration
   - Tool use support
   - Vision capabilities for CAPTCHA

2. **Add LM Studio provider** (`src/providers/lmstudio.rs`)
   - OpenAI-compatible API
   - Model listing
   - Configuration

3. **Add llama.cpp provider** (`src/providers/llamacpp.rs`)
   - Direct server integration
   - GGUF model support

4. **Update settings UI**
   - Provider selection dropdown
   - Provider-specific configuration
   - Model selection for local providers

5. **Implement provider auto-detection**
   - Check if Ollama/LM Studio is running
   - Suggest configuration

#### Acceptance Criteria
- [ ] OpenAI provider works with API key
- [ ] LM Studio connects locally
- [ ] llama.cpp server integration works
- [ ] UI shows all providers
- [ ] Auto-detection suggests options

#### Test Commands
```bash
cargo test -p spectral-llm
npm run tauri dev  # Manual testing with providers
```

#### Commit Instructions
```bash
git add crates/spectral-llm/ src/
git commit -m "feat(llm): add OpenAI, LM Studio, llama.cpp providers

- Implement OpenAI provider with GPT-4o support
- Add LM Studio OpenAI-compatible integration
- Integrate llama.cpp direct server connection
- Update settings with provider selection
- Add local provider auto-detection

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 2.7: Removal Verification Engine**

---

### Task 2.7: Removal Verification Engine
**Status:** `[ ]`
**Estimated Scope:** Track removal requests and verify completion

#### Objective
Build the verification system that confirms removals and tracks legal timelines.

#### Prerequisites
- Task 2.2 (Form-Based Removal) completed
- Task 2.4 (Scan Scheduler) completed

#### Implementation Steps

1. **Create spectral-verify crate**
   ```
   crates/spectral-verify/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── scheduler.rs
       ├── checker.rs
       ├── legal.rs
       └── escalation.rs
   ```

2. **Implement verification scheduler** (`src/scheduler.rs`)
   - Calculate optimal check times
   - Increase frequency near deadlines
   - Integrate with spectral-scheduler

3. **Implement verification checker** (`src/checker.rs`)
   - Re-scan broker for listing
   - Compare to original finding
   - Update status in database

4. **Implement legal tracking** (`src/legal.rs`)
   - CCPA 45-day timeline
   - GDPR 30-day timeline
   - Deadline calculations

5. **Implement escalation** (`src/escalation.rs`)
   - Warning when deadline approaching
   - Suggest next steps
   - Draft escalation email

#### Acceptance Criteria
- [ ] Verification checks run automatically
- [ ] Legal timelines calculated correctly
- [ ] Removals marked confirmed/reappeared
- [ ] Escalation warnings shown
- [ ] Escalation emails drafted

#### Test Commands
```bash
cargo test -p spectral-verify
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-verify/ src-tauri/ src/
git commit -m "feat(verify): add removal verification and legal tracking

- Schedule automatic re-scans after removal requests
- Track CCPA/GDPR legal timelines
- Mark listings as confirmed removed or reappeared
- Implement escalation warnings and email drafts
- Show verification status in dashboard

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 2.8: Email Thread Management**

---

### Task 2.8: Email Thread Management
**Status:** `[ ]`
**Estimated Scope:** Track email conversations with safety guardrails

#### Objective
Extend spectral-mail to manage ongoing email threads with brokers including safety controls.

#### Prerequisites
- Task 2.3 (Email Removal Flow) completed

#### Implementation Steps

1. **Extend spectral-mail**
   ```
   crates/spectral-mail/src/
   ├── thread.rs
   ├── classifier.rs
   ├── safety.rs
   └── budget.rs
   ```

2. **Implement thread tracking** (`src/thread.rs`)
   - Track email threads per broker
   - Store sent/received messages
   - State machine for thread lifecycle

3. **Implement response classifier** (`src/classifier.rs`)
   - Categorize broker responses
   - Detect confirmation, rejection, request for info
   - Flag suspicious responses

4. **Implement safety guardrails** (`src/safety.rs`)
   - Prompt injection detection
   - PII leak prevention
   - User approval for sensitive replies

5. **Implement reply budget** (`src/budget.rs`)
   - Limit auto-replies per thread
   - Track LLM token usage
   - Freeze thread when limit reached

#### Acceptance Criteria
- [ ] Threads tracked in database
- [ ] Responses classified correctly
- [ ] Safety checks prevent dangerous replies
- [ ] Budget limits auto-replies
- [ ] UI shows thread status

#### Test Commands
```bash
cargo test -p spectral-mail
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-mail/ src-tauri/ src/
git commit -m "feat(email): add thread management with safety guardrails

- Track email threads per broker result
- Implement response classification (confirm, reject, info-request)
- Add prompt injection and PII leak detection
- Enforce reply budget limits
- Show thread status in UI

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 2.9: Phase 2 Integration Testing**

---

### Task 2.9: Phase 2 Integration Testing
**Status:** `[ ]`
**Estimated Scope:** End-to-end testing of automation features

#### Objective
Verify all Phase 2 automation features work correctly before Phase 3.

#### Prerequisites
- All previous Phase 2 tasks completed

#### Implementation Steps

1. **Create Phase 2 integration tests**
   - Browser automation flow
   - Form submission flow
   - Email flow with thread tracking
   - Scheduler execution
   - Verification flow

2. **Create mock services**
   - Mock HTTP server for broker sites
   - Mock SMTP server
   - Mock LLM for responses

3. **Performance testing**
   - Scan 50 brokers concurrently
   - Measure memory usage
   - Check for leaks

4. **Update documentation**
   - Phase 2 feature guide
   - Automation configuration
   - Troubleshooting

#### Acceptance Criteria
- [ ] All integration tests pass
- [ ] Performance meets targets
- [ ] No memory leaks detected
- [ ] Documentation complete
- [ ] Release notes drafted

#### Test Commands
```bash
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
```

#### Commit Instructions
```bash
git add tests/ docs/
git commit -m "test(phase2): add automation integration tests

- Create browser, form, email, scheduler integration tests
- Add mock HTTP/SMTP/LLM services
- Verify performance with 50 concurrent brokers
- Update documentation for Phase 2 features

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
git tag -a v0.2.0 -m "Phase 2: Automation release"
git push origin v0.2.0
```

#### Next Task
→ Proceed to **Phase 3: Intelligence**

---

## Phase 3: Intelligence (v0.3) — ~6 weeks

### Overview
Add LLM-guided browser sessions, PII tokenization for cloud safety, smart matching, plugin system, and local discovery.

---

### Task 3.1: LLM-Guided Browser Sessions
**Status:** `[ ]`
**Estimated Scope:** LLM controls browser for complex broker sites

#### Objective
Enable the LLM to autonomously navigate complex broker sites when definitions aren't sufficient.

#### Prerequisites
- Task 2.1 (Browser Automation Engine) completed
- Task 1.8 (LLM Abstraction Layer) completed

#### Implementation Steps

1. **Extend spectral-browser**
   ```
   crates/spectral-browser/src/
   └── llm_session.rs
   ```

2. **Define browser tools for LLM**
   ```rust
   pub enum BrowserTool {
       Navigate { url: String },
       Click { selector: String },
       Type { selector: String, text: String },
       Screenshot,
       GetPageContent,
       WaitForSelector { selector: String },
   }
   ```

3. **Implement LLM session loop**
   - Provide page screenshot to LLM
   - LLM decides next action
   - Execute action and loop
   - Detect completion/failure

4. **Implement safety constraints**
   - Maximum actions per session
   - Allowed domains only
   - PII handling rules

5. **Update broker definitions**
   - Add `LlmGuided` search/removal methods
   - Provide natural language instructions

#### Acceptance Criteria
- [ ] LLM can navigate simple site
- [ ] Actions execute correctly
- [ ] Safety limits are enforced
- [ ] Sessions complete or timeout gracefully
- [ ] Screenshots are captured

#### Test Commands
```bash
cargo test -p spectral-browser
# Integration test with mock LLM
```

#### Commit Instructions
```bash
git add crates/spectral-browser/ broker-definitions/
git commit -m "feat(browser): add LLM-guided browser sessions

- Define BrowserTool enum for LLM actions
- Implement session loop with screenshot feedback
- Add safety constraints (max actions, allowed domains)
- Support LlmGuided broker definitions
- Capture session screenshots for debugging

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 3.2: PII Tokenization Pipeline**

---

### Task 3.2: PII Tokenization Pipeline
**Status:** `[ ]`
**Estimated Scope:** Reversible tokenization for safe cloud LLM usage

#### Objective
Implement the tokenization strategy that replaces real PII with reversible tokens for cloud LLM calls.

#### Prerequisites
- Task 1.8 (LLM Abstraction Layer) completed

#### Implementation Steps

1. **Extend spectral-llm pii_filter**
   ```rust
   pub struct TokenizationContext {
       token_map: HashMap<String, String>,
       reverse_map: HashMap<String, String>,
   }

   impl PiiFilter {
       pub fn tokenize(&mut self, text: &str) -> (String, TokenizationContext);
       pub fn detokenize(&self, text: &str, ctx: &TokenizationContext) -> String;
   }
   ```

2. **Implement token generation**
   - Unique tokens per session (e.g., `[NAME_A7F3]`, `[EMAIL_B2C4]`)
   - Type-preserving (email token looks like email)
   - Cryptographically random suffixes

3. **Implement token mapping persistence**
   - Store in memory only during request
   - Zeroize after detokenization
   - Never persist to disk

4. **Update router to use tokenization**
   - Tokenize before cloud send
   - Detokenize on response
   - Preserve local-only routing

5. **Add tokenization metrics**
   - Track PII detected per request
   - Log token usage (without values)

#### Acceptance Criteria
- [ ] PII replaced with tokens
- [ ] Tokens are reversible
- [ ] Maps are zeroized after use
- [ ] Metrics track detections
- [ ] Round-trip preserves meaning

#### Test Commands
```bash
cargo test -p spectral-llm
```

#### Commit Instructions
```bash
git add crates/spectral-llm/
git commit -m "feat(llm): add PII tokenization for cloud safety

- Implement reversible token generation
- Create type-preserving placeholder tokens
- Zeroize token maps after detokenization
- Update router to tokenize for cloud providers
- Add tokenization metrics logging

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 3.3: Smart Match Confidence**

---

### Task 3.3: Smart Match Confidence
**Status:** `[ ]`
**Estimated Scope:** ML-based confidence scoring for broker matches

#### Objective
Implement intelligent matching to determine if a broker listing actually belongs to the user.

#### Prerequisites
- Task 1.9 (Manual Scan Engine) completed
- Task 1.8 (LLM Abstraction Layer) completed

#### Implementation Steps

1. **Extend spectral-broker with matcher**
   ```
   crates/spectral-broker/src/
   └── matcher/
       ├── mod.rs
       ├── features.rs
       ├── scorer.rs
       └── trainer.rs  # future: user feedback learning
   ```

2. **Implement feature extraction** (`features.rs`)
   - Name similarity (fuzzy match)
   - Location proximity
   - Age range overlap
   - Associated people match
   - Phone/email partial match

3. **Implement confidence scorer** (`scorer.rs`)
   - Weighted feature combination
   - Configurable thresholds
   - LLM-assisted for ambiguous cases

4. **Update scan results**
   - Add confidence field
   - Store feature breakdown
   - Allow user override

5. **Create match review UI**
   - Show uncertain matches
   - Explain confidence factors
   - User confirms/rejects

#### Acceptance Criteria
- [ ] Features extracted correctly
- [ ] Confidence scores are sensible
- [ ] LLM assists ambiguous cases
- [ ] UI shows confidence breakdown
- [ ] User feedback is stored

#### Test Commands
```bash
cargo test -p spectral-broker
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-broker/ src/
git commit -m "feat(scan): add smart match confidence scoring

- Implement feature extraction (name, location, age, etc.)
- Create weighted confidence scorer
- Add LLM-assisted disambiguation
- Show confidence breakdown in UI
- Support user override for matches

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 3.4: Broker Definition Auto-Repair**

---

### Task 3.4: Broker Definition Auto-Repair
**Status:** `[ ]`
**Estimated Scope:** LLM detects and suggests fixes for broken broker definitions

#### Objective
Implement automatic detection of broken broker definitions and LLM-assisted repair suggestions.

#### Prerequisites
- Task 3.1 (LLM-Guided Browser Sessions) completed

#### Implementation Steps

1. **Implement definition health checker**
   - Track scan success/failure rate
   - Detect changed URLs/selectors
   - Monitor removal success rate

2. **Implement LLM repair assistant**
   - Analyze failed scans
   - Suggest updated selectors
   - Generate updated definition

3. **Create repair workflow**
   - Alert on broken definition
   - Show LLM suggestions
   - User approves changes
   - Submit to community (optional)

4. **Add health dashboard**
   - Show broker health status
   - List broken definitions
   - Repair queue

#### Acceptance Criteria
- [ ] Broken definitions detected
- [ ] LLM suggests repairs
- [ ] User can approve/reject
- [ ] Approved repairs are applied
- [ ] Health dashboard shows status

#### Test Commands
```bash
cargo test -p spectral-broker
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-broker/ src/
git commit -m "feat(broker): add automatic definition repair

- Track broker definition health metrics
- Detect broken selectors and URLs
- Use LLM to suggest definition repairs
- Create approval workflow for changes
- Add broker health dashboard

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 3.5: Plugin System (Extism)**

---

### Task 3.5: Plugin System (Extism)
**Status:** `[ ]`
**Estimated Scope:** WASM-based plugin runtime

#### Objective
Create the plugin system using Extism for safe, sandboxed extensibility.

#### Prerequisites
- Task 1.1 (spectral-core) completed

#### Implementation Steps

1. **Create spectral-plugins crate**
   ```
   crates/spectral-plugins/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── runtime.rs
       ├── manifest.rs
       ├── permissions.rs
       └── host_functions.rs
   ```

2. **Integrate Extism runtime** (`src/runtime.rs`)
   - Load WASM plugins
   - Execute plugin functions
   - Resource limits (memory, time)

3. **Define plugin manifest** (`src/manifest.rs`)
   - Plugin metadata (name, version, author)
   - Plugin type (broker, llm, notification, hook)
   - Required permissions

4. **Implement host functions** (`src/host_functions.rs`)
   - HTTP requests (allowed domains only)
   - Read profile (allowed fields only)
   - Log messages
   - Return results

5. **Create plugin loader**
   - Load from plugins/ directory
   - Verify signatures
   - Apply permission restrictions

#### Acceptance Criteria
- [ ] WASM plugins load successfully
- [ ] Host functions work
- [ ] Permissions are enforced
- [ ] Resource limits prevent abuse
- [ ] Signatures are verified

#### Test Commands
```bash
cargo test -p spectral-plugins
# Test with sample plugin
```

#### Commit Instructions
```bash
git add crates/spectral-plugins/
git commit -m "feat(plugins): add WASM plugin runtime with Extism

- Integrate Extism for WASM plugin execution
- Define plugin manifest format
- Implement permission-gated host functions
- Add resource limits (memory, time)
- Support plugin signature verification

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 3.6: Local PII Discovery**

---

### Task 3.6: Local PII Discovery
**Status:** `[ ]`
**Estimated Scope:** Scan local filesystem for PII exposure

#### Objective
Implement the local discovery engine that finds PII in files, emails, and browser data.

#### Prerequisites
- Task 1.11 (Permission System) completed
- Task 1.1 (spectral-core) completed

#### Implementation Steps

1. **Create spectral-discovery crate**
   ```
   crates/spectral-discovery/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── orchestrator.rs
       ├── detector.rs
       ├── scanners/
       │   ├── mod.rs
       │   ├── filesystem.rs
       │   ├── email.rs
       │   └── browser.rs
       └── parsers/
           ├── mod.rs
           ├── plaintext.rs
           ├── office.rs
           └── pdf.rs
   ```

2. **Implement filesystem scanner**
   - Walk directory tree
   - Respect exclusion patterns
   - Parse supported file types

3. **Implement file parsers**
   - Plain text extraction
   - Office document parsing (docx, xlsx)
   - PDF text extraction

4. **Implement PII detector**
   - Regex patterns for PII types
   - Match against user profile
   - Calculate exposure risk

5. **Add discovery UI**
   - Start scan with directory selection
   - Show progress
   - Display findings with file paths
   - Risk scoring

#### Acceptance Criteria
- [ ] Filesystem scan works
- [ ] Parsers extract text correctly
- [ ] PII matching is accurate
- [ ] Permission check gates scan
- [ ] UI displays findings clearly

#### Test Commands
```bash
cargo test -p spectral-discovery
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-discovery/ src-tauri/ src/
git commit -m "feat(discovery): add local PII discovery engine

- Implement filesystem scanner with exclusion patterns
- Add parsers for plaintext, Office, PDF
- Create PII detector with regex patterns
- Match findings against user profile
- Build discovery UI with risk scoring

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 3.7: Network Telemetry Engine**

---

### Task 3.7: Network Telemetry Engine
**Status:** `[ ]`
**Estimated Scope:** Monitor network connections for privacy insights

#### Objective
Build the network monitoring engine that detects connections to data brokers and trackers.

#### Prerequisites
- Task 1.11 (Permission System) completed

#### Implementation Steps

1. **Create spectral-netmon crate**
   ```
   crates/spectral-netmon/
   ├── Cargo.toml
   └── src/
       ├── lib.rs
       ├── collectors/
       │   ├── mod.rs
       │   ├── dns.rs
       │   └── connections.rs
       ├── intelligence.rs
       ├── scoring.rs
       └── alerts.rs
   ```

2. **Implement DNS collector** (`collectors/dns.rs`)
   - Read system DNS cache
   - Parse resolved domains
   - Platform-specific implementation

3. **Implement connection collector** (`collectors/connections.rs`)
   - Read active connections (netstat/ss)
   - Map to domains
   - Track over time

4. **Implement domain intelligence** (`intelligence.rs`)
   - Load domain classification database
   - Categorize: broker, tracker, analytics, etc.
   - Score privacy risk

5. **Create network dashboard**
   - Show recent connections
   - Highlight concerning domains
   - Privacy score impact

#### Acceptance Criteria
- [ ] DNS cache reading works
- [ ] Connections are tracked
- [ ] Domains are classified
- [ ] Privacy score calculated
- [ ] Dashboard shows data

#### Test Commands
```bash
cargo test -p spectral-netmon
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-netmon/ src-tauri/ src/
git commit -m "feat(netmon): add network telemetry engine

- Implement DNS cache reader
- Add connection collector (netstat/ss)
- Create domain intelligence database
- Calculate privacy score from network activity
- Build network monitoring dashboard

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 3.8: Cross-Correlation Intelligence**

---

### Task 3.8: Cross-Correlation Intelligence
**Status:** `[ ]`
**Estimated Scope:** Generate insights from combined data sources

#### Objective
Implement the correlation engine that combines broker, discovery, and network data for insights.

#### Prerequisites
- Task 3.6 (Local PII Discovery) completed
- Task 3.7 (Network Telemetry Engine) completed

#### Implementation Steps

1. **Extend spectral-core with correlation**
   ```
   crates/spectral-core/src/
   └── correlation/
       ├── mod.rs
       ├── engine.rs
       ├── insights.rs
       └── suggestions.rs
   ```

2. **Implement correlation engine**
   - Connect data sources
   - Find relationships
   - Generate insights

3. **Define insight types**
   ```rust
   pub enum InsightType {
       BrokerNetworkActivity,    // Your device contacted a broker
       PiiExposureMatch,         // Local file contains broker-listed data
       RemovalSuccessPattern,    // Which removal methods work best
       RiskTrendAnalysis,        // Privacy score changes over time
   }
   ```

4. **Implement suggestion engine**
   - Recommend actions based on insights
   - Prioritize by impact
   - Track acted-on suggestions

5. **Create insights dashboard**
   - Show generated insights
   - Explain evidence
   - Action buttons

#### Acceptance Criteria
- [ ] Insights are generated
- [ ] Evidence is accurate
- [ ] Suggestions are actionable
- [ ] Dashboard displays insights
- [ ] Actions can be taken directly

#### Test Commands
```bash
cargo test -p spectral-core
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-core/ src/
git commit -m "feat(intelligence): add cross-correlation insights engine

- Implement correlation engine connecting all data sources
- Generate insights: broker activity, PII exposure, patterns
- Create suggestion engine with prioritized actions
- Build insights dashboard with evidence display
- Track acted-on suggestions

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 3.9: Notification Integrations**

---

### Task 3.9: Notification Integrations
**Status:** `[ ]`
**Estimated Scope:** Desktop notifications and email digests

#### Objective
Add notification channels for important events (found listings, removal confirmations, etc.).

#### Prerequisites
- Task 2.7 (Removal Verification Engine) completed

#### Implementation Steps

1. **Create notification system**
   ```
   crates/spectral-core/src/
   └── notifications/
       ├── mod.rs
       ├── channels/
       │   ├── desktop.rs
       │   └── email.rs
       └── templates.rs
   ```

2. **Implement desktop notifications**
   - Use tauri-plugin-notification
   - Critical alerts (new listing found)
   - Info alerts (removal confirmed)

3. **Implement email digest**
   - Daily/weekly summary option
   - HTML email template
   - Send via configured SMTP

4. **Define notification preferences**
   - Per-event-type toggles
   - Quiet hours
   - Frequency limits

5. **Add notification settings UI**
   - Channel configuration
   - Event preferences
   - Test notification button

#### Acceptance Criteria
- [ ] Desktop notifications work
- [ ] Email digest sends correctly
- [ ] Preferences are respected
- [ ] Quiet hours enforced
- [ ] Settings UI complete

#### Test Commands
```bash
cargo test -p spectral-core
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-core/ src-tauri/ src/
git commit -m "feat(notifications): add desktop and email notifications

- Integrate tauri-plugin-notification
- Implement email digest with SMTP
- Create notification templates
- Add per-event preferences and quiet hours
- Build notification settings UI

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 3.10: Phase 3 Integration Testing**

---

### Task 3.10: Phase 3 Integration Testing
**Status:** `[ ]`
**Estimated Scope:** End-to-end testing of intelligence features

#### Objective
Verify all Phase 3 intelligence features work correctly before Phase 4.

#### Prerequisites
- All previous Phase 3 tasks completed

#### Implementation Steps

1. **Create Phase 3 integration tests**
   - LLM-guided browser flow
   - PII tokenization round-trip
   - Smart matching accuracy
   - Plugin loading and execution
   - Discovery scanning
   - Network monitoring
   - Correlation insights

2. **Security audit**
   - PII tokenization review
   - Plugin sandboxing verification
   - Permission enforcement check

3. **Performance testing**
   - Discovery scan on large directory
   - Plugin execution overhead
   - Correlation engine memory

4. **Update documentation**
   - Phase 3 feature guide
   - Plugin development guide
   - Security model documentation

#### Acceptance Criteria
- [ ] All integration tests pass
- [ ] Security audit complete
- [ ] Performance acceptable
- [ ] Documentation updated
- [ ] Release notes drafted

#### Test Commands
```bash
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
```

#### Commit Instructions
```bash
git add tests/ docs/
git commit -m "test(phase3): add intelligence feature tests and security audit

- Create LLM session, tokenization, matching tests
- Add plugin security tests
- Verify discovery and netmon performance
- Complete security audit
- Update documentation for Phase 3

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
git tag -a v0.3.0 -m "Phase 3: Intelligence release"
git push origin v0.3.0
```

#### Next Task
→ Proceed to **Phase 4: Community**

---

## Phase 4: Community (v0.4) — ~4 weeks

### Overview
Add plugin marketplace, broker definition contributions, multi-profile support, and export features.

---

### Task 4.1: Plugin Marketplace
**Status:** `[ ]`
**Estimated Scope:** Browse and install community plugins

#### Objective
Create the plugin marketplace UI for discovering and installing community plugins.

#### Prerequisites
- Task 3.5 (Plugin System) completed

#### Implementation Steps

1. **Define plugin registry API**
   - GitHub-based registry
   - Plugin metadata JSON
   - Signature verification

2. **Create marketplace components**
   ```
   src/lib/components/plugins/
   ├── PluginMarketplace.svelte
   ├── PluginCard.svelte
   ├── PluginDetail.svelte
   └── InstalledPlugins.svelte
   ```

3. **Implement plugin installation**
   - Download WASM from registry
   - Verify signature
   - Store in plugins/ directory
   - Enable/disable toggle

4. **Add plugin management**
   - View installed plugins
   - Check for updates
   - Remove plugins

5. **Create marketplace route** (`src/routes/plugins/+page.svelte`)

#### Acceptance Criteria
- [ ] Marketplace shows available plugins
- [ ] Installation works
- [ ] Signatures verified
- [ ] Updates detected
- [ ] Plugins can be removed

#### Test Commands
```bash
npm run check
npm run lint
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add src/
git commit -m "feat(plugins): add plugin marketplace

- Create marketplace UI with plugin browser
- Implement GitHub-based plugin registry
- Add signature verification on install
- Support update checking
- Enable plugin removal

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 4.2: Broker Definition Contributions**

---

### Task 4.2: Broker Definition Contributions
**Status:** `[ ]`
**Estimated Scope:** Allow users to contribute new broker definitions

#### Objective
Create workflow for users to submit new or updated broker definitions to the community.

#### Prerequisites
- Task 1.7 (Broker Definitions Crate) completed
- Task 3.4 (Broker Definition Auto-Repair) completed

#### Implementation Steps

1. **Create broker definition editor**
   - Form-based definition creation
   - Test against live site
   - Validate completeness

2. **Implement contribution workflow**
   - Export definition as TOML
   - GitHub PR creation (via gh CLI)
   - Track submission status

3. **Add contribution UI**
   - "Contribute" button on broken broker
   - Definition editor
   - Submit to community

4. **Create contribution guide**
   - Documentation for definition format
   - Testing requirements
   - Review process

#### Acceptance Criteria
- [ ] Editor creates valid definitions
- [ ] Testing validates against site
- [ ] PR submission works
- [ ] Status tracking works
- [ ] Documentation complete

#### Test Commands
```bash
npm run check
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add src/ docs/
git commit -m "feat(community): add broker definition contribution workflow

- Create broker definition editor UI
- Implement live site testing
- Add GitHub PR submission
- Track contribution status
- Document contribution process

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 4.3: Broker Definition CI**

---

### Task 4.3: Broker Definition CI
**Status:** `[ ]`
**Estimated Scope:** Automated testing of broker definitions in CI

#### Objective
Set up CI pipeline to automatically test broker definitions for validity.

#### Prerequisites
- Task 4.2 (Broker Definition Contributions) completed

#### Implementation Steps

1. **Create broker test runner**
   - Load definition
   - Execute search against live site
   - Verify selectors work
   - Report results

2. **Add GitHub Action workflow** (`.github/workflows/broker-tests.yml`)
   - Trigger on broker-definitions/ changes
   - Run tests in matrix
   - Report failures

3. **Implement test fixtures**
   - Mock responses for unit tests
   - Integration test against real sites (scheduled)

4. **Add status badges**
   - Per-broker health badges
   - Include in broker README

#### Acceptance Criteria
- [ ] CI runs on definition changes
- [ ] Tests validate definitions
- [ ] Failures are reported
- [ ] Status badges work
- [ ] Scheduled integration tests run

#### Test Commands
```bash
cargo test -p spectral-broker
# CI tests run automatically
```

#### Commit Instructions
```bash
git add .github/workflows/ tests/ broker-definitions/
git commit -m "ci(brokers): add automated broker definition testing

- Create broker test runner
- Add GitHub Actions workflow for definitions
- Implement mock fixtures for unit tests
- Schedule weekly integration tests
- Add health status badges

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 4.4: Multi-Profile Support**

---

### Task 4.4: Multi-Profile Support
**Status:** `[ ]`
**Estimated Scope:** Support multiple user profiles (family plans)

#### Objective
Allow multiple profiles in a single vault for family use cases.

#### Prerequisites
- Task 1.3 (Encrypted Vault) completed
- Task 1.6 (Profile Setup UI) completed

#### Implementation Steps

1. **Extend vault for multi-profile**
   - Multiple profiles table support
   - Profile switching
   - Per-profile encryption keys

2. **Update profile management**
   - Add new profile
   - Switch active profile
   - Delete profile

3. **Update scan/removal tracking**
   - Filter by active profile
   - Show profile indicator in results

4. **Create profile switcher UI**
   - Profile selector in header
   - Quick switch menu
   - Profile management page

5. **Update dashboard for multi-profile**
   - Per-profile stats
   - Aggregate family view

#### Acceptance Criteria
- [ ] Multiple profiles can be created
- [ ] Switching works correctly
- [ ] Data is profile-isolated
- [ ] UI shows active profile
- [ ] Aggregate view works

#### Test Commands
```bash
cargo test -p spectral-vault
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-vault/ src-tauri/ src/
git commit -m "feat(vault): add multi-profile support

- Extend vault schema for multiple profiles
- Implement profile switching with isolation
- Add profile selector to header
- Create profile management page
- Support aggregate family dashboard

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 4.5: Export and Reporting**

---

### Task 4.5: Export and Reporting
**Status:** `[ ]`
**Estimated Scope:** Export data in multiple formats

#### Objective
Add export functionality for user data and activity reports.

#### Prerequisites
- Task 2.5 (Dashboard) completed

#### Implementation Steps

1. **Implement exporters**
   - Markdown report generator
   - JSON data export
   - PDF report (using printpdf or similar)

2. **Define report templates**
   - Activity summary
   - Broker status report
   - Privacy score history
   - Evidence archive

3. **Add export UI**
   - Export button in dashboard
   - Format selection
   - Date range filter
   - Download handling

4. **Implement scheduled reports**
   - Weekly summary export
   - Email delivery option
   - Storage management

#### Acceptance Criteria
- [ ] Markdown export works
- [ ] JSON export is valid
- [ ] PDF renders correctly
- [ ] Reports include all data
- [ ] Scheduled exports run

#### Test Commands
```bash
cargo test
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/ src-tauri/ src/
git commit -m "feat(reporting): add export and reporting features

- Implement Markdown, JSON, PDF exporters
- Create report templates for common use cases
- Add export UI with format and date selection
- Support scheduled report generation
- Enable email delivery option

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 4.6: Domain Intelligence Contributions**

---

### Task 4.6: Domain Intelligence Contributions
**Status:** `[ ]`
**Estimated Scope:** Community-contributed domain classifications

#### Objective
Allow users to contribute domain classifications for the network monitoring feature.

#### Prerequisites
- Task 3.7 (Network Telemetry Engine) completed

#### Implementation Steps

1. **Create domain classification editor**
   - Domain entry form
   - Category selection
   - Evidence linking

2. **Implement contribution workflow**
   - Export as TOML
   - GitHub PR submission
   - Review tracking

3. **Add community domain lists**
   - Import from external sources
   - Merge community contributions
   - Version tracking

4. **Update network monitoring**
   - Show community source
   - Allow local overrides

#### Acceptance Criteria
- [ ] Domains can be classified
- [ ] Contributions submit to GitHub
- [ ] External lists import
- [ ] Overrides work
- [ ] Sources are shown

#### Test Commands
```bash
cargo test -p spectral-netmon
npm run tauri dev  # Manual testing
```

#### Commit Instructions
```bash
git add crates/spectral-netmon/ src/ domains/
git commit -m "feat(community): add domain intelligence contributions

- Create domain classification editor
- Implement GitHub contribution workflow
- Support external list import
- Allow local category overrides
- Show contribution sources in UI

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 4.7: Documentation and Guides**

---

### Task 4.7: Documentation and Guides
**Status:** `[ ]`
**Estimated Scope:** Comprehensive user and contributor documentation

#### Objective
Create complete documentation for users, contributors, and plugin developers.

#### Prerequisites
- All other Phase 4 tasks completed

#### Implementation Steps

1. **Create user documentation**
   - Getting started guide
   - Feature walkthroughs
   - FAQ

2. **Create contributor guides**
   - Development setup
   - Code style guide
   - PR process

3. **Create plugin development guide**
   - Plugin architecture
   - API reference
   - Example plugins

4. **Create security documentation**
   - Threat model
   - Security practices
   - Responsible disclosure

5. **Set up documentation site**
   - MkDocs or similar
   - GitHub Pages deployment
   - Search functionality

#### Acceptance Criteria
- [ ] User docs cover all features
- [ ] Contributor guide enables new devs
- [ ] Plugin guide is comprehensive
- [ ] Security model documented
- [ ] Docs site is deployed

#### Test Commands
```bash
# Build and preview docs
mkdocs serve
```

#### Commit Instructions
```bash
git add docs/
git commit -m "docs: add comprehensive documentation

- Create getting started and feature guides
- Document contribution process
- Add plugin development guide
- Explain security model
- Deploy documentation site

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
```

#### Next Task
→ Proceed to **Task 4.8: Phase 4 Integration Testing**

---

### Task 4.8: Phase 4 Final Testing and Release
**Status:** `[ ]`
**Estimated Scope:** Final testing, polish, and v0.4 release

#### Objective
Complete final testing, polish, and prepare the v0.4 release.

#### Prerequisites
- All other Phase 4 tasks completed

#### Implementation Steps

1. **Comprehensive testing**
   - All integration tests
   - Manual QA checklist
   - Cross-platform verification

2. **Performance optimization**
   - Profile and optimize hot paths
   - Memory usage audit
   - Startup time optimization

3. **UI polish**
   - Consistent styling
   - Loading states
   - Error messages
   - Accessibility audit

4. **Release preparation**
   - Update version numbers
   - Finalize changelog
   - Create release binaries
   - Update website

5. **Community launch**
   - Announcement post
   - Social media
   - Documentation updates

#### Acceptance Criteria
- [ ] All tests pass
- [ ] Performance targets met
- [ ] UI polish complete
- [ ] Binaries built for all platforms
- [ ] Launch announcement ready

#### Test Commands
```bash
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
npm run check
npm run lint
```

#### Commit Instructions
```bash
git add .
git commit -m "chore(release): prepare v0.4.0 release

- Complete integration testing
- Apply performance optimizations
- Polish UI and error messages
- Update all documentation
- Prepare release binaries

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"

git push origin master
git tag -a v0.4.0 -m "Phase 4: Community release"
git push origin v0.4.0
```

---

## Appendix: Task Dependencies

```
Phase 1 (Foundation)
├── 1.1 Core Crate Setup
│   ├── 1.2 Database Layer → 1.3 Encrypted Vault
│   │                           ├── 1.4 Tauri Commands Vault
│   │                           │   └── 1.5 Unlock Screen UI
│   │                           │       └── 1.6 Profile Setup UI
│   │                           └── 1.9 Manual Scan Engine
│   │                               └── 1.10 Scan Results UI
│   ├── 1.7 Broker Definitions ──────┘
│   ├── 1.8 LLM Abstraction ─────────────────┐
│   └── 1.11 Permission System               │
│       └── 1.12 Command Palette              │
│           └── 1.13 Chat Interface ──────────┘
│               └── 1.14 Settings UI
│                   └── 1.15 Phase 1 Integration Testing

Phase 2 (Automation)
├── 2.1 Browser Automation
│   └── 2.2 Form-Based Removal
│       └── 2.5 Dashboard with Privacy Score
├── 2.3 Email Removal Flow
│   └── 2.8 Email Thread Management
├── 2.4 Scan Scheduler
│   └── 2.7 Removal Verification
├── 2.6 Additional LLM Providers
└── 2.9 Phase 2 Integration Testing

Phase 3 (Intelligence)
├── 3.1 LLM-Guided Browser
│   └── 3.4 Broker Auto-Repair
├── 3.2 PII Tokenization
├── 3.3 Smart Match Confidence
├── 3.5 Plugin System
├── 3.6 Local PII Discovery ─────┐
├── 3.7 Network Telemetry ───────┼→ 3.8 Cross-Correlation
│                                │
├── 3.9 Notification Integrations
└── 3.10 Phase 3 Integration Testing

Phase 4 (Community)
├── 4.1 Plugin Marketplace
├── 4.2 Broker Contributions
│   └── 4.3 Broker CI
├── 4.4 Multi-Profile Support
├── 4.5 Export and Reporting
├── 4.6 Domain Contributions
├── 4.7 Documentation
└── 4.8 Final Testing and Release
```

---

## Quick Reference

### Starting a Task
1. Read the full task description
2. Check prerequisites are complete
3. Update task status to `[~]`
4. Follow implementation steps
5. Run test commands
6. Commit with provided template
7. Update task status to `[x]`
8. Proceed to next task

### Commit Message Format
All commits should follow conventional commits:
- `feat(scope): description` - New features
- `fix(scope): description` - Bug fixes
- `test(scope): description` - Test additions
- `docs(scope): description` - Documentation
- `chore(scope): description` - Maintenance

### If You Get Stuck
1. Re-read the acceptance criteria
2. Check the architecture docs in `architecture/`
3. Review patterns in `patterns.md`
4. Check previous similar tasks for patterns
5. Ask for clarification before proceeding
