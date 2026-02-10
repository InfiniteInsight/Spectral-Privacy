# Vault Commands Design - Task 1.4

**Date:** 2024-02-10
**Status:** Approved
**Dependencies:** Task 1.3 (spectral-vault) ✅

## Overview

Design for exposing vault operations to the SvelteKit frontend via Tauri IPC commands. This creates the bridge between the Rust backend (spectral-vault crate) and the frontend, enabling vault creation, unlocking, locking, and status queries with multi-user support.

## Architecture Decisions

### Multi-Vault Support

**Decision:** Support multiple vaults from the start for family use cases.

**Implementation:**
- Each vault stored in `~/.local/share/spectral/vaults/{vault-id}/`
- `AppState` tracks multiple unlocked vaults in `HashMap<String, Arc<Vault>>`
- All commands require explicit `vault_id` parameter (no ambient authority)

**Rationale:**
- **Security:** Explicit vault_id prevents confused deputy attacks, provides clear audit trail
- **Family workflows:** Multiple family members can have vaults unlocked simultaneously
- **No state confusion:** Commands can't accidentally operate on wrong vault

### Filesystem Structure

```
~/.local/share/spectral/vaults/
├── {vault-id-1}/
│   ├── metadata.json      # Display name, timestamps
│   ├── vault.db           # SQLite database (encrypted profiles)
│   └── .vault_salt        # Argon2 salt (32 bytes)
└── {vault-id-2}/
    ├── metadata.json
    ├── vault.db
    └── .vault_salt
```

**Vault ID Format:** UUID v4 for uniqueness
**Metadata Schema:**
```json
{
  "vault_id": "550e8400-e29b-41d4-a716-446655440000",
  "display_name": "Alice",
  "created_at": "2024-02-09T20:00:00Z",
  "last_accessed": "2024-02-09T20:30:00Z"
}
```

### Error Handling

**CommandError Structure** (per patterns.md):
```rust
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,           // e.g., "VAULT_LOCKED"
    pub message: String,        // User-friendly message
    pub details: Option<serde_json::Value>, // Debugging context
}
```

**Error Code Mapping:**
- `VaultError::Locked` → `"VAULT_LOCKED"`
- `VaultError::InvalidPassword` → `"INVALID_PASSWORD"`
- `VaultError::VaultNotFound` → `"VAULT_NOT_FOUND"`
- `VaultError::KeyDerivation` → `"KEY_DERIVATION_FAILED"`
- `VaultError::Encryption` → `"ENCRYPTION_FAILED"`
- `VaultError::Decryption` → `"DECRYPTION_FAILED"`

**Security:** Never include passwords or sensitive data in error messages/details.

## File Structure

```
src-tauri/src/
├── lib.rs              # Command registration, AppState setup
├── main.rs             # Binary entry (unchanged)
├── state.rs            # AppState definition
├── error.rs            # CommandError type
└── commands/
    ├── mod.rs          # Module re-exports
    └── vault.rs        # Vault command implementations
```

## Component Details

### AppState (state.rs)

```rust
pub struct AppState {
    /// Base directory for all vaults
    vaults_dir: PathBuf,

    /// Currently unlocked vaults: vault_id -> Vault
    /// RwLock allows concurrent reads (status checks)
    unlocked_vaults: RwLock<HashMap<String, Arc<Vault>>>,

    scanner: RwLock<Option<Arc<Scanner>>>,
    config: RwLock<Config>,
}

impl AppState {
    pub fn new() -> Self {
        let dirs = directories::ProjectDirs::from("com", "spectral", "spectral")
            .expect("failed to determine project directories");
        let vaults_dir = dirs.data_dir().join("vaults");

        // Create vaults directory if it doesn't exist
        std::fs::create_dir_all(&vaults_dir).ok();

        Self {
            vaults_dir,
            unlocked_vaults: RwLock::new(HashMap::new()),
            scanner: RwLock::new(None),
            config: RwLock::new(Config::default()),
        }
    }
}
```

### Commands (commands/vault.rs)

#### 1. vault_create

**Signature:**
```rust
#[tauri::command]
async fn vault_create(
    state: State<'_, AppState>,
    vault_id: String,
    display_name: String,
    password: String,
) -> Result<(), CommandError>
```

**Flow:**
1. Validate vault_id doesn't already exist
2. Create directory: `vaults_dir/{vault_id}/`
3. Call `Vault::create(password, vault_db_path)` (creates db + salt)
4. Write `metadata.json` with display_name, timestamps
5. Insert into `unlocked_vaults` HashMap
6. Return success

**Error Cases:**
- Vault already exists → `VAULT_ALREADY_EXISTS`
- Invalid vault_id format → `INVALID_VAULT_ID`
- Filesystem errors → `FILESYSTEM_ERROR`
- Vault creation fails → propagate from VaultError

#### 2. vault_unlock

**Signature:**
```rust
#[tauri::command]
async fn vault_unlock(
    state: State<'_, AppState>,
    vault_id: String,
    password: String,
) -> Result<(), CommandError>
```

**Flow:**
1. Check vault directory exists
2. Call `Vault::unlock(password, vault_db_path)`
3. If successful, insert into `unlocked_vaults`
4. Update `last_accessed` in metadata.json
5. Return success

**Error Cases:**
- Vault doesn't exist → `VAULT_NOT_FOUND`
- Wrong password → `INVALID_PASSWORD`
- Already unlocked → Return success (idempotent)

#### 3. vault_lock

**Signature:**
```rust
#[tauri::command]
async fn vault_lock(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<(), CommandError>
```

**Flow:**
1. Remove vault from `unlocked_vaults` HashMap
2. Vault's `Drop` impl automatically zeroizes key
3. Return success

**Error Cases:**
- Not unlocked → Return success (idempotent)

