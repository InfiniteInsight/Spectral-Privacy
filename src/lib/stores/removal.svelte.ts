/**
 * Removal Store - Reactive state management for removal operations
 *
 * Manages removal attempt state including:
 * - Removal attempts list
 * - Real-time event updates from Tauri
 * - Queue filtering (submitted, captcha, failed, in-progress)
 * - Loading and error states
 *
 * Uses Svelte 5 runes for reactive state management.
 *
 * @module $lib/stores/removal
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { RemovalAttempt } from '$lib/api/removal';
import { removalAPI } from '$lib/api/removal';

// Event payload types
interface RemovalStartedEvent {
	attempt_id: string;
	broker_id: string;
}

interface RemovalSuccessEvent {
	attempt_id: string;
	outcome: string;
}

interface RemovalCaptchaEvent {
	attempt_id: string;
	outcome: string;
}

interface RemovalFailedEvent {
	attempt_id: string;
	error: string;
}

interface RemovalRetryEvent {
	attempt_id: string;
	attempt_num: number;
}

/**
 * Removal state interface
 */
interface RemovalState {
	removalAttempts: RemovalAttempt[];
	scanJobId: string | null;
	loading: boolean;
	error: string | null;
}

/**
 * Extract CAPTCHA URL from debug-formatted outcome string
 * Parses strings like: 'RequiresCaptcha { captcha_url: "https://example.com" }'
 *
 * @param outcome - The debug-formatted outcome string from backend
 * @returns The extracted CAPTCHA URL or null if not found
 */
function extractCaptchaUrl(outcome: string): string | null {
	const match = /captcha_url:\s*"([^"]+)"/.exec(outcome);
	return match ? match[1] : null;
}

/**
 * Create a reactive removal store using Svelte 5 runes
 *
 * @returns Removal store with getters and actions
 */
