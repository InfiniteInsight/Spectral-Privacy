/**
 * Vault Store - Reactive state management for vault operations
 *
 * Manages vault state including:
 * - Current vault selection
 * - Available vaults list
 * - Unlocked vault tracking
 * - Loading and error states
 *
 * Uses Svelte 5 runes for reactive state management.
 *
 * @module $lib/stores/vault
 */

import { listVaults, unlockVault, lockVault, createVault } from '$lib/api/vault';
import type { VaultInfo } from '$lib/api/vault';
import { profileStore } from '$lib/stores/profile.svelte';
import { scanStore } from '$lib/stores/scan.svelte';
import { removalStore } from '$lib/stores/removal.svelte';

/**
 * Vault state interface
 */
interface VaultState {
	currentVaultId: string | null;
	availableVaults: VaultInfo[];
	unlockedVaultIds: Set<string>;
	loading: boolean;
	error: string | null;
}

/**
 * Create a reactive vault store using Svelte 5 runes
 *
 * @returns Vault store with getters and actions
 */
function createVaultStore() {
	// State using Svelte 5 runes
	let state = $state<VaultState>({
		currentVaultId: null,
		availableVaults: [],
		unlockedVaultIds: new Set<string>(),
		loading: false,
		error: null
	});

	return {
		// Getters for reactive access
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

		// Derived getter - computed value
		get isCurrentVaultUnlocked(): boolean {
			return state.currentVaultId !== null && state.unlockedVaultIds.has(state.currentVaultId);
		},

		/**
		 * Load all available vaults from the backend
		 * Auto-selects first vault if none is currently selected
		 */
		async loadVaults() {
			state.loading = true;
			state.error = null;
			try {
				state.availableVaults = await listVaults();

				// Auto-select first vault if none selected
				if (state.currentVaultId === null && state.availableVaults.length > 0) {
					state.currentVaultId = state.availableVaults[0].vault_id;
				}
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to load vaults';
			} finally {
				state.loading = false;
			}
		},

		/**
		 * Create a new vault
		 *
		 * @param vaultId - Unique vault identifier
		 * @param displayName - Human-readable vault name
		 * @param password - Master password
		 * @throws {Error} If creation fails
		 */
		async createVault(vaultId: string, displayName: string, password: string) {
			state.loading = true;
			state.error = null;
			try {
				await createVault(vaultId, displayName, password);
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to create vault';
				throw err;
			} finally {
				state.loading = false;
			}
		},

		/**
		 * Unlock a vault with the provided password
		 *
		 * @param vaultId - Vault identifier
		 * @param password - Master password
		 * @throws {Error} If unlock fails (re-thrown for component handling)
		 */
		async unlock(vaultId: string, password: string) {
			state.loading = true;
			state.error = null;
			try {
				await unlockVault(vaultId, password);
				state.unlockedVaultIds = new Set([...state.unlockedVaultIds, vaultId]);
				state.currentVaultId = vaultId;
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to unlock vault';
				throw err; // Re-throw for component error handling
			} finally {
				state.loading = false;
			}
		},

		/**
		 * Lock a currently unlocked vault
		 *
		 * @param vaultId - Vault identifier
		 */
		async lock(vaultId: string) {
			state.loading = true;
			state.error = null;
			try {
				await lockVault(vaultId);
				const newUnlocked = new Set(state.unlockedVaultIds);
				newUnlocked.delete(vaultId);
				state.unlockedVaultIds = newUnlocked;
				if (state.currentVaultId === vaultId) {
					state.currentVaultId = null;
				}
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to lock vault';
			} finally {
				state.loading = false;
			}
		},

		/**
		 * Set the current active vault
		 *
		 * @param vaultId - Vault identifier or null to clear
		 */
		setCurrentVault(vaultId: string | null): void {
			if (vaultId !== state.currentVaultId) {
				profileStore.reset();
				scanStore.reset();
				removalStore.reset();
			}
			state.currentVaultId = vaultId;
			state.error = null;
		},

		/**
		 * Clear any error state
		 */
		clearError() {
			state.error = null;
		}
	};
}

/**
 * Global vault store instance
 */
export const vaultStore = createVaultStore();
