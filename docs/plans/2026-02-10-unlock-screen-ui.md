# Unlock Screen UI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the SvelteKit frontend unlock screen with vault selection and password entry.

**Architecture:** Create TypeScript API wrappers for Tauri vault commands, Svelte 5 stores with runes for state management, and unlock screen UI components with vault selection dropdown and password input. Support multiple vaults, error handling, and loading states.

**Tech Stack:** SvelteKit, Svelte 5 (runes), TypeScript, Tauri API (@tauri-apps/api/core), Tailwind CSS

---

## Task 1: Vault API Wrapper

**Files:**
- Create: `src/lib/api/vault.ts`

**Step 1: Create vault API module with TypeScript interfaces**

```typescript
//! Vault API - Tauri command wrappers for vault operations

import { invoke } from '@tauri-apps/api/core';

/**
 * Vault status response from backend
 */
export interface VaultStatus {
	exists: boolean;
	unlocked: boolean;
	display_name?: string;
}

/**
 * Vault information for listing
 */
export interface VaultInfo {
	vault_id: string;
	display_name: string;
	created_at: string;
	last_accessed: string;
	unlocked: boolean;
}

/**
 * Error response from Tauri commands
 */
export interface CommandError {
	code: string;
	message: string;
	details?: unknown;
}

/**
 * Create a new vault
 */
export async function createVault(
	vaultId: string,
	displayName: string,
	password: string
): Promise<void> {
	return invoke('vault_create', { vaultId, displayName, password });
}

/**
 * Unlock an existing vault
 */
export async function unlockVault(vaultId: string, password: string): Promise<void> {
	return invoke('vault_unlock', { vaultId, password });
}

/**
 * Lock a vault
 */
export async function lockVault(vaultId: string): Promise<void> {
	return invoke('vault_lock', { vaultId });
}

/**
 * Get vault status
 */
export async function getVaultStatus(vaultId: string): Promise<VaultStatus> {
	return invoke('vault_status', { vaultId });
}

/**
 * List all available vaults
 */
export async function listVaults(): Promise<VaultInfo[]> {
	return invoke('list_vaults');
}
```

**Step 2: Build check**

Run: `npm run check`

Expected: No TypeScript errors

**Step 3: Commit**

```bash
git add src/lib/api/vault.ts
git commit -m "feat(ui): add vault API wrapper with Tauri commands

- TypeScript interfaces for VaultStatus and VaultInfo
- Wrapper functions for all vault commands
- JSDoc documentation for all exports"
```

---

## Task 2: Vault Store with Svelte 5 Runes

**Files:**
- Create: `src/lib/stores/vault.svelte.ts`

**Step 1: Create vault store with Svelte 5 runes**

```typescript
//! Vault Store - Reactive state management for vaults using Svelte 5 runes

import * as vaultApi from '$lib/api/vault';
import type { VaultInfo } from '$lib/api/vault';

interface VaultState {
	currentVaultId: string | null;
	availableVaults: VaultInfo[];
	unlockedVaultIds: Set<string>;
	loading: boolean;
	error: string | null;
}

/**
 * Create vault store with Svelte 5 runes
 */
function createVaultStore() {
	let state = $state<VaultState>({
		currentVaultId: null,
		availableVaults: [],
		unlockedVaultIds: new Set(),
		loading: false,
		error: null
	});

	return {
		// Reactive state getters
		get currentVaultId() {
			return state.currentVaultId;
		},
		get availableVaults() {
			return state.availableVaults;
		},
		get unlockedVaultIds() {
			return state.unlockedVaultIds;
		},
		get loading() {
			return state.loading;
		},
		get error() {
			return state.error;
		},

		// Derived state
		get isCurrentVaultUnlocked(): boolean {
			return (
				state.currentVaultId !== null &&
				state.unlockedVaultIds.has(state.currentVaultId)
			);
		},

		// Actions
		async loadVaults() {
			state.loading = true;
			state.error = null;
			try {
				const vaults = await vaultApi.listVaults();
				state.availableVaults = vaults;

				// Update unlocked vault IDs
				const unlocked = new Set<string>();
				for (const vault of vaults) {
					if (vault.unlocked) {
						unlocked.add(vault.vault_id);
					}
				}
				state.unlockedVaultIds = unlocked;
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to load vaults';
			} finally {
				state.loading = false;
			}
		},

		setCurrentVault(vaultId: string) {
			state.currentVaultId = vaultId;
			state.error = null;
		},

		async unlock(vaultId: string, password: string) {
			state.loading = true;
			state.error = null;
			try {
				await vaultApi.unlockVault(vaultId, password);
				state.currentVaultId = vaultId;
				state.unlockedVaultIds = new Set([...state.unlockedVaultIds, vaultId]);

				// Reload vaults to get updated status
				await this.loadVaults();
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to unlock vault';
				throw err;
			} finally {
				state.loading = false;
			}
		},

		async lock(vaultId: string) {
			state.loading = true;
			state.error = null;
			try {
				await vaultApi.lockVault(vaultId);
				const newUnlocked = new Set(state.unlockedVaultIds);
				newUnlocked.delete(vaultId);
				state.unlockedVaultIds = newUnlocked;

				if (state.currentVaultId === vaultId) {
					state.currentVaultId = null;
				}
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to lock vault';
				throw err;
			} finally {
				state.loading = false;
			}
		},

		clearError() {
			state.error = null;
		}
	};
}

export const vaultStore = createVaultStore();
```