function createRemovalStore() {
	// State using Svelte 5 runes
	let state = $state<RemovalState>({
		removalAttempts: [],
		scanJobId: null,
		loading: false,
		error: null
	});

	// Event unlisteners
	let unlisteners: UnlistenFn[] = [];

	// Derived queues using $derived
	const submitted = $derived(
		state.removalAttempts.filter((r) => r.status === 'Submitted' || r.status === 'Completed')
	);

	const captchaQueue = $derived(
		state.removalAttempts.filter(
			(r) => r.status === 'Pending' && r.error_message?.startsWith('CAPTCHA_REQUIRED')
		)
	);

	const failedQueue = $derived(state.removalAttempts.filter((r) => r.status === 'Failed'));

	const inProgress = $derived(state.removalAttempts.filter((r) => r.status === 'Processing'));

	return {
		// Getters for reactive access
		get removalAttempts() {
			return state.removalAttempts;
		},
		get scanJobId() {
			return state.scanJobId;
		},
		get loading() {
			return state.loading;
		},
		get error() {
			return state.error;
		},

		// Derived queue getters
		get submitted() {
			return submitted;
		},
		get captchaQueue() {
			return captchaQueue;
		},
		get failedQueue() {
			return failedQueue;
		},
		get inProgress() {
			return inProgress;
		},
		get allComplete() {
			// True when all terminal states reached: nothing in progress,
			// nothing in CAPTCHA queue, and nothing pending without CAPTCHA
			const pendingWithoutCaptcha = state.removalAttempts.filter(
				(r) => r.status === 'Pending' && !r.error_message?.startsWith('CAPTCHA_REQUIRED')
			);
			return (
				state.removalAttempts.length > 0 &&
				this.inProgress.length === 0 &&
				this.captchaQueue.length === 0 &&
				pendingWithoutCaptcha.length === 0
			);
		},

		/**
		 * Load removal attempts for a scan job
		 *
		 * @param vaultId - The vault ID
		 * @param scanJobId - The scan job ID
		 */
		async loadRemovalAttempts(vaultId: string, scanJobId: string): Promise<void> {
			// Reset clears stale data from any previous job loaded in this session
			state.removalAttempts = [];
			state.scanJobId = null;
			state.loading = true;
			state.error = null;

			try {
				const attempts = await removalAPI.getByScanJob(vaultId, scanJobId);
				state.removalAttempts = attempts;
				state.scanJobId = scanJobId;
			} catch (err) {
				state.error = err instanceof Error ? err.message : String(err);
				console.error('Failed to load removal attempts:', err);
			} finally {
				state.loading = false;
			}
		},

		/**
		 * Retry a failed removal attempt
		 *
		 * @param vaultId - The vault ID
		 * @param attemptId - The removal attempt ID
		 */
		async retryRemoval(vaultId: string, attemptId: string): Promise<void> {
			try {
				await removalAPI.retry(vaultId, attemptId);
				// Event listener will update state
			} catch (err) {
				console.error('Failed to retry removal:', err);
				throw err;
			}
		},

		/**
		 * Update a removal attempt with partial data
		 *
		 * @param attemptId - The removal attempt ID
		 * @param updates - Partial removal attempt data to merge
		 */
		updateAttempt(attemptId: string, updates: Partial<RemovalAttempt>): void {
			const index = state.removalAttempts.findIndex((a) => a.id === attemptId);
			if (index !== -1) {
				state.removalAttempts[index] = {
					...state.removalAttempts[index],
					...updates
				};
			}
		},

		/**
		 * Set up Tauri event listeners for real-time removal updates
		 * Listens for: removal:started, removal:success, removal:captcha, removal:failed, removal:retry
		 */
		async setupEventListeners(): Promise<void> {
			// Clean up any existing listeners
			this.cleanupEventListeners();

			// removal:started
			const unlistenStarted = await listen<RemovalStartedEvent>('removal:started', (event) => {
				this.updateAttempt(event.payload.attempt_id, {
					status: 'Processing'
				});
			});
			unlisteners.push(unlistenStarted);

			// removal:success
			const unlistenSuccess = await listen<RemovalSuccessEvent>('removal:success', (event) => {
				this.updateAttempt(event.payload.attempt_id, {
					status: 'Submitted',
					submitted_at: new Date().toISOString()
				});
			});
			unlisteners.push(unlistenSuccess);

			// removal:captcha
			const unlistenCaptcha = await listen<RemovalCaptchaEvent>('removal:captcha', (event) => {
				const captchaUrl = extractCaptchaUrl(event.payload.outcome);
				this.updateAttempt(event.payload.attempt_id, {
					status: 'Pending',
					error_message: captchaUrl ? `CAPTCHA_REQUIRED:${captchaUrl}` : 'CAPTCHA_REQUIRED'
				});
			});
			unlisteners.push(unlistenCaptcha);

			// removal:failed
			const unlistenFailed = await listen<RemovalFailedEvent>('removal:failed', (event) => {
				this.updateAttempt(event.payload.attempt_id, {
					status: 'Failed',
					error_message: event.payload.error
				});
			});
			unlisteners.push(unlistenFailed);

			// removal:retry
			const unlistenRetry = await listen<RemovalRetryEvent>('removal:retry', (event) => {
				this.updateAttempt(event.payload.attempt_id, {
					status: 'Processing',
					error_message: null
				});
			});
			unlisteners.push(unlistenRetry);
		},

		/**
		 * Reset store state — call at start of each new job load
		 * to prevent stale data from previous jobs accumulating
		 */
		reset(): void {
			this.cleanupEventListeners();
			state.removalAttempts = [];
			state.scanJobId = null;
			state.loading = false;
			state.error = null;
		},

		/**
		 * Clean up all event listeners
		 * Should be called when the store is no longer needed
		 */
		cleanupEventListeners(): void {
			unlisteners.forEach((unlisten) => unlisten());
			unlisteners = [];
		}
	};
}

/**
 * Global removal store instance
 */
export const removalStore = createRemovalStore();
