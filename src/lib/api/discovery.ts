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
	pii_type?: 'email' | 'phone' | 'ssn';
	remediated: boolean;
	ignored: boolean;
	still_present_after_remediation: boolean;
	found_at: string;
	matched_value?: string | null;
	line_number?: number | null;
}

/**
 * Start a discovery scan of local files
 * Scans entire user profile by default, or custom directories if specified
 */
export async function startDiscoveryScan(
	vaultId: string,
	customDirectories?: string[]
): Promise<string> {
	return invoke('start_discovery_scan', {
		vaultId,
		customDirectories: customDirectories ?? null
	});
}

/**
 * Get all discovery findings for a vault
 * @param includeIgnored - If true, includes ignored findings; if false, excludes them
 */
export async function getDiscoveryFindings(
	vaultId: string,
	includeIgnored?: boolean
): Promise<DiscoveryFinding[]> {
	return invoke('get_discovery_findings', { vaultId, includeIgnored });
}

/**
 * Mark a finding as remediated
 */
export async function markFindingRemediated(vaultId: string, findingId: string): Promise<void> {
	return invoke('mark_finding_remediated', { vaultId, findingId });
}

/**
 * Mark a finding as ignored (false positive or acceptable)
 */
export async function markFindingIgnored(
	vaultId: string,
	findingId: string,
	ignored: boolean
): Promise<void> {
	return invoke('mark_finding_ignored', { vaultId, findingId, ignored });
}

/**
 * Pause the current discovery scan
 */
export async function pauseDiscoveryScan(): Promise<void> {
	return invoke('pause_discovery_scan');
}

/**
 * Resume a paused discovery scan
 */
export async function resumeDiscoveryScan(): Promise<void> {
	return invoke('resume_discovery_scan');
}

/**
 * Stop the current discovery scan
 */
export async function stopDiscoveryScan(): Promise<void> {
	return invoke('stop_discovery_scan');
}

/**
 * Open the folder containing a file
 */
export async function openFileLocation(filePath: string): Promise<void> {
	return invoke('open_file_location', { filePath });
}