**Step 2: Build check**

Run: `npm run check`

Expected: No TypeScript errors

**Step 3: Commit**

```bash
git add src/lib/stores/vault.svelte.ts
git commit -m "feat(ui): add vault store with Svelte 5 runes

- Reactive state management using $state and $derived
- Load vaults, unlock, lock operations
- Error handling and loading states
- Tracks current vault and unlocked vaults"
```

---

## Task 3: Unlock Screen Component

**Files:**
- Create: `src/lib/components/UnlockScreen.svelte`

**Step 1: Create unlock screen component**

```svelte
<script lang="ts">
	import { vaultStore } from '$lib/stores/vault.svelte';
	import { onMount } from 'svelte';

	let password = $state('');
	let selectedVaultId = $state<string | null>(null);
	let showPassword = $state(false);

	// Load vaults on mount
	onMount(async () => {
		await vaultStore.loadVaults();

		// Auto-select first vault if available
		if (vaultStore.availableVaults.length > 0) {
			selectedVaultId = vaultStore.availableVaults[0].vault_id;
		}
	});

	// Get selected vault info
	const selectedVault = $derived(
		vaultStore.availableVaults.find((v) => v.vault_id === selectedVaultId)
	);

	async function handleUnlock() {
		if (!selectedVaultId || !password) return;

		try {
			await vaultStore.unlock(selectedVaultId, password);
			// Clear password after successful unlock
			password = '';
		} catch (err) {
			// Error is stored in vaultStore.error
			console.error('Unlock failed:', err);
		}
	}

	function handleKeyPress(event: KeyboardEvent) {
		if (event.key === 'Enter' && selectedVaultId && password) {
			handleUnlock();
		}
	}
</script>

<div class="min-h-screen flex items-center justify-center bg-gradient-to-br from-primary-50 to-primary-100 p-4">
	<div class="w-full max-w-md">
		<!-- Header -->
		<div class="text-center mb-8">
			<h1 class="text-4xl font-bold text-primary-900 mb-2">Spectral</h1>
			<p class="text-primary-700">Unlock your vault to continue</p>
		</div>

		<!-- Unlock Card -->
		<div class="bg-white rounded-lg shadow-xl p-8 space-y-6">
			<!-- Loading State -->
			{#if vaultStore.loading && vaultStore.availableVaults.length === 0}
				<div class="text-center py-8">
					<div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
					<p class="mt-4 text-gray-600">Loading vaults...</p>
				</div>
			{:else if vaultStore.availableVaults.length === 0}
				<!-- No Vaults -->
				<div class="text-center py-8">
					<p class="text-gray-600 mb-4">No vaults found</p>
					<p class="text-sm text-gray-500">Create a new vault to get started</p>
				</div>
			{:else}
				<!-- Vault Selection -->
				<div>
					<label for="vault-select" class="block text-sm font-medium text-gray-700 mb-2">
						Select Vault
					</label>
					<select
						id="vault-select"
						bind:value={selectedVaultId}
						class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
					>
						{#each vaultStore.availableVaults as vault}
							<option value={vault.vault_id}>
								{vault.display_name}
								{vault.unlocked ? '(Unlocked)' : ''}
							</option>
						{/each}
					</select>
				</div>

				<!-- Password Input -->
				<div>
					<label for="password" class="block text-sm font-medium text-gray-700 mb-2">
						Master Password
					</label>
					<div class="relative">
						<input
							id="password"
							type={showPassword ? 'text' : 'password'}
							bind:value={password}
							onkeypress={handleKeyPress}
							placeholder="Enter your master password"
							class="w-full px-4 py-2 pr-12 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
							disabled={vaultStore.loading}
						/>
						<button
							type="button"
							onclick={() => (showPassword = !showPassword)}
							class="absolute right-2 top-1/2 -translate-y-1/2 p-2 text-gray-500 hover:text-gray-700"
							aria-label={showPassword ? 'Hide password' : 'Show password'}
						>
							{#if showPassword}
								<!-- Eye Slash Icon -->
								<svg
									xmlns="http://www.w3.org/2000/svg"
									class="h-5 w-5"
									fill="none"
									viewBox="0 0 24 24"
									stroke="currentColor"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"
									/>
								</svg>
							{:else}
								<!-- Eye Icon -->
								<svg
									xmlns="http://www.w3.org/2000/svg"
									class="h-5 w-5"
									fill="none"
									viewBox="0 0 24 24"
									stroke="currentColor"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
									/>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
									/>
								</svg>
							{/if}
						</button>
					</div>
				</div>

				<!-- Error Message -->
				{#if vaultStore.error}
					<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
						<p class="text-sm text-red-800">{vaultStore.error}</p>
					</div>
				{/if}

				<!-- Unlock Button -->
				<button
					onclick={handleUnlock}
					disabled={!selectedVaultId || !password || vaultStore.loading}
					class="w-full px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
				>
					{#if vaultStore.loading}
						<span class="flex items-center justify-center">
							<span class="inline-block animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-2"></span>
							Unlocking...
						</span>
					{:else}
						Unlock Vault
					{/if}
				</button>

				<!-- Vault Info -->
				{#if selectedVault}
					<div class="pt-4 border-t border-gray-200">
						<p class="text-xs text-gray-500 text-center">
							Last accessed: {new Date(selectedVault.last_accessed).toLocaleString()}
						</p>
					</div>
				{/if}
			{/if}
		</div>
	</div>
</div>
```

