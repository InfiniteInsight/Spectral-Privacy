/**
 * Form filling with LLM integration.
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * A form field to be filled.
 */
export interface FormField {
	/** Field identifier or name */
	name: string;
	/** Field label or description */
	label: string;
	/** Field type (e.g., "text", "email", "phone", "address") */
	type: string;
	/** Whether this field is required */
	required?: boolean;
}

/**
 * Form filling request parameters.
 */
export interface FormFillingRequest {
	/** The form fields to fill */
	fields: FormField[];
	/** Optional context about the form or purpose */
	context?: string;
}

/**
 * Form filling response.
 */
export interface FormFillingResponse {
	/** Filled field values, keyed by field name */
	values: Record<string, string>;
	/** Optional metadata about the generation */
	metadata?: {
		provider: string;
		pii_filtered: boolean;
		fields_filled: number;
	};
}

/**
 * Fill a form using LLM.
 *
 * @param vaultId - The vault ID to use for privacy settings
 * @param request - Form filling request parameters
 * @returns The filled form values
 */
export async function fillForm(
	vaultId: string,
	request: FormFillingRequest
): Promise<FormFillingResponse> {
	return invoke('fill_form', {
		vaultId,
		request
	});
}
