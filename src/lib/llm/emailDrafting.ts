/**
 * Email drafting with LLM integration.
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * Email draft request parameters.
 */
export interface EmailDraftRequest {
	/** The context or instructions for the email */
	prompt: string;
	/** Optional recipient information */
	recipient?: string;
	/** Optional subject hint */
	subject?: string;
	/** Optional tone preference (e.g., "formal", "casual") */
	tone?: string;
}

/**
 * Email draft response.
 */
export interface EmailDraftResponse {
	/** The generated email subject */
	subject: string;
	/** The generated email body */
	body: string;
	/** Optional metadata about the generation */
	metadata?: {
		provider: string;
		pii_filtered: boolean;
	};
}

/**
 * Draft an email using LLM.
 *
 * @param vaultId - The vault ID to use for privacy settings
 * @param request - Email draft request parameters
 * @returns The generated email draft
 */
export async function draftEmail(
	vaultId: string,
	request: EmailDraftRequest
): Promise<EmailDraftResponse> {
	return invoke('draft_email', {
		vaultId,
		request
	});
}
