/**
 * Scan Store - Reactive state management for scan operations
 *
 * Manages scan workflow state including:
 * - Current scan job ID and status
 * - Findings list with verification
 * - Polling for status updates
 * - Loading and error states
 *
 * Uses Svelte 5 runes for reactive state management.
 *
 * @module $lib/stores/scan
 */

import { scanAPI, type ScanJobStatus, type Finding } from '$lib/api/scan';

/**
 * Finding filter type alias
 */
type FindingFilter = 'PendingVerification' | 'Confirmed' | 'Rejected';

/**
 * Scan state interface
 */
interface ScanState {
	currentScanId: string | null;
	scanStatus: ScanJobStatus | null;
	findings: Finding[];
	loading: boolean;
	error: string | null;
	pollingInterval: number | null;
	isPolling: boolean;
}

/**
 * Create a reactive scan store using Svelte 5 runes
 *
 * @returns Scan store with getters and actions
 */
function createScanStore() {
	// State using Svelte 5 runes
	let state = $state<ScanState>({
		currentScanId: null,
		scanStatus: null,
		findings: [],
		loading: false,
		error: null,
		pollingInterval: null,
		isPolling: false
	});

	return {
		// Getters for reactive access
		get currentScanId() {
			return state.currentScanId;
		},
		get scanStatus() {
			return state.scanStatus;
		},
		get findings() {
			return state.findings;
		},
		get loading() {
			return state.loading;
		},
		get error() {
			return state.error;
		},

		/**
		 * Start a new scan job
		 *
		 * @param profileId - The profile ID to scan
		 * @returns The scan job ID or null on error
		 */
		async startScan(profileId: string): Promise<string | null> {
			this.stopPolling();
			state.findings = [];
			state.scanStatus = null;
			state.loading = true;
			state.error = null;

			try {
				const scanJobStatus = await scanAPI.start(profileId);
				state.currentScanId = scanJobStatus.id;
				state.scanStatus = scanJobStatus;
				return scanJobStatus.id;
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to start scan';
				console.error('Start scan error:', err);
				return null;
			} finally {
				state.loading = false;
			}
		},

		/**
		 * Fetch the current status of a scan job
		 *
		 * @param scanJobId - The scan job ID to check
		 */
		async fetchStatus(scanJobId: string): Promise<void> {
			state.error = null;

			try {
				const status = await scanAPI.getStatus(scanJobId);
				state.scanStatus = status;
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to fetch scan status';
				state.scanStatus = null;
				console.error('Fetch status error:', err);
			}
		},

		/**
		 * Start polling for scan status updates
		 * Automatically stops on terminal status (Completed, Failed, Cancelled)
		 *
		 * @param scanJobId - The scan job ID to poll
		 * @param intervalMs - Polling interval in milliseconds (default: 2000)
		 */
		startPolling(scanJobId: string, intervalMs: number = 2000): void {
			// Clear any existing interval
			this.stopPolling();

			// Immediate fetch
			this.fetchStatus(scanJobId);

			// Start polling
			state.pollingInterval = window.setInterval(async () => {
				if (state.isPolling) return;
				state.isPolling = true;
				try {
					await this.fetchStatus(scanJobId);

					// Auto-stop on terminal status
					if (
						state.scanStatus?.status === 'Completed' ||
						state.scanStatus?.status === 'Failed' ||
						state.scanStatus?.status === 'Cancelled'
					) {
						this.stopPolling();
					}
				} finally {
					state.isPolling = false;
				}
			}, intervalMs);
		},

		/**
		 * Stop polling for status updates
		 */
		stopPolling(): void {
			if (state.pollingInterval !== null) {
				clearInterval(state.pollingInterval);
				state.pollingInterval = null;
			}
		},

		/**
		 * Load findings for a scan job
		 *
		 * @param scanJobId - The scan job ID
		 * @param filter - Optional filter by verification status
		 */
		async loadFindings(scanJobId: string, filter?: FindingFilter): Promise<void> {
			state.loading = true;
			state.error = null;

			try {
				const findings = await scanAPI.getFindings(scanJobId, filter);
				state.findings = findings;
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to load findings';
				console.error('Load findings error:', err);
			} finally {
				state.loading = false;
			}
		},

		/**
		 * Verify or reject a finding
		 *
		 * @param findingId - The finding ID to verify
		 * @param isMatch - True to confirm, false to reject
		 */
		async verifyFinding(findingId: string, isMatch: boolean): Promise<void> {
			state.error = null;

			try {
				await scanAPI.verify(findingId, isMatch);

				// Update after success (pessimistic)
				state.findings = state.findings.map((f) =>
					f.id === findingId ? { ...f, verification_status: isMatch ? 'Confirmed' : 'Rejected' } : f
				);
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to verify finding';
				console.error('Verify finding error:', err);
			}
		},

		/**
		 * Submit removal requests for all confirmed findings
		 *
		 * @param scanJobId - The scan job ID
		 * @returns Count of removal requests submitted
		 */
		async submitRemovals(scanJobId: string): Promise<number> {
			state.loading = true;
			state.error = null;

			try {
				const removalIds = await scanAPI.submitRemovals(scanJobId);
				return removalIds.length;
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to submit removals';
				console.error('Submit removals error:', err);
				return 0;
			} finally {
				state.loading = false;
			}
		},

		/**
		 * Reset the entire scan store state
		 * Stops polling and clears all data
		 */
		reset(): void {
			this.stopPolling();
			state.currentScanId = null;
			state.scanStatus = null;
			state.findings = [];
			state.loading = false;
			state.error = null;
			state.isPolling = false;
		}
	};
}

/**
 * Global scan store instance
 */
export const scanStore = createScanStore();
