/**
 * Privacy Settings API
 *
 * Type-safe wrappers for privacy-related Tauri commands.
 * Provides functionality for managing privacy levels, feature flags, and LLM provider settings.
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * Privacy level presets
 */
export type PrivacyLevel = 'paranoid' | 'local_privacy' | 'balanced' | 'custom';

/**
 * Granular feature control flags
 */
export interface FeatureFlags {
	allow_local_llm: boolean;
	allow_cloud_llm: boolean;
	allow_browser_automation: boolean;
	allow_email_sending: boolean;
	allow_imap_monitoring: boolean;
	allow_pii_scanning: boolean;
}

/**
 * Privacy settings response
 */
export interface PrivacySettings {
	privacy_level: PrivacyLevel;
	feature_flags: FeatureFlags;
}

/**
 * Supported LLM providers
 */
export type LlmProvider = 'open-ai' | 'gemini' | 'claude' | 'ollama' | 'lm-studio';

/**
 * Task types that can have provider preferences
 */
export type TaskType = 'email-draft' | 'form-fill';

/**
 * LLM provider settings response
 */
export interface LlmProviderSettings {
	primary_provider: LlmProvider | null;
	email_draft_provider: LlmProvider | null;
	form_fill_provider: LlmProvider | null;
	has_openai_key: boolean;
	has_gemini_key: boolean;
	has_claude_key: boolean;
}

/**
 * Get current privacy settings
 *
 * @param vaultId - The ID of the vault to get settings from
 * @returns Privacy level and feature flags
 */
export async function getPrivacySettings(vaultId: string): Promise<PrivacySettings> {
	return invoke('get_privacy_settings', { vaultId });
}

/**
 * Set privacy level
 *
 * Updates the privacy level and automatically updates feature flags if not Custom.
 *
 * @param vaultId - The ID of the vault to update
 * @param level - The privacy level to set
 */
export async function setPrivacyLevel(vaultId: string, level: PrivacyLevel): Promise<void> {
	return invoke('set_privacy_level', { vaultId, level });
}

/**
 * Set custom feature flags
 *
 * Only takes effect when privacy level is set to Custom.
 *
 * @param vaultId - The ID of the vault to update
 * @param flags - The feature flags to set
 */
export async function setCustomFeatureFlags(vaultId: string, flags: FeatureFlags): Promise<void> {
	return invoke('set_custom_feature_flags', { vaultId, flags });
}

/**
 * Get LLM provider settings
 *
 * Returns the primary provider, task-specific providers, and which providers have API keys configured.
 *
 * @param vaultId - The ID of the vault to get settings from
 * @returns LLM provider settings
 */
export async function getLlmProviderSettings(vaultId: string): Promise<LlmProviderSettings> {
	return invoke('get_llm_provider_settings', { vaultId });
}

/**
 * Set primary LLM provider
 *
 * Sets the default provider to use when no task-specific preference is configured.
 *
 * @param vaultId - The ID of the vault to update
 * @param provider - The provider to set as primary
 */
export async function setLlmPrimaryProvider(vaultId: string, provider: LlmProvider): Promise<void> {
	return invoke('set_llm_primary_provider', { vaultId, provider });
}

/**
 * Set task-specific LLM provider
 *
 * Sets the provider to use for a specific task type.
 *
 * @param vaultId - The ID of the vault to update
 * @param taskType - The task type to configure
 * @param provider - The provider to use for this task type
 */
export async function setLlmTaskProvider(
	vaultId: string,
	taskType: TaskType,
	provider: LlmProvider
): Promise<void> {
	return invoke('set_llm_task_provider', { vaultId, taskType, provider });
}

/**
 * Set API key for an LLM provider
 *
 * Stores the API key encrypted in the vault database.
 *
 * @param vaultId - The ID of the vault to update
 * @param provider - The provider to set the API key for
 * @param apiKey - The API key to store
 */
export async function setLlmApiKey(
	vaultId: string,
	provider: LlmProvider,
	apiKey: string
): Promise<void> {
	return invoke('set_llm_api_key', { vaultId, provider, apiKey });
}

/**
 * Test connection to an LLM provider
 *
 * Attempts to connect to the provider and make a simple test request.
 *
 * @param vaultId - The ID of the vault to test from
 * @param provider - The provider to test
 * @returns Success message or error
 */
export async function testLlmProvider(vaultId: string, provider: LlmProvider): Promise<string> {
	return invoke('test_llm_provider', { vaultId, provider });
}
