/**
 * Vault API Wrappers
 *
 * Type-safe wrappers for vault-related Tauri commands.
 * Provides functionality for vault creation, unlocking, locking, and status management.
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * Check if we're running in Tauri environment
 * Uses multiple detection methods for reliability
 */
function isTauriAvailable(): boolean {
	if (typeof window === 'undefined') return false;

	// Check multiple ways Tauri might be available
	return (
		'__TAURI__' in window ||
		'__TAURI_INTERNALS__' in window ||
		!!(window as any).__TAURI__ ||
		!!(window as any).__TAURI_INTERNALS__
	);
}

/**
 * Wrapper around invoke that checks Tauri availability
 */
async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	try {
		// Just try to invoke - let Tauri handle its own errors
		return await invoke<T>(command, args);
	} catch (error) {
		// Add context to Tauri errors
		console.error('Vault command failed:', {
			command,
			args,
			error,
			isTauriDetected: isTauriAvailable()
		});
		throw error;
	}
}

/**
 * Vault status information
 */
export interface VaultStatus {
	exists: boolean;
	unlocked: boolean;
	display_name?: string;
}

/**
 * Vault metadata and information
 */
export interface VaultInfo {
	vault_id: string;
	display_name: string;
	created_at: string;
	last_accessed: string;
	unlocked: boolean;
}

/**
 * Error response from vault commands
 */
export interface CommandError {
	code: string;
	message: string;
	details?: unknown;
}

/**
 * Create a new vault with the specified ID and password
 *
 * @param vaultId - Unique identifier for the vault
 * @param displayName - Display name for the vault
 * @param password - Master password for the vault
 * @throws {CommandError} If vault creation fails
 */
export async function createVault(
	vaultId: string,
	displayName: string,
	password: string
): Promise<void> {
	return safeInvoke<void>('vault_create', { vaultId, displayName, password });
}

/**
 * Unlock an existing vault with the provided password
 *
 * @param vaultId - Vault identifier
 * @param password - Master password for the vault
 * @throws {CommandError} If unlock fails (wrong password, vault not found, etc.)
 */
export async function unlockVault(vaultId: string, password: string): Promise<void> {
	return safeInvoke<void>('vault_unlock', { vaultId, password });
}

/**
 * Lock a currently unlocked vault
 *
 * @param vaultId - Vault identifier
 * @throws {CommandError} If lock fails
 */
export async function lockVault(vaultId: string): Promise<void> {
	return safeInvoke<void>('vault_lock', { vaultId });
}

/**
 * Get the current status of a vault
 *
 * @param vaultId - Vault identifier
 * @returns {VaultStatus} Current vault status
 * @throws {CommandError} If status check fails
 */
export async function getVaultStatus(vaultId: string): Promise<VaultStatus> {
	return safeInvoke<VaultStatus>('vault_status', { vaultId });
}

/**
 * List all available vaults
 *
 * @returns {VaultInfo[]} Array of vault information
 * @throws {CommandError} If listing fails
 */
export async function listVaults(): Promise<VaultInfo[]> {
	return safeInvoke<VaultInfo[]>('list_vaults');
}

/**
 * Rename an existing vault
 *
 * @param vaultId - Vault identifier
 * @param newName - New display name for the vault
 * @throws {CommandError} If renaming fails
 */
export async function renameVault(vaultId: string, newName: string): Promise<void> {
	return safeInvoke<void>('rename_vault', { vaultId, newName });
}

/**
 * Change the master password of an existing vault
 *
 * @param vaultId - Vault identifier
 * @param oldPassword - Current master password
 * @param newPassword - New master password to set
 * @throws {CommandError} If password change fails
 */
export async function changeVaultPassword(
	vaultId: string,
	oldPassword: string,
	newPassword: string
): Promise<void> {
	return safeInvoke<void>('change_vault_password', { vaultId, oldPassword, newPassword });
}

/**
 * Permanently delete a vault and all its data
 *
 * @param vaultId - Vault identifier
 * @param password - Master password required to confirm deletion
 * @throws {CommandError} If deletion fails
 */
export async function deleteVault(vaultId: string, password: string): Promise<void> {
	return safeInvoke<void>('delete_vault', { vaultId, password });
}
