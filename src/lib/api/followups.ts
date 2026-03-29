import { invoke } from '@tauri-apps/api/core';

export interface PendingFollowup {
	id: string;
	attempt_id: string;
	broker_id: string;
	/** Email address the follow-up will be sent to. */
	recipient: string;
	/** ISO-8601 datetime when the follow-up is due. */
	follow_up_at: string;
}

/** Return all pending (unsent, undismissed) follow-ups for the vault. */
export async function getPendingFollowups(vaultId: string): Promise<PendingFollowup[]> {
	return invoke('get_pending_followups', { vaultId });
}

/** Mark a follow-up as dismissed (user handled it manually). */
export async function dismissFollowup(vaultId: string, followupId: string): Promise<void> {
	return invoke('dismiss_followup', { vaultId, followupId });
}
