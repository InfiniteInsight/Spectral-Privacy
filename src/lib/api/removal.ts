import { invoke } from '@tauri-apps/api/core';

export interface BatchSubmissionResult {
	job_id: string;
	total_count: number;
	queued_count: number;
}

export interface RemovalAttempt {
	id: string;
	finding_id: string;
	broker_id: string;
	status: 'Pending' | 'Processing' | 'Submitted' | 'Completed' | 'Failed';
	created_at: string;
	submitted_at: string | null;
	completed_at: string | null;
	error_message: string | null;
}

export const removalAPI = {
	/**
	 * Process a batch of removal attempts
	 */
	async processBatch(vaultId: string, removalAttemptIds: string[]): Promise<BatchSubmissionResult> {
		return await invoke<BatchSubmissionResult>('process_removal_batch', {
			vaultId,
			removalAttemptIds
		});
	},

	/**
	 * Get CAPTCHA queue
	 */
	async getCaptchaQueue(vaultId: string): Promise<RemovalAttempt[]> {
		return await invoke<RemovalAttempt[]>('get_captcha_queue', { vaultId });
	},

	/**
	 * Get failed queue
	 */
	async getFailedQueue(vaultId: string): Promise<RemovalAttempt[]> {
		return await invoke<RemovalAttempt[]>('get_failed_queue', { vaultId });
	},

	/**
	 * Retry a failed removal
	 */
	async retry(vaultId: string, removalAttemptId: string): Promise<void> {
		return await invoke('retry_removal', { vaultId, removalAttemptId });
	},

	/**
	 * Get removal attempts by scan job ID
	 */
	async getByScanJob(vaultId: string, scanJobId: string): Promise<RemovalAttempt[]> {
		return await invoke<RemovalAttempt[]>('get_removal_attempts_by_scan_job', {
			vaultId,
			scanJobId
		});
	}
};

export interface RemovalJobSummary {
	scan_job_id: string;
	submitted_at: string;
	total: number;
	submitted_count: number;
	completed_count: number;
	failed_count: number;
	pending_count: number;
}

export async function getJobHistory(vaultId: string): Promise<RemovalJobSummary[]> {
	return await invoke<RemovalJobSummary[]>('get_removal_job_history', { vaultId });
}