#### 4. vault_status

**Signature:**
```rust
#[tauri::command]
async fn vault_status(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<VaultStatus, CommandError>
```

**Response:**
```rust
#[derive(Debug, Serialize)]
pub struct VaultStatus {
    pub exists: bool,
    pub unlocked: bool,
    pub display_name: Option<String>,
}
```

**Flow:**
1. Check if vault directory exists
2. Check if vault_id in `unlocked_vaults` HashMap
3. If exists, read `display_name` from metadata.json
4. Return status

#### 5. list_vaults

**Signature:**
```rust
#[tauri::command]
async fn list_vaults(
    state: State<'_, AppState>,
) -> Result<Vec<VaultInfo>, CommandError>
```

**Response:**
```rust
#[derive(Debug, Serialize)]
pub struct VaultInfo {
    pub vault_id: String,
    pub display_name: String,
    pub created_at: String,
    pub last_accessed: String,
    pub unlocked: bool,
}
```

**Flow:**
1. Scan `vaults_dir` for subdirectories
2. For each directory, read `metadata.json`
3. Check if vault_id in `unlocked_vaults`
4. Return Vec<VaultInfo>

## Frontend Integration

### API Wrapper ($lib/api/vault.ts)

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface VaultStatus {
  exists: boolean;
  unlocked: boolean;
  display_name?: string;
}

export interface VaultInfo {
  vault_id: string;
  display_name: string;
  created_at: string;
  last_accessed: string;
  unlocked: boolean;
}

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
```

### Vault Store ($lib/stores/vault.ts)

```typescript
import { writable, derived } from 'svelte/store';
import * as vaultApi from '$lib/api/vault';

interface VaultState {
  currentVaultId: string | null;
  availableVaults: VaultInfo[];
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

    async loadVaults() {
      const vaults = await vaultApi.listVaults();
      update(state => ({ ...state, availableVaults: vaults }));
    },

    setCurrentVault(vaultId: string) {
      update(state => ({ ...state, currentVaultId: vaultId }));
    },

    async unlock(vaultId: string, password: string) {
      await vaultApi.unlockVault(vaultId, password);
      update(state => ({
        ...state,
        currentVaultId: vaultId,
        unlockedVaultIds: new Set([...state.unlockedVaultIds, vaultId]),
      }));
    },

    async lock(vaultId: string) {
      await vaultApi.lockVault(vaultId);
      update(state => {
        const newUnlocked = new Set(state.unlockedVaultIds);
        newUnlocked.delete(vaultId);
        return {
          ...state,
          unlockedVaultIds: newUnlocked,
          currentVaultId: state.currentVaultId === vaultId ? null : state.currentVaultId,
        };
      });
    },
  };
}

export const vaultStore = createVaultStore();
export const currentVaultId = derived(vaultStore, $v => $v.currentVaultId);
export const isCurrentVaultUnlocked = derived(
  vaultStore,
  $v => $v.currentVaultId !== null && $v.unlockedVaultIds.has($v.currentVaultId)
);
```

## Testing Strategy

### Unit Tests

**error.rs:**
- Test VaultError → CommandError conversion
- Verify error code mapping
- Ensure sensitive data not leaked

**state.rs:**
- Test AppState initialization
- Test vaults_dir path construction
- Test concurrent HashMap access

**commands/vault.rs:**
- Test vault_create creates correct directory structure
- Test vault_unlock with correct/wrong password
- Test vault_lock removes from HashMap
- Test vault_status for all states (exists+locked, exists+unlocked, not exists)
- Test list_vaults scanning multiple vaults

### Integration Tests

**vault_commands.rs:**
```rust
#[tokio::test]
async fn test_full_vault_lifecycle() {
    // 1. Create vault
    // 2. Verify exists and unlocked
    // 3. Lock vault
    // 4. Verify exists and locked
    // 5. Unlock with correct password
    // 6. Verify wrong password fails
    // 7. Test list_vaults sees it
}

#[tokio::test]
async fn test_multiple_vaults() {
    // 1. Create vault1 and vault2
    // 2. Unlock both
    // 3. Verify both in list as unlocked
    // 4. Lock vault1
    // 5. Verify vault2 still unlocked
}
```

## Acceptance Criteria

- ✅ Frontend can create new vault
- ✅ Frontend can unlock vault with correct password
- ✅ Frontend can lock vault
- ✅ Frontend can query vault status (exists, unlocked, display_name)
- ✅ Frontend can list all available vaults
- ✅ Errors properly serialized with codes and messages
- ✅ State persists across command calls
- ✅ Multiple vaults supported simultaneously
- ✅ All commands pass `cargo test -p spectral-app`
- ✅ All commands pass `cargo clippy -- -D warnings`

## Security Considerations

1. **Password handling:** Passwords never logged, never in error messages
2. **Explicit vault_id:** No ambient authority, clear audit trail
3. **Key zeroization:** Vault Drop impl zeroizes keys automatically
4. **Metadata separation:** Display names in plaintext, PII stays encrypted
5. **Directory permissions:** System-default (user-only on Unix)

## Implementation Order

1. **error.rs** - CommandError type and conversions
2. **state.rs** - AppState with multi-vault support
3. **commands/vault.rs** - All 5 command implementations
4. **lib.rs** - Wire up commands and state
5. **Cargo.toml** - Add dependencies (spectral-vault, spectral-db, directories)
6. **Tests** - Unit tests for each module
7. **Integration tests** - Full workflow tests

## Follow-up Tasks

After Task 1.4 completion:
- **Task 1.5:** Unlock Screen UI - Build SvelteKit components using vault API
- **Task 1.6:** Database Integration - Extend commands for profile operations
- **Task 1.12:** Onboarding UI - First-run vault creation flow
