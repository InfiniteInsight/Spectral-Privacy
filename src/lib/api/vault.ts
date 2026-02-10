/**
 * Vault API Wrappers
 *
 * Type-safe wrappers for vault-related Tauri commands.
 * Provides functionality for vault creation, unlocking, locking, and status management.
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * Check if we're running in Tauri environment
 */
function isTauriAvailable(): boolean {
	return typeof window !== 'undefined' && '__TAURI__' in window;
}

/**
 * Wrapper around invoke that checks Tauri availability
 */
async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	if (!isTauriAvailable()) {
		throw new Error(
			'Tauri is not available. This application must be run in the Tauri desktop app, not in a regular browser.'
		);
	}
	return invoke<T>(command, args);
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
