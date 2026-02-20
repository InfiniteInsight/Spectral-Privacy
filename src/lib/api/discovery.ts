/**
 * Discovery API - Local PII discovery commands
 */

import { invoke } from '@tauri-apps/api/core';

export interface DiscoveryFinding {
	id: string;
	source: 'filesystem' | 'browser' | 'email';
	source_detail: string;
	finding_type: 'pii_exposure' | 'broker_contact' | 'broker_account';
	risk_level: 'critical' | 'medium' | 'informational';
	description: string;
	recommended_action: string | null;
	remediated: boolean;
	found_at: string;
}

/**
 * Start a discovery scan of local files
 * Scans common user directories for PII
 */
export async function startDiscoveryScan(vaultId: string): Promise<string> {
	return invoke('start_discovery_scan', { vaultId });
}

/**
 * Get all discovery findings for a vault
 */
export async function getDiscoveryFindings(vaultId: string): Promise<DiscoveryFinding[]> {
	return invoke('get_discovery_findings', { vaultId });
}

/**
 * Mark a finding as remediated
 */
export async function markFindingRemediated(vaultId: string, findingId: string): Promise<void> {
	return invoke('mark_finding_remediated', { vaultId, findingId });
}
