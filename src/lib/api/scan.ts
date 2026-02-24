import { invoke } from '@tauri-apps/api/core';

export interface ScanJobStatus {
	id: string;
	status: 'InProgress' | 'Completed' | 'Failed' | 'Cancelled';
	completed_brokers: number;
	total_brokers: number;
	current_broker_name?: string;
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

export interface PossibleMatch {
	finding: Finding;
	similarity_score: number;
	name_similarity: number;
	location_matched: boolean;
	source_broker_id: string;
}

export interface ZeroResultBroker {
	broker_id: string;
	possible_matches: PossibleMatch[];
}

export interface GoogleRemovalRequest {
	id: string;
	finding_id: string;
	status: 'URLGenerated' | 'Submitted' | 'Completed' | 'Failed';
	google_removal_url: string;
	generated_at: string;
	submitted_at?: string;
	completed_at?: string;
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
	},

	/**
	 * Get possible matches for zero-result brokers
	 */
	async getPossibleMatches(vaultId: string, scanJobId: string): Promise<ZeroResultBroker[]> {
		return await invoke<ZeroResultBroker[]>('get_possible_matches', {
			vaultId,
			scanJobId
		});
	},

	/**
	 * Accept a possible match and create a finding for the zero-result broker
	 */
	async acceptMatch(
		vaultId: string,
		scanJobId: string,
		zeroResultBrokerId: string,
		matchedFindingId: string
	): Promise<Finding> {
		return await invoke<Finding>('accept_possible_match', {
			vaultId,
			scanJobId,
			zeroResultBrokerId,
			matchedFindingId
		});
	},

	/**
	 * Dismiss a possible match
	 */
	async dismissMatch(
		vaultId: string,
		zeroResultBrokerId: string,
		matchedFindingId: string
	): Promise<void> {
		return await invoke('dismiss_possible_match', {
			vaultId,
			zeroResultBrokerId,
			matchedFindingId
		});
	},

	/**
	 * Get Google removal request for a finding
	 */
	async getGoogleRemovalRequest(
		vaultId: string,
		findingId: string
	): Promise<GoogleRemovalRequest | null> {
		return await invoke<GoogleRemovalRequest | null>('get_google_removal_request', {
			vaultId,
			findingId
		});
	},

	/**
	 * Mark a Google removal request as submitted by the user
	 */
	async markGoogleRemovalSubmitted(
		vaultId: string,
		requestId: string,
		notes?: string
	): Promise<void> {
		return await invoke('mark_google_removal_submitted', {
			vaultId,
			requestId,
			notes
		});
	}
};

/**
 * Start a new scan with tier-based or custom broker selection
 */
export async function startScan(
	vaultId: string,
	profileId: string,
	options: { tier?: 'Tier1' | 'Tier2' | 'All'; brokerIds?: string[] } = {}
): Promise<string> {
	const result = await invoke<ScanJobStatus>('start_scan', {
		vaultId,
		profileId,
		brokerFilter: null,
		tier: options.tier ?? null,
		brokerIds: options.brokerIds ?? null
	});
	return result.id;
}
