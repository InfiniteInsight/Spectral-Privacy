/**
 * Discovery API - Local PII discovery commands
 */

import { invoke } from '@tauri-apps/api/core';

export interface DiscoveryFinding {
	id: string;
	source: string;
	source_detail: string;
	finding_type: string;
	risk_level: 'critical' | 'high' | 'medium' | 'low';
	description: string;
	recommended_action: string | null;
	pii_type: 'email' | 'phone' | 'ssn' | 'address' | 'name' | 'dob';
	remediated: boolean;
	ignored: boolean;
	still_present_after_remediation: boolean;
	found_at: string;
	matched_value?: string | null;
	line_number?: number | null;
}

export interface ScanConfig {
	scan_emails: boolean;
	scan_phones: boolean;
	scan_ssn: boolean;
	scan_addresses: boolean;
	scan_names: boolean;
	scan_dob: boolean;
}

export interface ScanProgress {
	session_id: string;
	files_scanned: number;
	files_with_findings: number;
	current_directory: string;
	is_complete: boolean;
	was_stopped: boolean;
}

/**
 * Start a discovery scan with PII type configuration
 */
export async function startDiscoveryScan(vaultId: string, config: ScanConfig): Promise<string> {
	return invoke('start_discovery_scan', { vaultId, config });
}

/**
 * Stop the current discovery scan
 */
export async function stopDiscoveryScan(): Promise<void> {
	return invoke('stop_discovery_scan');
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
 * Clear all discovery findings and scan history for a vault
 */
export async function clearDiscoveryResults(vaultId: string): Promise<void> {
	return invoke('clear_discovery_results', { vaultId });
}

/**
 * Delete a file from the filesystem
 */
export async function deleteFile(filePath: string): Promise<void> {
	return invoke('delete_file', { filePath });
}

/**
 * Open the folder containing a file
 */
export async function openFileLocation(filePath: string): Promise<void> {
	return invoke('open_file_location', { filePath });
}

/**
 * Get the scan log for a session
 */
export async function getScanLog(vaultId: string, sessionId: string): Promise<string> {
	return invoke('get_scan_log', { vaultId, sessionId });
}
