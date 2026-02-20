import { invoke } from '@tauri-apps/api/core';

export async function markAttemptVerified(vaultId: string, attemptId: string): Promise<void> {
	return invoke('mark_attempt_verified', { vaultId, attemptId });
}
