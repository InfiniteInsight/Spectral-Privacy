import { invoke } from '@tauri-apps/api/core';

export interface PrivacyScoreResult {
	score: number;
	descriptor: string;
	unresolved_count: number;
	confirmed_count: number;
	failed_count: number;
}

export async function getPrivacyScore(vaultId: string): Promise<PrivacyScoreResult> {
	return await invoke<PrivacyScoreResult>('get_privacy_score', { vaultId });
}
