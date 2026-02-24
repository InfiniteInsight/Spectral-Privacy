/**
 * Audit log API for privacy transparency.
 */

import { invoke } from '@tauri-apps/api/core';

export interface AuditLogEntry {
	id: string;
	timestamp: string;
	event_type: string;
	subject: string;
	pii_fields: string[] | null;
	data_destination: string;
	outcome: string;
}

export const auditAPI = {
	/**
	 * Get all audit log entries for a vault.
	 */
	async getAuditLog(vaultId: string, limit?: number): Promise<AuditLogEntry[]> {
		return await invoke<AuditLogEntry[]>('get_audit_log', {
			vaultId,
			limit: limit ?? null
		});
	},

	/**
	 * Get audit log entries filtered by event type.
	 */
	async getAuditLogByType(
		vaultId: string,
		eventType: string,
		limit?: number
	): Promise<AuditLogEntry[]> {
		return await invoke<AuditLogEntry[]>('get_audit_log_by_type', {
			vaultId,
			eventType,
			limit: limit ?? null
		});
	},

	/**
	 * Create a new audit log entry.
	 */
	async createAuditEntry(
		vaultId: string,
		eventType: string,
		subject: string,
		piiFields: string[] | null,
		dataDestination: string,
		outcome: string
	): Promise<AuditLogEntry> {
		return await invoke<AuditLogEntry>('create_audit_entry', {
			vaultId,
			eventType,
			subject,
			piiFields,
			dataDestination,
			outcome
		});
	},

	/**
	 * Clear all audit log entries for a vault.
	 */
	async clearAuditLog(vaultId: string): Promise<number> {
		return await invoke<number>('clear_audit_log', { vaultId });
	}
};
