import { invoke } from '@tauri-apps/api/core';

export interface BrokerSummary {
	id: string;
	name: string;
	domain: string;
	category: string;
	difficulty: string;
	typical_removal_days: number;
}

export interface EmailTemplate {
	email: string;
	subject: string;
	body: string;
	response_days: number;
	notes: string | null;
}

export interface BrokerDetail {
	id: string;
	name: string;
	domain: string;
	category: string;
	difficulty: string;
	typical_removal_days: number;
	removal_method: string;
	url: string;
	recheck_interval_days: number;
	last_verified: string;
	scan_status: string | null;
	finding_count: number | null;
	email_template: EmailTemplate | null;
}

export const brokerAPI = {
	/**
	 * List all broker definitions
	 */
	async listBrokers(): Promise<BrokerSummary[]> {
		return await invoke<BrokerSummary[]>('list_brokers');
	},

	/**
	 * Get detailed information about a specific broker
	 */
	async getBrokerDetail(brokerId: string, vaultId: string): Promise<BrokerDetail> {
		return await invoke<BrokerDetail>('get_broker_detail', {
			brokerId,
			vaultId
		});
	}
};
