import { invoke } from '@tauri-apps/api/core';

export async function testSmtpConnection(
	host: string,
	port: number,
	username: string,
	password: string
): Promise<void> {
	return await invoke('test_smtp_connection', { host, port, username, password });
}

export async function testImapConnection(
	host: string,
	port: number,
	username: string,
	password: string
): Promise<void> {
	return await invoke('test_imap_connection', { host, port, username, password });
}

export interface ScheduledJob {
	id: string;
	job_type: 'ScanAll' | 'VerifyRemovals' | 'PollImap';
	interval_days: number;
	next_run_at: string;
	last_run_at: string | null;
	enabled: boolean;
}

export async function getScheduledJobs(vaultId: string): Promise<ScheduledJob[]> {
	return invoke('get_scheduled_jobs', { vaultId });
}

export async function updateScheduledJob(
	vaultId: string,
	jobId: string,
	intervalDays: number,
	enabled: boolean
): Promise<void> {
	return invoke('update_scheduled_job', { vaultId, jobId, intervalDays, enabled });
}

export async function runJobNow(vaultId: string, jobType: string): Promise<void> {
	return invoke('run_job_now', { vaultId, jobType });
}
