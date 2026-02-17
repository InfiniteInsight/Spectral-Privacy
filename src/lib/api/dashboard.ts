import { invoke } from '@tauri-apps/api/core';

export interface RemovalCounts {
	submitted: number;
	pending: number;
	failed: number;
}

export interface ActivityEvent {
	event_type: string;
	timestamp: string;
	description: string;
}

export interface DashboardSummary {
	privacy_score: number | null;
	brokers_scanned: number;
	brokers_total: number;
	last_scan_at: string | null;
	active_removals: RemovalCounts;
	recent_events: ActivityEvent[];
}

export async function getDashboardSummary(vaultId: string): Promise<DashboardSummary> {
	return await invoke<DashboardSummary>('get_dashboard_summary', { vaultId });
}
