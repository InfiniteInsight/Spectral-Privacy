import { invoke } from '@tauri-apps/api/core';

export interface ScanJobStatus {
	id: string;
	status: 'InProgress' | 'Completed' | 'Failed' | 'Cancelled';
	completed_brokers: number;
	total_brokers: number;
	error_message?: string;
}

export interface Finding {
	id: string;
	broker_id: string;
	listing_url: string;
	verification_status: 'PendingVerification' | 'Confirmed' | 'Rejected';
	extracted_data: ExtractedData;
	discovered_at: string;
}

export interface ExtractedData {
	name?: string;
	age?: number;
	addresses: string[];
	phone_numbers: string[];
	relatives: string[];
	emails: string[];
}

export const scanAPI = {
	/**
	 * Start a new scan job
	 */
	async start(vaultId: string, profileId: string, brokerFilter?: string): Promise<ScanJobStatus> {
		return await invoke<ScanJobStatus>('start_scan', {
			vaultId,
			profileId,
			brokerFilter
		});
	},

	/**
	 * Get scan job status
	 */
	async getStatus(vaultId: string, scanJobId: string): Promise<ScanJobStatus> {
		return await invoke<ScanJobStatus>('get_scan_status', {
			vaultId,
			scanJobId
		});
	},

	/**
	 * Get findings for a scan job
	 */
	async getFindings(
		vaultId: string,
		scanJobId: string,
		filter?: 'PendingVerification' | 'Confirmed' | 'Rejected'
	): Promise<Finding[]> {
		return await invoke<Finding[]>('get_findings', {
			vaultId,
			scanJobId,
			filter
		});
	},

	/**
	 * Verify a finding
	 */
	async verify(vaultId: string, findingId: string, isMatch: boolean): Promise<void> {
		return await invoke('verify_finding', {
			vaultId,
			findingId,
			isMatch
		});
	},

	/**
	 * Submit removal requests for all confirmed findings
	 */
	async submitRemovals(vaultId: string, scanJobId: string): Promise<string[]> {
		return await invoke<string[]>('submit_removals_for_confirmed', {
			vaultId,
			scanJobId
		});
	}
};
