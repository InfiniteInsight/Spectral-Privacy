# Spectral Development Patterns

This document defines the coding patterns, conventions, and architectural decisions for the Spectral codebase. All contributors should follow these patterns for consistency.

## Table of Contents

1. [Error Handling](#1-error-handling)
2. [Async & Concurrency](#2-async--concurrency)
3. [State Management](#3-state-management)
4. [Testing](#4-testing)
5. [Logging & Tracing](#5-logging--tracing)
6. [Configuration](#6-configuration)
7. [API Design](#7-api-design)
8. [Frontend Components](#8-frontend-components)
9. [Security Patterns](#9-security-patterns)
10. [Database Patterns](#10-database-patterns)
11. [Authentication](#11-authentication)

---

## 1. Error Handling

### Rust Error Types

Use `thiserror` for library crates, `anyhow` only in the Tauri app shell.

```rust
// In library crates (spectral-vault, spectral-broker, etc.)
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("vault is locked")]
    Locked,

    #[error("decryption failed: invalid key or corrupted data")]
    DecryptionFailed,

    #[error("field not found: {field}")]
    FieldNotFound { field: String },

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// In the Tauri app shell (src-tauri/src/)
use anyhow::{Context, Result};

fn load_config() -> Result<Config> {
    let path = config_path().context("failed to determine config path")?;
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config from {}", path.display()))?;
    toml::from_str(&contents).context("failed to parse config TOML")
}
```

### Error Propagation

- Use `?` operator for propagation, never `.unwrap()` in production code
- Use `.expect("reason")` only when the condition is truly impossible
- Add context when crossing module boundaries

```rust
// Good: context at boundary
pub async fn scan_broker(&self, broker_id: &str) -> Result<ScanResult, BrokerError> {
    let definition = self.get_definition(broker_id)
        .await
        .map_err(|e| BrokerError::DefinitionLoad { broker_id: broker_id.to_string(), source: e })?;

    self.browser.navigate(&definition.search_url)
        .await
        .map_err(|e| BrokerError::Navigation { url: definition.search_url.clone(), source: e })?;

    // ...
}

// Bad: losing context
pub async fn scan_broker(&self, broker_id: &str) -> Result<ScanResult, BrokerError> {
    let definition = self.get_definition(broker_id).await?; // Which broker failed?
    self.browser.navigate(&definition.search_url).await?;    // Which URL?
}
```

### User-Facing Errors

Tauri commands should return structured errors that the frontend can display:

```rust
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,           // machine-readable: "vault_locked", "broker_not_found"
    pub message: String,        // user-readable: "Please unlock your vault first"
    pub details: Option<String>, // technical details for debugging (never PII)
}

impl From<VaultError> for CommandError {
    fn from(e: VaultError) -> Self {
        match e {
            VaultError::Locked => CommandError {
                code: "vault_locked".into(),
                message: "Please unlock your vault first.".into(),
                details: None,
            },
            VaultError::DecryptionFailed => CommandError {
                code: "decryption_failed".into(),
                message: "Failed to decrypt data. Your password may be incorrect.".into(),
                details: None,
            },
            // ...
        }
    }
}
```

---

## 2. Async & Concurrency

### Task Spawning

Use `tokio::spawn` for fire-and-forget tasks, but prefer structured concurrency:

```rust
// Good: structured concurrency with join
pub async fn scan_all_brokers(&self, broker_ids: &[String]) -> Vec<ScanResult> {
    let futures: Vec<_> = broker_ids
        .iter()
        .map(|id| self.scan_broker(id))
        .collect();

    let results = futures::future::join_all(futures).await;
    results.into_iter().filter_map(Result::ok).collect()
}

// Good: bounded concurrency
use futures::stream::{self, StreamExt};

pub async fn scan_all_brokers(&self, broker_ids: &[String]) -> Vec<ScanResult> {
    stream::iter(broker_ids)
        .map(|id| self.scan_broker(id))
        .buffer_unordered(5)  // max 5 concurrent scans
        .filter_map(|r| async { r.ok() })
        .collect()
        .await
}
```

### Cancellation

All long-running operations must support cancellation via `CancellationToken`:

```rust
use tokio_util::sync::CancellationToken;

pub async fn scan_broker(
    &self,
    broker_id: &str,
    cancel: CancellationToken,
) -> Result<ScanResult, BrokerError> {
    // Check cancellation at async boundaries
    if cancel.is_cancelled() {
        return Err(BrokerError::Cancelled);
    }

    let page = self.browser.new_page().await?;

    tokio::select! {
        _ = cancel.cancelled() => Err(BrokerError::Cancelled),
        result = self.do_scan(&page, broker_id) => result,
    }
}
```

### Channel Patterns

Use typed channels for inter-component communication:

```rust
// Event channel for progress updates
pub enum ScanEvent {
    Started { broker_id: String },
    Progress { broker_id: String, step: String },
    Found { broker_id: String, listing_url: String },
    NotFound { broker_id: String },
    Error { broker_id: String, error: String },
    Completed,
}

pub struct Scanner {
    event_tx: mpsc::Sender<ScanEvent>,
}

// In Tauri, emit events to frontend
impl Scanner {
    pub fn with_tauri_events(app: AppHandle) -> Self {
        let (tx, mut rx) = mpsc::channel(100);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                app.emit("scan-event", &event).ok();
            }
        });

        Self { event_tx: tx }
    }
}
```

---

## 3. State Management

### Rust-Side State

Use Tauri's managed state for shared resources:

```rust
// src-tauri/src/lib.rs
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            unlock_vault,
            lock_vault,
            scan_broker,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri app");
}

pub struct AppState {
    /// Base directory for all vaults: ~/.local/share/spectral/vaults/
    vaults_dir: PathBuf,
    /// Currently unlocked vaults: vault_id -> Vault
    /// Multiple family members can have vaults unlocked simultaneously
    unlocked_vaults: RwLock<HashMap<String, Arc<Vault>>>,
    scanner: RwLock<Option<Arc<Scanner>>>,
    config: RwLock<Config>,
}

impl AppState {
    pub fn new() -> Self {
        let dirs = directories::ProjectDirs::from("com", "spectral", "spectral")
            .expect("failed to determine project directories");
        let vaults_dir = dirs.data_dir().join("vaults");

        Self {
            vaults_dir,
            unlocked_vaults: RwLock::new(HashMap::new()),
            scanner: RwLock::new(None),
            config: RwLock::new(Config::default()),
        }
    }
}

// Access in commands - all commands take vault_id for explicit security
#[tauri::command]
async fn scan_broker(
    state: State<'_, AppState>,
    vault_id: String,
    broker_id: String,
) -> Result<ScanResult, CommandError> {
    let vaults = state.unlocked_vaults.read().await;
    let vault = vaults
        .get(&vault_id)
        .ok_or(CommandError::vault_locked())?
        .clone();

    // ...
}
```

**Multi-Vault Filesystem Structure:**

Each vault is stored in its own directory with metadata:

```
~/.local/share/spectral/vaults/
├── {vault-id-1}/
│   ├── metadata.json      # { "name": "Alice", "created_at": "..." }
│   ├── vault.db           # SQLite database with encrypted profiles
│   └── .vault_salt        # Argon2 salt (32 bytes)
└── {vault-id-2}/
    ├── metadata.json      # { "name": "Bob", "created_at": "..." }
    ├── vault.db
    └── .vault_salt
```

**Design Rationale:**
- **Explicit vault_id in all commands** for security (no ambient authority)
- **HashMap allows multiple concurrent unlocked vaults** for family use
- **RwLock for concurrent reads** (multiple commands can check status)
- **Each vault is self-contained** in its directory for easy backup/migration

### Frontend State (Svelte)

Use Svelte 5 runes for reactive state:

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  // Local component state
  let isLoading = $state(false);
  let error = $state<string | null>(null);

  // Derived state
  let canSubmit = $derived(!isLoading && formIsValid);

  async function handleScan() {
    isLoading = true;
    error = null;
    try {
      const result = await invoke('scan_broker', { brokerId: 'spokeo' });
      // handle result
    } catch (e) {
      error = e.message;
    } finally {
      isLoading = false;
    }
  }
</script>
```

### Global Stores

Place shared stores in `$lib/stores/`:

```typescript
// $lib/stores/vault.ts
import { writable, derived } from 'svelte/store';
import * as vaultApi from '$lib/api/vault';
import type { VaultInfo } from '$lib/types';

interface VaultState {
  /// Currently selected vault ID (user can switch between family members)
  currentVaultId: string | null;
  /// List of all available vaults
  availableVaults: VaultInfo[];
  /// Which vaults are currently unlocked
  unlockedVaultIds: Set<string>;
}

function createVaultStore() {
  const { subscribe, set, update } = writable<VaultState>({
    currentVaultId: null,
    availableVaults: [],
    unlockedVaultIds: new Set(),
  });

  return {
    subscribe,

    /// Load list of available vaults from disk
    async loadVaults() {
      const vaults = await vaultApi.listVaults();
      update(state => ({ ...state, availableVaults: vaults }));
    },

    /// Set the current active vault
    setCurrentVault(vaultId: string) {
      update(state => ({ ...state, currentVaultId: vaultId }));
    },

    /// Unlock a vault
    async unlock(vaultId: string, password: string) {
      await vaultApi.unlockVault(vaultId, password);
      update(state => ({
        ...state,
        currentVaultId: vaultId,
        unlockedVaultIds: new Set([...state.unlockedVaultIds, vaultId]),
      }));
    },

    /// Lock a vault
    async lock(vaultId: string) {
      await vaultApi.lockVault(vaultId);
      update(state => {
        const newUnlocked = new Set(state.unlockedVaultIds);
        newUnlocked.delete(vaultId);
        return {
          ...state,
          unlockedVaultIds: newUnlocked,
          // Clear current if we locked it
          currentVaultId: state.currentVaultId === vaultId ? null : state.currentVaultId,
        };
      });
    },
  };
}

export const vaultStore = createVaultStore();

// Derived stores for common checks
export const currentVaultId = derived(vaultStore, $v => $v.currentVaultId);
export const isCurrentVaultUnlocked = derived(
  vaultStore,
  $v => $v.currentVaultId !== null && $v.unlockedVaultIds.has($v.currentVaultId)
);
```

**Usage in components:**

```svelte
<script lang="ts">
  import { vaultStore, currentVaultId, isCurrentVaultUnlocked } from '$lib/stores/vault';
  import { scanBroker } from '$lib/api/broker';

  async function handleScan(brokerId: string) {
    if (!$currentVaultId) {
      throw new Error('No vault selected');
    }

    // currentVaultId is passed to all vault-requiring operations
    const result = await scanBroker($currentVaultId, brokerId);
  }
</script>
```

---

## 4. Testing

### Unit Tests

Place unit tests in the same file using `#[cfg(test)]`:

```rust
// src/vault/cipher.rs

pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
    // implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = [0u8; 32];
        let plaintext = b"sensitive data";

        let ciphertext = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &ciphertext).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn decrypt_fails_with_wrong_key() {
        let key1 = [0u8; 32];
        let key2 = [1u8; 32];
        let plaintext = b"sensitive data";

        let ciphertext = encrypt(&key1, plaintext).unwrap();
        let result = decrypt(&key2, &ciphertext);

        assert!(matches!(result, Err(CipherError::DecryptionFailed)));
    }
}
```

### Integration Tests

Place integration tests in `tests/` directory:

```rust
// crates/spectral-vault/tests/vault_integration.rs

use spectral_vault::{Vault, VaultConfig};
use tempfile::TempDir;

#[tokio::test]
async fn vault_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let config = VaultConfig {
        db_path: tmp.path().join("vault.db"),
        ..Default::default()
    };

    // Create and initialize
    let vault = Vault::create(&config, "password123").await.unwrap();

    // Store and retrieve
    vault.set_profile_field("email", "test@example.com").await.unwrap();
    let email = vault.get_profile_field("email").await.unwrap();
    assert_eq!(email, "test@example.com");

    // Lock and unlock
    drop(vault);
    let vault = Vault::open(&config, "password123").await.unwrap();
    let email = vault.get_profile_field("email").await.unwrap();
    assert_eq!(email, "test@example.com");
}
```

### Test Fixtures

Use builder patterns for complex test data:

```rust
// crates/spectral-broker/src/test_fixtures.rs

#[cfg(test)]
pub struct BrokerDefinitionBuilder {
    def: BrokerDefinition,
}

#[cfg(test)]
impl BrokerDefinitionBuilder {
    pub fn new(id: &str) -> Self {
        Self {
            def: BrokerDefinition {
                id: id.to_string(),
                name: id.to_string(),
                ..Default::default()
            },
        }
    }

    pub fn with_search_method(mut self, method: SearchMethod) -> Self {
        self.def.search_method = method;
        self
    }

    pub fn with_removal_method(mut self, method: RemovalMethod) -> Self {
        self.def.removal_method = method;
        self
    }

    pub fn build(self) -> BrokerDefinition {
        self.def
    }
}

// Usage in tests
#[test]
fn test_scan_with_url_template() {
    let broker = BrokerDefinitionBuilder::new("test-broker")
        .with_search_method(SearchMethod::UrlTemplate {
            template: "https://example.com/{first}-{last}".into(),
            requires_fields: vec![PiiField::FirstName, PiiField::LastName],
        })
        .build();

    // test with broker
}
```

### Frontend Testing

Use Vitest for unit tests, Playwright for E2E:

```typescript
// src/lib/components/BrokerCard.test.ts
import { render, screen } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import BrokerCard from './BrokerCard.svelte';

test('displays broker name and status', () => {
  render(BrokerCard, {
    props: {
      broker: { id: 'spokeo', name: 'Spokeo', status: 'found' },
    },
  });

  expect(screen.getByText('Spokeo')).toBeInTheDocument();
  expect(screen.getByText('Found')).toBeInTheDocument();
});

test('calls onRemove when button clicked', async () => {
  const onRemove = vi.fn();
  render(BrokerCard, {
    props: {
      broker: { id: 'spokeo', name: 'Spokeo', status: 'found' },
      onRemove,
    },
  });

  await screen.getByRole('button', { name: /remove/i }).click();
  expect(onRemove).toHaveBeenCalledWith('spokeo');
});
```

---

## 5. Logging & Tracing

### Tracing Setup

Use `tracing` for all logging (never `println!` or `log`):

```rust
use tracing::{debug, error, info, instrument, warn, Span};

// Instrument async functions
#[instrument(skip(self, password), fields(broker_id))]
pub async fn scan_broker(&self, broker_id: &str, password: &str) -> Result<ScanResult> {
    info!("starting broker scan");

    let result = self.do_scan(broker_id).await;

    match &result {
        Ok(r) if r.found => info!(listing_url = %r.listing_url.as_deref().unwrap_or(""), "listing found"),
        Ok(_) => info!("no listing found"),
        Err(e) => warn!(error = %e, "scan failed"),
    }

    result
}
```

### Log Levels

| Level | Use for |
|-------|---------|
| `error!` | Unrecoverable errors, failed operations |
| `warn!` | Recoverable issues, degraded functionality |
| `info!` | Significant state changes, operation milestones |
| `debug!` | Detailed operation flow, intermediate values |
| `trace!` | Very detailed debugging, raw data (never in production) |

### Span Structure

Create hierarchical spans for complex operations:

```rust
pub async fn scan_all_brokers(&self, broker_ids: &[String]) -> Vec<ScanResult> {
    let span = tracing::info_span!("scan_all", broker_count = broker_ids.len());
    let _guard = span.enter();

    let mut results = Vec::new();
    for broker_id in broker_ids {
        let broker_span = tracing::info_span!("scan_broker", %broker_id);
        let result = self.scan_broker(broker_id)
            .instrument(broker_span)
            .await;
        results.push(result);
    }

    info!(found_count = results.iter().filter(|r| r.found).count(), "scan complete");
    results
}
```

### PII Redaction

Never log PII. Use record IDs or hashed summaries:

```rust
// Bad: logs PII
info!("scanning for user {} at {}", user.email, user.address);

// Good: logs identifiers only
info!(user_id = %user.id, "starting scan");

// Good: logs hashed summary for debugging
let email_hash = hash_for_logging(&user.email);
debug!(email_hash = %email_hash, "checking email domain");
```

---

## 6. Configuration

### Config File Structure

Use TOML with clear sections:

```toml
# ~/.config/spectral/config.toml

[general]
auto_lock_minutes = 15
theme = "system"  # "light", "dark", "system"

[scanning]
concurrent_scans = 3
delay_between_scans_ms = 2000
respect_robots_txt = true

[browser]
headless = true
user_agent = "Mozilla/5.0 ..."
proxy = ""  # "socks5://127.0.0.1:1080"

[llm]
enabled = false
provider = "ollama"
model = "llama3.1:8b"

[notifications]
enabled = true
on_removal_confirmed = true
on_new_listing_found = true
```

### Config Loading Pattern

```rust
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub scanning: ScanningConfig,
    pub browser: BrowserConfig,
    pub llm: LlmConfig,
    pub notifications: NotificationConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            scanning: ScanningConfig::default(),
            browser: BrowserConfig::default(),
            llm: LlmConfig::default(),
            notifications: NotificationConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let dirs = ProjectDirs::from("com", "spectral", "spectral")
            .ok_or(ConfigError::NoConfigDir)?;

        let config_path = dirs.config_dir().join("config.toml");

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            Ok(toml::from_str(&contents)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let dirs = ProjectDirs::from("com", "spectral", "spectral")
            .ok_or(ConfigError::NoConfigDir)?;

        fs::create_dir_all(dirs.config_dir())?;
        let config_path = dirs.config_dir().join("config.toml");
        let contents = toml::to_string_pretty(self)?;
        fs::write(config_path, contents)?;
        Ok(())
    }
}
```

### Environment Overrides

Support environment variable overrides for CI/testing:

```rust
impl Config {
    pub fn load_with_env() -> Result<Self, ConfigError> {
        let mut config = Self::load()?;

        // Override from environment
        if let Ok(val) = std::env::var("SPECTRAL_LLM_ENABLED") {
            config.llm.enabled = val.parse().unwrap_or(false);
        }
        if let Ok(val) = std::env::var("SPECTRAL_HEADLESS") {
            config.browser.headless = val.parse().unwrap_or(true);
        }

        Ok(config)
    }
}
```

---

## 7. API Design

### Tauri Command Signatures

Commands should be async, return `Result`, and use structured types:

```rust
// Good: clear input/output types
#[tauri::command]
async fn scan_broker(
    state: State<'_, AppState>,
    broker_id: String,
    options: Option<ScanOptions>,
) -> Result<ScanResult, CommandError> {
    // ...
}

// Good: batch operations return vec
#[tauri::command]
async fn scan_brokers(
    state: State<'_, AppState>,
    broker_ids: Vec<String>,
) -> Result<Vec<ScanResult>, CommandError> {
    // ...
}

// Bad: too many parameters
#[tauri::command]
async fn scan_broker(
    state: State<'_, AppState>,
    broker_id: String,
    include_screenshot: bool,
    timeout_ms: u64,
    retry_count: u32,
) -> Result<ScanResult, CommandError> {
    // Use an options struct instead
}
```

### Vault Command Patterns

**All vault-related commands must include `vault_id` parameter for explicit security:**

```rust
// Vault lifecycle commands
#[tauri::command]
async fn vault_create(
    state: State<'_, AppState>,
    vault_id: String,
    display_name: String,
    password: String,
) -> Result<(), CommandError> {
    // Create new vault at vaults_dir/{vault_id}/
}

#[tauri::command]
async fn vault_unlock(
    state: State<'_, AppState>,
    vault_id: String,
    password: String,
) -> Result<(), CommandError> {
    // Derive key, open database, insert into unlocked_vaults
}

#[tauri::command]
async fn vault_lock(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<(), CommandError> {
    // Remove from unlocked_vaults, key auto-zeroized
}

#[tauri::command]
async fn vault_status(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<VaultStatus, CommandError> {
    // Return { exists: bool, unlocked: bool }
}

#[tauri::command]
async fn list_vaults(
    state: State<'_, AppState>,
) -> Result<Vec<VaultInfo>, CommandError> {
    // Scan vaults_dir, read metadata.json from each
}

// Profile operations - also take vault_id
#[tauri::command]
async fn create_profile(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<String, CommandError> {
    let vaults = state.unlocked_vaults.read().await;
    let vault = vaults.get(&vault_id)
        .ok_or(CommandError::vault_locked())?;

    let profile_id = vault.create_profile().await?;
    Ok(profile_id)
}
```

**Security rationale:**
- Explicit `vault_id` prevents confused deputy attacks
- No ambient authority (no "active vault" state)
- Clear audit trail of which vault each operation targets
- Frontend maintains current vault ID in local state

### Frontend API Wrapper

Wrap Tauri commands in `$lib/api/`:

```typescript
// $lib/api/vault.ts
import { invoke } from '@tauri-apps/api/core';
import type { VaultStatus, VaultInfo } from '$lib/types';

export async function createVault(
  vaultId: string,
  displayName: string,
  password: string
): Promise<void> {
  return invoke('vault_create', { vaultId, displayName, password });
}

export async function unlockVault(
  vaultId: string,
  password: string
): Promise<void> {
  return invoke('vault_unlock', { vaultId, password });
}

export async function lockVault(vaultId: string): Promise<void> {
  return invoke('vault_lock', { vaultId });
}

export async function getVaultStatus(vaultId: string): Promise<VaultStatus> {
  return invoke('vault_status', { vaultId });
}

export async function listVaults(): Promise<VaultInfo[]> {
  return invoke('list_vaults');
}

// $lib/api/broker.ts
import { invoke } from '@tauri-apps/api/core';
import type { ScanResult, ScanOptions, BrokerDefinition } from '$lib/types';

export async function scanBroker(
  vaultId: string,
  brokerId: string,
  options?: ScanOptions
): Promise<ScanResult> {
  return invoke('scan_broker', { vaultId, brokerId, options });
}

export async function scanBrokers(brokerIds: string[]): Promise<ScanResult[]> {
  return invoke('scan_brokers', { brokerIds });
}

export async function getBrokerDefinitions(): Promise<BrokerDefinition[]> {
  return invoke('get_broker_definitions');
}

export async function submitRemoval(brokerId: string): Promise<void> {
  return invoke('submit_removal', { brokerId });
}
```

### Pagination

Use cursor-based pagination for large result sets:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct PageRequest {
    pub cursor: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Serialize)]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[tauri::command]
async fn list_scan_results(
    state: State<'_, AppState>,
    page: PageRequest,
) -> Result<PageResponse<ScanResult>, CommandError> {
    let limit = page.limit.min(100).max(1);
    let results = state.db.query_scan_results(page.cursor, limit + 1).await?;

    let has_more = results.len() > limit as usize;
    let items: Vec<_> = results.into_iter().take(limit as usize).collect();
    let next_cursor = if has_more {
        items.last().map(|r| r.id.to_string())
    } else {
        None
    };

    Ok(PageResponse { items, next_cursor, has_more })
}
```

---

## 8. Frontend Components

### Component Structure

Use shadcn-svelte components as base, extend as needed:

```
src/lib/components/
├── ui/                    # shadcn-svelte base components
│   ├── button/
│   ├── card/
│   └── input/
├── broker/                # Domain-specific components
│   ├── BrokerCard.svelte
│   ├── BrokerList.svelte
│   └── ScanProgress.svelte
├── vault/
│   ├── UnlockForm.svelte
│   └── ProfileEditor.svelte
└── layout/
    ├── Sidebar.svelte
    └── Header.svelte
```

### Component Props

Use TypeScript for prop definitions:

```svelte
<script lang="ts">
  import type { BrokerResult } from '$lib/types';
  import { Button } from '$lib/components/ui/button';
  import { Card } from '$lib/components/ui/card';

  interface Props {
    broker: BrokerResult;
    onRemove?: (id: string) => void;
    onRescan?: (id: string) => void;
    disabled?: boolean;
  }

  let { broker, onRemove, onRescan, disabled = false }: Props = $props();

  function handleRemove() {
    onRemove?.(broker.id);
  }
</script>

<Card class="p-4">
  <h3 class="font-semibold">{broker.name}</h3>
  <p class="text-sm text-muted-foreground">{broker.status}</p>

  <div class="mt-4 flex gap-2">
    <Button onclick={handleRemove} {disabled}>Remove</Button>
    <Button variant="outline" onclick={() => onRescan?.(broker.id)} {disabled}>
      Rescan
    </Button>
  </div>
</Card>
```

### Loading States

Use consistent loading patterns:

```svelte
<script lang="ts">
  import { Skeleton } from '$lib/components/ui/skeleton';

  interface Props {
    isLoading: boolean;
    error: string | null;
    data: BrokerResult[] | null;
  }

  let { isLoading, error, data }: Props = $props();
</script>

{#if isLoading}
  <div class="space-y-4">
    {#each Array(3) as _}
      <Skeleton class="h-24 w-full" />
    {/each}
  </div>
{:else if error}
  <div class="rounded-md bg-destructive/10 p-4 text-destructive">
    <p class="font-medium">Error loading brokers</p>
    <p class="text-sm">{error}</p>
  </div>
{:else if data && data.length > 0}
  {#each data as broker}
    <BrokerCard {broker} />
  {/each}
{:else}
  <p class="text-muted-foreground">No brokers found.</p>
{/if}
```

### Form Patterns

Use controlled forms with validation:

```svelte
<script lang="ts">
  import { z } from 'zod';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';

  const schema = z.object({
    email: z.string().email('Invalid email address'),
    firstName: z.string().min(1, 'First name is required'),
    lastName: z.string().min(1, 'Last name is required'),
  });

  type FormData = z.infer<typeof schema>;

  let formData = $state<FormData>({
    email: '',
    firstName: '',
    lastName: '',
  });

  let errors = $state<Partial<Record<keyof FormData, string>>>({});
  let isSubmitting = $state(false);

  function validate(): boolean {
    const result = schema.safeParse(formData);
    if (!result.success) {
      errors = result.error.flatten().fieldErrors as typeof errors;
      return false;
    }
    errors = {};
    return true;
  }

  async function handleSubmit() {
    if (!validate()) return;

    isSubmitting = true;
    try {
      await invoke('save_profile', { profile: formData });
    } finally {
      isSubmitting = false;
    }
  }
</script>

<form onsubmit|preventDefault={handleSubmit} class="space-y-4">
  <div>
    <Label for="email">Email</Label>
    <Input id="email" type="email" bind:value={formData.email} />
    {#if errors.email}<p class="text-sm text-destructive">{errors.email}</p>{/if}
  </div>

  <div>
    <Label for="firstName">First Name</Label>
    <Input id="firstName" bind:value={formData.firstName} />
    {#if errors.firstName}<p class="text-sm text-destructive">{errors.firstName}</p>{/if}
  </div>

  <Button type="submit" disabled={isSubmitting}>
    {isSubmitting ? 'Saving...' : 'Save Profile'}
  </Button>
</form>
```

---

## 9. Security Patterns

### PII Handling

Always use `Zeroizing` wrapper for sensitive data:

```rust
use zeroize::Zeroizing;

pub struct VaultCipher {
    key: Zeroizing<[u8; 32]>,
}

impl VaultCipher {
    pub fn new(password: &str) -> Result<Self, CipherError> {
        let key = Zeroizing::new(derive_key(password)?);
        Ok(Self { key })
    }
}

// Key is automatically zeroed when dropped
```

### Input Sanitization

Sanitize all external input before use:

```rust
/// Sanitize user input for safe use in queries/display
pub fn sanitize_input(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_control())
        .take(1000)  // reasonable length limit
        .collect()
}

/// Sanitize external content before LLM exposure
pub fn sanitize_for_llm(content: &str) -> String {
    // Remove potential prompt injection patterns
    let content = remove_prompt_injections(content);
    // Limit length
    let content = truncate_with_notice(&content, 4000);
    // Escape any remaining special sequences
    escape_special_tokens(&content)
}
```

### Never Trust External Data

```rust
// Bad: trusting broker response
let email = broker_response.email;  // could be anything

// Good: validate and sanitize
let email = broker_response.email
    .filter(|e| is_valid_email(e))
    .map(|e| sanitize_input(&e));
```

### Encryption Patterns

```rust
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use chacha20poly1305::aead::{Aead, NewAead};
use rand::Rng;

pub fn encrypt_field(key: &[u8; 32], plaintext: &[u8]) -> Result<EncryptedField, CipherError> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, plaintext)
        .map_err(|_| CipherError::EncryptionFailed)?;

    Ok(EncryptedField {
        ciphertext,
        nonce: nonce_bytes,
    })
}
```

---

## 10. Database Patterns

### SQLx Query Patterns

Always use compile-time checked queries:

```rust
// Good: compile-time checked
let results = sqlx::query_as!(
    BrokerResult,
    r#"
    SELECT id, broker_id, status as "status: BrokerStatus", found_url, scanned_at
    FROM broker_results
    WHERE user_id = ?
    ORDER BY scanned_at DESC
    LIMIT ?
    "#,
    user_id,
    limit
)
.fetch_all(&self.pool)
.await?;

// Bad: runtime query building
let query = format!("SELECT * FROM broker_results WHERE user_id = '{}'", user_id);
```

### Migrations

Place migrations in `crates/spectral-db/migrations/`:

```sql
-- migrations/001_initial.sql

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE broker_results (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    broker_id TEXT NOT NULL,
    status TEXT NOT NULL,
    found_url TEXT,
    scanned_at TEXT NOT NULL,
    UNIQUE(user_id, broker_id)
);

CREATE INDEX idx_broker_results_user ON broker_results(user_id);
CREATE INDEX idx_broker_results_status ON broker_results(status);
```

### Transaction Patterns

```rust
pub async fn submit_removal(&self, broker_result_id: &str) -> Result<(), DbError> {
    let mut tx = self.pool.begin().await?;

    // Update result status
    sqlx::query!(
        "UPDATE broker_results SET status = 'removal_pending' WHERE id = ?",
        broker_result_id
    )
    .execute(&mut *tx)
    .await?;

    // Create removal record
    let removal_id = Uuid::new_v4().to_string();
    sqlx::query!(
        "INSERT INTO removal_requests (id, broker_result_id, submitted_at) VALUES (?, ?, datetime('now'))",
        removal_id,
        broker_result_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
```

---

## Quick Reference

### Do

- Use `thiserror` for library errors, `anyhow` for app shell
- Use `tracing` for all logging
- Use `Zeroizing` for sensitive data
- Use `sqlx::query!` for compile-time checked queries
- Use `CancellationToken` for cancellable operations
- Use Svelte 5 runes (`$state`, `$derived`, `$effect`)
- Wrap Tauri commands in `$lib/api/`
- Use shadcn-svelte components as base

### Don't

- Use `.unwrap()` in production code
- Log PII (use IDs or hashed summaries)
- Use `println!` or `log` crate
- Build SQL queries with string formatting
- Store API keys in config files (use encrypted vault)
- Trust external data without validation
- Skip cancellation checks in long operations

---

## 11. Authentication

Spectral has a layered authentication model. Since it's a local-first desktop app, there's no server-side auth, but we still need to protect sensitive data.

### 11.1 Authentication Layers

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 1: App Launch Authentication                         │
│  ────────────────────────────────────────────────────────── │
│  Required to open the app at all                            │
│  Options: PIN, Biometrics, Password                         │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  Layer 2: Vault Unlock (Master Password)                    │
│  ────────────────────────────────────────────────────────── │
│  Required to access encrypted PII                           │
│  Argon2id KDF → ChaCha20-Poly1305 encryption               │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  Layer 3: Credential Storage (in Vault)                     │
│  ────────────────────────────────────────────────────────── │
│  LLM API keys, Email credentials, OAuth tokens              │
│  All encrypted at rest with vault key                       │
└─────────────────────────────────────────────────────────────┘
```

### 11.2 App Launch Authentication

Users must authenticate before accessing the app. This prevents casual access on shared computers.

```rust
// crates/spectral-auth/src/lib.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppAuthMethod {
    /// Simple PIN (4-8 digits)
    Pin,
    /// Full password (separate from vault password)
    Password,
    /// OS-level biometrics (Windows Hello, Touch ID, etc.)
    Biometric,
    /// Hardware security key (FIDO2/WebAuthn)
    SecurityKey,
    /// No app-level auth (rely on OS login + vault password)
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppAuthConfig {
    pub method: AppAuthMethod,
    /// Max failed attempts before lockout
    pub max_attempts: u32,
    /// Lockout duration in seconds
    pub lockout_duration_secs: u64,
    /// Require re-auth after this many minutes of inactivity
    pub timeout_minutes: u32,
}

impl Default for AppAuthConfig {
    fn default() -> Self {
        Self {
            method: AppAuthMethod::Pin,
            max_attempts: 5,
            lockout_duration_secs: 300, // 5 minutes
            timeout_minutes: 15,
        }
    }
}
```

**Implementation approaches:**

```rust
// PIN authentication
pub struct PinAuth {
    /// PIN hash (Argon2id)
    pin_hash: [u8; 32],
    salt: [u8; 16],
    failed_attempts: AtomicU32,
    locked_until: RwLock<Option<Instant>>,
}

impl PinAuth {
    pub fn verify(&self, pin: &str) -> Result<(), AuthError> {
        // Check lockout
        if let Some(until) = *self.locked_until.read().unwrap() {
            if Instant::now() < until {
                return Err(AuthError::LockedOut {
                    remaining_secs: (until - Instant::now()).as_secs(),
                });
            }
        }

        // Verify PIN
        let hash = argon2_hash(pin.as_bytes(), &self.salt)?;
        if constant_time_eq(&hash, &self.pin_hash) {
            self.failed_attempts.store(0, Ordering::SeqCst);
            Ok(())
        } else {
            let attempts = self.failed_attempts.fetch_add(1, Ordering::SeqCst) + 1;
            if attempts >= MAX_ATTEMPTS {
                *self.locked_until.write().unwrap() = Some(Instant::now() + LOCKOUT_DURATION);
            }
            Err(AuthError::InvalidPin { attempts_remaining: MAX_ATTEMPTS - attempts })
        }
    }
}

// Biometric authentication via OS APIs
// crates/spectral-auth/src/biometric.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BiometricError {
    #[error("biometric authentication not available")]
    NotAvailable,
    #[error("biometric authentication canceled")]
    Canceled,
    #[error("biometric authentication failed")]
    Failed,
    #[error("biometric not enrolled")]
    NotEnrolled,
    #[error("platform error: {0}")]
    Platform(String),
}

/// Check if biometric authentication is available on this system
pub fn is_biometric_available() -> bool {
    #[cfg(target_os = "windows")]
    { windows_biometric::is_available() }

    #[cfg(target_os = "macos")]
    { macos_biometric::is_available() }

    #[cfg(target_os = "linux")]
    { linux_biometric::is_available() }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    { false }
}

/// Verify user via biometric authentication
pub async fn verify_biometric(reason: &str) -> Result<(), BiometricError> {
    #[cfg(target_os = "windows")]
    { windows_biometric::verify(reason).await }

    #[cfg(target_os = "macos")]
    { macos_biometric::verify(reason).await }

    #[cfg(target_os = "linux")]
    { linux_biometric::verify(reason).await }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    { Err(BiometricError::NotAvailable) }
}

// =============================================================================
// Windows: Windows Hello (face, fingerprint, PIN)
// =============================================================================
#[cfg(target_os = "windows")]
mod windows_biometric {
    use super::*;
    use windows::Security::Credentials::UI::*;

    pub fn is_available() -> bool {
        UserConsentVerifier::CheckAvailabilityAsync()
            .and_then(|op| op.get())
            .map(|availability| availability == UserConsentVerifierAvailability::Available)
            .unwrap_or(false)
    }

    pub async fn verify(reason: &str) -> Result<(), BiometricError> {
        let result = UserConsentVerifier::RequestVerificationAsync(&reason.into())
            .map_err(|e| BiometricError::Platform(e.to_string()))?
            .await
            .map_err(|e| BiometricError::Platform(e.to_string()))?;

        match result {
            UserConsentVerificationResult::Verified => Ok(()),
            UserConsentVerificationResult::Canceled => Err(BiometricError::Canceled),
            UserConsentVerificationResult::DeviceNotPresent => Err(BiometricError::NotAvailable),
            UserConsentVerificationResult::NotConfiguredForUser => Err(BiometricError::NotEnrolled),
            _ => Err(BiometricError::Failed),
        }
    }
}

// =============================================================================
// macOS: Touch ID via LocalAuthentication framework
// =============================================================================
#[cfg(target_os = "macos")]
mod macos_biometric {
    use super::*;
    use objc2::rc::Id;
    use objc2::runtime::Bool;
    use objc2::{class, msg_send, msg_send_id};
    use objc2_foundation::{NSError, NSString};
    use std::ptr;
    use tokio::sync::oneshot;

    // LAContext from LocalAuthentication framework
    pub fn is_available() -> bool {
        unsafe {
            let context: Id<objc2::runtime::AnyObject> =
                msg_send_id![class!(LAContext), new];

            let mut error: *mut NSError = ptr::null_mut();
            let policy: i64 = 1; // LAPolicyDeviceOwnerAuthenticationWithBiometrics

            let available: Bool = msg_send![
                &*context,
                canEvaluatePolicy: policy
                error: &mut error
            ];

            available.as_bool()
        }
    }

    pub async fn verify(reason: &str) -> Result<(), BiometricError> {
        let reason = reason.to_string();
        let (tx, rx) = oneshot::channel();

        // Must run on main thread for UI
        std::thread::spawn(move || {
            unsafe {
                let context: Id<objc2::runtime::AnyObject> =
                    msg_send_id![class!(LAContext), new];

                let reason_ns = NSString::from_str(&reason);
                let policy: i64 = 1; // LAPolicyDeviceOwnerAuthenticationWithBiometrics

                // Create block for callback
                let tx = std::sync::Mutex::new(Some(tx));
                let block = block2::ConcreteBlock::new(move |success: Bool, error: *mut NSError| {
                    let result = if success.as_bool() {
                        Ok(())
                    } else if error.is_null() {
                        Err(BiometricError::Canceled)
                    } else {
                        let code: i64 = msg_send![error, code];
                        match code {
                            -2 => Err(BiometricError::Canceled),      // LAErrorUserCancel
                            -5 => Err(BiometricError::NotAvailable), // LAErrorPasscodeNotSet
                            -6 => Err(BiometricError::NotAvailable), // LAErrorTouchIDNotAvailable
                            -7 => Err(BiometricError::NotEnrolled),  // LAErrorTouchIDNotEnrolled
                            _ => Err(BiometricError::Failed),
                        }
                    };
                    if let Some(tx) = tx.lock().unwrap().take() {
                        tx.send(result).ok();
                    }
                });

                let _: () = msg_send![
                    &*context,
                    evaluatePolicy: policy
                    localizedReason: &*reason_ns
                    reply: &*block.copy()
                ];
            }
        });

        rx.await.map_err(|_| BiometricError::Failed)?
    }
}

// =============================================================================
// Linux: fprintd (fingerprint) or polkit (system auth prompt)
// =============================================================================
#[cfg(target_os = "linux")]
mod linux_biometric {
    use super::*;
    use std::process::Command;
    use zbus::blocking::Connection;

    /// Check if fprintd is available and user has enrolled fingerprints
    pub fn is_available() -> bool {
        // Check for fprintd
        if is_fprintd_available() {
            return true;
        }

        // Fallback: check for polkit (system auth dialog)
        is_polkit_available()
    }

    fn is_fprintd_available() -> bool {
        // Check if fprintd service is running via D-Bus
        if let Ok(conn) = Connection::system() {
            let result: Result<bool, _> = conn.call_method(
                Some("net.reactivated.Fprint"),
                "/net/reactivated/Fprint/Manager",
                Some("net.reactivated.Fprint.Manager"),
                "GetDefaultDevice",
                &(),
            ).and_then(|reply| reply.body().deserialize());

            return result.is_ok();
        }
        false
    }

    fn is_polkit_available() -> bool {
        // Check if pkcheck is available
        Command::new("pkcheck")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub async fn verify(reason: &str) -> Result<(), BiometricError> {
        // Try fprintd first
        if is_fprintd_available() {
            return verify_fprintd().await;
        }

        // Fallback to polkit
        verify_polkit(reason).await
    }

    /// Verify via fprintd (fingerprint daemon)
    async fn verify_fprintd() -> Result<(), BiometricError> {
        // Use fprintd-verify command or D-Bus API
        let output = tokio::process::Command::new("fprintd-verify")
            .output()
            .await
            .map_err(|e| BiometricError::Platform(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("canceled") || stderr.contains("user cancel") {
                Err(BiometricError::Canceled)
            } else if stderr.contains("no enrolled") {
                Err(BiometricError::NotEnrolled)
            } else {
                Err(BiometricError::Failed)
            }
        }
    }

    /// Verify via polkit (shows system auth dialog)
    async fn verify_polkit(reason: &str) -> Result<(), BiometricError> {
        // Use pkexec with a no-op action to prompt for auth
        // Or use zenity/kdialog for password prompt + PAM verification

        // Option 1: Use polkit agent via D-Bus
        let conn = zbus::Connection::system()
            .await
            .map_err(|e| BiometricError::Platform(e.to_string()))?;

        // Create authentication agent request
        // This will show the system authentication dialog

        // Option 2: Use zenity/kdialog for graphical password prompt
        let password = show_password_dialog(reason).await?;
        verify_password_pam(&password).await
    }

    async fn show_password_dialog(reason: &str) -> Result<String, BiometricError> {
        // Try zenity (GTK)
        let output = tokio::process::Command::new("zenity")
            .args(["--password", "--title=Spectral Authentication", "--text", reason])
            .output()
            .await;

        if let Ok(output) = output {
            if output.status.success() {
                return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
            } else if output.status.code() == Some(1) {
                return Err(BiometricError::Canceled);
            }
        }

        // Try kdialog (KDE)
        let output = tokio::process::Command::new("kdialog")
            .args(["--password", reason, "--title", "Spectral Authentication"])
            .output()
            .await;

        if let Ok(output) = output {
            if output.status.success() {
                return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
            } else if output.status.code() == Some(1) {
                return Err(BiometricError::Canceled);
            }
        }

        Err(BiometricError::NotAvailable)
    }

    /// Verify password against PAM
    async fn verify_password_pam(password: &str) -> Result<(), BiometricError> {
        use pam::Authenticator;

        let username = std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .map_err(|_| BiometricError::Platform("cannot determine username".into()))?;

        // Run PAM auth in blocking task (pam crate is sync)
        let password = password.to_string();
        tokio::task::spawn_blocking(move || {
            let mut auth = Authenticator::with_password("spectral")
                .map_err(|e| BiometricError::Platform(e.to_string()))?;

            auth.get_handler()
                .set_credentials(&username, &password);

            auth.authenticate()
                .map_err(|_| BiometricError::Failed)?;

            Ok(())
        })
        .await
        .map_err(|e| BiometricError::Platform(e.to_string()))?
    }
}
```

### 11.3 Vault Authentication (Master Password)

The vault password is the primary protection for all PII. It's separate from the app launch PIN.

```rust
// crates/spectral-vault/src/auth.rs

pub struct VaultAuth {
    config: Argon2Config,
}

#[derive(Debug, Clone)]
pub struct Argon2Config {
    /// Memory cost in KB (default: 256MB = 262144 KB)
    pub memory_cost: u32,
    /// Time cost (iterations, default: 4)
    pub time_cost: u32,
    /// Parallelism (default: 4)
    pub parallelism: u32,
    /// Output length (default: 32 bytes)
    pub output_len: usize,
}

impl Default for Argon2Config {
    fn default() -> Self {
        Self {
            memory_cost: 262144,  // 256 MB
            time_cost: 4,
            parallelism: 4,
            output_len: 32,
        }
    }
}

impl VaultAuth {
    /// Derive the master key from password
    pub fn derive_key(&self, password: &str, salt: &[u8; 16]) -> Result<Zeroizing<[u8; 32]>, AuthError> {
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(
                self.config.memory_cost,
                self.config.time_cost,
                self.config.parallelism,
                Some(self.config.output_len),
            )?,
        );

        let mut key = Zeroizing::new([0u8; 32]);
        argon2.hash_password_into(password.as_bytes(), salt, key.as_mut())?;
        Ok(key)
    }

    /// Verify password by attempting to decrypt a known value
    pub fn verify(&self, password: &str, salt: &[u8; 16], verification_blob: &[u8]) -> Result<Zeroizing<[u8; 32]>, AuthError> {
        let key = self.derive_key(password, salt)?;

        // Try to decrypt the verification blob
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&*key));
        cipher.decrypt(/* ... */)
            .map_err(|_| AuthError::InvalidPassword)?;

        Ok(key)
    }
}
```

**Session management:**

```rust
pub struct VaultSession {
    /// The derived master key (held in memory while unlocked)
    key: Zeroizing<[u8; 32]>,
    /// When the session was created
    unlocked_at: Instant,
    /// Last activity timestamp
    last_activity: AtomicU64,
    /// Auto-lock timeout
    timeout: Duration,
}

impl VaultSession {
    pub fn touch(&self) {
        self.last_activity.store(
            Instant::now().elapsed().as_secs(),
            Ordering::SeqCst,
        );
    }

    pub fn is_expired(&self) -> bool {
        let last = Duration::from_secs(self.last_activity.load(Ordering::SeqCst));
        Instant::now().duration_since(self.unlocked_at) - last > self.timeout
    }

    /// Check expiry and lock if needed
    pub fn check_and_lock(&mut self) -> bool {
        if self.is_expired() {
            self.lock();
            true
        } else {
            false
        }
    }

    pub fn lock(&mut self) {
        // Key is automatically zeroized when dropped
        self.key = Zeroizing::new([0u8; 32]);
    }
}
```

### 11.4 External API Credentials

API keys (LLM providers, etc.) are stored encrypted in the vault:

```rust
// crates/spectral-vault/src/credentials.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialType {
    LlmApiKey { provider: String },
    SmtpCredential { host: String },
    ImapCredential { host: String },
    OAuthToken { provider: String },
    WebhookSecret { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCredential {
    pub id: String,
    pub credential_type: CredentialType,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    // The actual secret is stored encrypted, not in this struct
}

impl Vault {
    /// Store a credential (encrypted with vault key)
    pub async fn store_credential(
        &self,
        credential_type: CredentialType,
        secret: &str,
    ) -> Result<String, VaultError> {
        let id = Uuid::new_v4().to_string();

        // Encrypt the secret
        let encrypted = self.cipher.encrypt(secret.as_bytes())?;

        // Store metadata and encrypted secret
        sqlx::query!(
            r#"
            INSERT INTO credentials (id, type, encrypted_secret, created_at)
            VALUES (?, ?, ?, datetime('now'))
            "#,
            id,
            serde_json::to_string(&credential_type)?,
            encrypted,
        )
        .execute(&self.db)
        .await?;

        Ok(id)
    }

    /// Retrieve and decrypt a credential
    pub async fn get_credential(&self, id: &str) -> Result<Zeroizing<String>, VaultError> {
        let row = sqlx::query!(
            "SELECT encrypted_secret FROM credentials WHERE id = ?",
            id
        )
        .fetch_one(&self.db)
        .await?;

        let decrypted = self.cipher.decrypt(&row.encrypted_secret)?;
        let secret = String::from_utf8(decrypted)
            .map_err(|_| VaultError::CorruptedData)?;

        // Update last_used
        sqlx::query!("UPDATE credentials SET last_used = datetime('now') WHERE id = ?", id)
            .execute(&self.db)
            .await?;

        Ok(Zeroizing::new(secret))
    }
}
```

### 11.5 Email Authentication

Support both password-based (SMTP/IMAP) and OAuth2 (Gmail, Outlook):

```rust
// crates/spectral-mail/src/auth.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailAuthMethod {
    /// Traditional username/password (App Password for Gmail)
    Password {
        username: String,
        // Password stored in vault credentials
        credential_id: String,
    },
    /// OAuth2 (Gmail, Outlook, etc.)
    OAuth2 {
        provider: OAuthProvider,
        // Tokens stored in vault credentials
        access_token_id: String,
        refresh_token_id: String,
        expires_at: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OAuthProvider {
    Google,
    Microsoft,
    Custom {
        auth_url: String,
        token_url: String,
        client_id: String,
        // client_secret in vault
        client_secret_id: String,
    },
}

impl EmailAuthMethod {
    /// Get credentials for SMTP/IMAP connection
    pub async fn get_credentials(&self, vault: &Vault) -> Result<EmailCredentials, MailError> {
        match self {
            Self::Password { username, credential_id } => {
                let password = vault.get_credential(credential_id).await?;
                Ok(EmailCredentials::Password {
                    username: username.clone(),
                    password,
                })
            }
            Self::OAuth2 { access_token_id, expires_at, .. } => {
                // Check if token needs refresh
                if Utc::now() > *expires_at {
                    self.refresh_token(vault).await?;
                }
                let token = vault.get_credential(access_token_id).await?;
                Ok(EmailCredentials::OAuth2 { token })
            }
        }
    }

    /// Refresh OAuth2 token
    async fn refresh_token(&self, vault: &Vault) -> Result<(), MailError> {
        if let Self::OAuth2 { provider, refresh_token_id, .. } = self {
            let refresh_token = vault.get_credential(refresh_token_id).await?;

            let (new_access, new_refresh, expires_in) = match provider {
                OAuthProvider::Google => refresh_google_token(&refresh_token).await?,
                OAuthProvider::Microsoft => refresh_microsoft_token(&refresh_token).await?,
                OAuthProvider::Custom { token_url, client_id, client_secret_id } => {
                    let client_secret = vault.get_credential(client_secret_id).await?;
                    refresh_custom_token(token_url, client_id, &client_secret, &refresh_token).await?
                }
            };

            // Update stored tokens
            // ...
        }
        Ok(())
    }
}
```

### 11.6 OAuth2 Flow (Desktop App)

For desktop apps, use PKCE (Proof Key for Code Exchange):

```rust
use oauth2::{
    AuthorizationCode, AuthUrl, ClientId, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenUrl,
    basic::BasicClient,
};

pub struct OAuthFlow {
    client: BasicClient,
    pkce_verifier: Option<PkceCodeVerifier>,
}

impl OAuthFlow {
    pub fn new_google() -> Self {
        let client = BasicClient::new(
            ClientId::new(GOOGLE_CLIENT_ID.to_string()),
            None,  // No client secret for public clients
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap(),
            Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap()),
        )
        .set_redirect_uri(
            // Use localhost redirect for desktop apps
            RedirectUrl::new("http://127.0.0.1:9876/callback".to_string()).unwrap()
        );

        Self { client, pkce_verifier: None }
    }

    /// Generate the authorization URL
    pub fn get_auth_url(&mut self, scopes: &[&str]) -> String {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        self.pkce_verifier = Some(pkce_verifier);

        let mut auth_request = self.client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge);

        for scope in scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.to_string()));
        }

        let (url, _csrf) = auth_request.url();
        url.to_string()
    }

    /// Exchange the authorization code for tokens
    pub async fn exchange_code(&self, code: &str) -> Result<TokenResponse, OAuthError> {
        let verifier = self.pkce_verifier.as_ref()
            .ok_or(OAuthError::NoPkceVerifier)?;

        let token = self.client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(verifier.clone())
            .request_async(oauth2::reqwest::async_http_client)
            .await?;

        Ok(TokenResponse {
            access_token: token.access_token().secret().clone(),
            refresh_token: token.refresh_token().map(|t| t.secret().clone()),
            expires_in: token.expires_in(),
        })
    }
}

/// Start local server to receive OAuth callback
pub async fn start_oauth_callback_server() -> Result<String, OAuthError> {
    use axum::{routing::get, Router, extract::Query};
    use tokio::sync::oneshot;

    let (tx, rx) = oneshot::channel();
    let tx = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));

    let app = Router::new().route("/callback", get(move |Query(params): Query<CallbackParams>| {
        let tx = tx.clone();
        async move {
            if let Some(tx) = tx.lock().unwrap().take() {
                tx.send(params.code).ok();
            }
            "Authorization successful! You can close this window."
        }
    }));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:9876").await?;
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    // Wait for callback
    let code = rx.await?;
    Ok(code)
}
```

### 11.7 Frontend Authentication Flow

```svelte
<!-- src/routes/+layout.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { appAuth, vault } from '$lib/stores/auth';

  let isInitialized = $state(false);

  onMount(async () => {
    // Check if app auth is required
    const authRequired = await invoke('is_app_auth_required');

    if (authRequired && !$appAuth.isAuthenticated) {
      goto('/auth/app');
      return;
    }

    // Check vault status
    const vaultStatus = await invoke('get_vault_status');
    if (vaultStatus === 'locked') {
      goto('/auth/vault');
      return;
    }

    isInitialized = true;
  });
</script>

{#if isInitialized}
  <slot />
{:else}
  <div class="flex items-center justify-center h-screen">
    <p>Loading...</p>
  </div>
{/if}
```

```svelte
<!-- src/routes/auth/app/+page.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { goto } from '$app/navigation';
  import { PinInput } from '$lib/components/ui/pin-input';

  let pin = $state('');
  let error = $state<string | null>(null);
  let attemptsRemaining = $state<number | null>(null);

  async function handleSubmit() {
    error = null;
    try {
      await invoke('verify_app_auth', { pin });
      goto('/auth/vault');
    } catch (e: any) {
      error = e.message;
      attemptsRemaining = e.attempts_remaining;
    }
  }

  async function handleBiometric() {
    try {
      await invoke('verify_biometric');
      goto('/auth/vault');
    } catch (e: any) {
      error = e.message;
    }
  }
</script>

<div class="flex flex-col items-center justify-center min-h-screen p-4">
  <h1 class="text-2xl font-bold mb-8">Welcome to Spectral</h1>

  <form onsubmit|preventDefault={handleSubmit} class="space-y-4 w-full max-w-sm">
    <PinInput bind:value={pin} length={6} />

    {#if error}
      <p class="text-destructive text-sm">{error}</p>
      {#if attemptsRemaining !== null}
        <p class="text-muted-foreground text-sm">
          {attemptsRemaining} attempts remaining
        </p>
      {/if}
    {/if}

    <Button type="submit" class="w-full">Unlock</Button>

    <div class="relative">
      <div class="absolute inset-0 flex items-center">
        <span class="w-full border-t" />
      </div>
      <div class="relative flex justify-center text-xs uppercase">
        <span class="bg-background px-2 text-muted-foreground">Or</span>
      </div>
    </div>

    <Button variant="outline" class="w-full" onclick={handleBiometric}>
      Use Biometrics
    </Button>
  </form>
</div>
```

### 11.8 Security Considerations

| Concern | Mitigation |
|---------|-----------|
| PIN brute force | Lockout after 5 failed attempts (5 min) |
| Vault password brute force | Argon2id with high memory cost (256MB) |
| Session hijacking | Auto-lock after inactivity, zeroize keys |
| Credential theft | All secrets encrypted in vault, never in config files |
| OAuth token theft | Tokens encrypted at rest, refresh on expiry |
| Clipboard snooping | Clear clipboard after pasting passwords |
| Memory dumping | Use `Zeroizing` wrapper, minimize PII residence time |
| Keylogging | Support biometric auth, hardware keys |

### 11.9 Recommended Defaults

```toml
# config.toml

[auth.app]
method = "pin"              # "pin", "password", "biometric", "none"
max_attempts = 5
lockout_duration_secs = 300
timeout_minutes = 15

[auth.vault]
argon2_memory_kb = 262144   # 256 MB
argon2_iterations = 4
argon2_parallelism = 4
auto_lock_minutes = 15

[auth.session]
require_reauth_for_sensitive = true  # Re-auth for PII export, credential view
```