**Step 2: Build check**

Run: `npm run check`

Expected: No TypeScript errors

**Step 3: Commit**

```bash
git add src/lib/components/UnlockScreen.svelte
git commit -m "feat(ui): add unlock screen component

- Vault selection dropdown
- Password input with show/hide toggle
- Loading states and error handling
- Enter key support for unlock
- Last accessed timestamp display
- Responsive design with Tailwind CSS"
```

---

## Task 4: Integrate Unlock Screen into App

**Files:**
- Modify: `src/routes/+page.svelte`

**Step 1: Replace placeholder with unlock screen**

```svelte
<script lang="ts">
	import UnlockScreen from '$lib/components/UnlockScreen.svelte';
	import { vaultStore } from '$lib/stores/vault.svelte';
</script>

{#if vaultStore.isCurrentVaultUnlocked}
	<!-- TODO: Show dashboard when vault is unlocked -->
	<div class="min-h-screen flex items-center justify-center bg-gradient-to-br from-primary-50 to-primary-100">
		<div class="text-center p-8">
			<h1 class="text-4xl font-bold text-primary-900 mb-4">Vault Unlocked</h1>
			<p class="text-lg text-primary-700 mb-8">
				Welcome to Spectral
			</p>
			<button
				onclick={() => vaultStore.lock(vaultStore.currentVaultId!)}
				class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700"
			>
				Lock Vault
			</button>
		</div>
	</div>
{:else}
	<UnlockScreen />
{/if}
```

**Step 2: Test in dev mode**

Run: `npm run tauri dev`

Expected: App launches and shows unlock screen

**Step 3: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat(ui): integrate unlock screen into main route

- Show unlock screen when vault is locked
- Show placeholder dashboard when vault is unlocked
- Add lock button for testing"
```

---

## Task 5: Manual Testing

**Files:**
- None (manual testing)

**Step 1: Test unlock flow**

1. Start app: `npm run tauri dev`
2. Verify unlock screen appears
3. Test vault selection dropdown
4. Enter password and unlock
5. Verify "Vault Unlocked" message appears
6. Click "Lock Vault" button
7. Verify unlock screen reappears

**Step 2: Test error handling**

1. Enter wrong password
2. Verify error message appears
3. Enter correct password
4. Verify error clears and unlock succeeds

**Step 3: Test password visibility toggle**

1. Enter password
2. Click eye icon
3. Verify password becomes visible
4. Click again to hide

**Step 4: Test keyboard interaction**

1. Enter password
2. Press Enter
3. Verify unlock is triggered

**Expected Results:**
- All UI interactions work smoothly
- Loading states appear during async operations
- Errors display user-friendly messages
- Unlock/lock cycle works correctly

---

## Acceptance Criteria

- ✅ Vault API wrapper created with all 5 commands
- ✅ Vault store uses Svelte 5 runes ($state, $derived)
- ✅ Unlock screen displays vault selection dropdown
- ✅ Password input with show/hide toggle works
- ✅ Loading states displayed during operations
- ✅ Error messages displayed on failure
- ✅ Enter key triggers unlock
- ✅ Last accessed timestamp shown
- ✅ Unlock → Dashboard → Lock cycle works
- ✅ Responsive design with Tailwind CSS
- ✅ No TypeScript errors
- ✅ App builds and runs successfully

---

## Next Steps

After Task 1.5 completion:
- **Task 1.6:** Database Integration - Profile CRUD operations
- **Task 1.12:** Onboarding UI - First-run vault creation flow
- **Task 2.x:** Dashboard implementation with data broker scanning
