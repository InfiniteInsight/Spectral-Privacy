/**
 * Application State Store
 *
 * Core application state using Svelte 5 runes.
 */

import { writable } from 'svelte/store';

export type AppState = 'loading' | 'locked' | 'unlocked' | 'error';

/**
 * Current application state
 */
export const appState = writable<AppState>('loading');

/**
 * Whether the vault is currently unlocked
 */
export const vaultUnlocked = writable<boolean>(false);

/**
 * Current error message, if any
 */
export const errorMessage = writable<string | null>(null);
