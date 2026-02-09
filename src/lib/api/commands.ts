/**
 * Tauri Command Wrappers
 *
 * Type-safe wrappers for Tauri commands defined in src-tauri/src/lib.rs
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * Check if the Tauri backend is ready
 */
export async function healthCheck(): Promise<string> {
	return invoke<string>('health_check');
}

/**
 * Get application version from Tauri
 */
export async function getVersion(): Promise<string> {
	return invoke<string>('get_version');
}
