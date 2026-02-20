/**
 * Profile API Wrappers
 *
 * Type-safe wrappers for profile-related Tauri commands.
 * Provides functionality for creating, reading, updating, and listing profiles.
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * Email address with type classification
 */
export interface EmailAddress {
	email: string; // Display format as entered by user
	email_normalized?: string; // Normalized format for matching (lowercase)
	email_type: 'Personal' | 'Work' | 'Other';
}

/**
 * Phone number with type classification
 */
export interface PhoneNumber {
	number: string; // Display format as entered by user
	number_normalized?: string; // Normalized format for matching (digits only)
	phone_type: 'Mobile' | 'Home' | 'Work';
}

/**
 * Previous address with date range
 */
export interface PreviousAddress {
	address_line1: string;
	address_line2?: string;
	city: string;
	state: string;
	zip_code: string;
	lived_from?: string; // YYYY-MM-DD
	lived_to?: string; // YYYY-MM-DD
}

/**
 * Alias or alternative name
 */
export interface Alias {
	first_name?: string;
	middle_name?: string;
	last_name?: string;
	nickname?: string;
}

/**
 * Relative or family member
 */
export interface Relative {
	first_name?: string;
	middle_name?: string;
	last_name?: string;
	maiden_name?: string;
	relationship: 'Spouse' | 'Partner' | 'Parent' | 'Child' | 'Sibling' | 'Other';
}

/**
 * Profile completeness metrics
 */
export interface ProfileCompleteness {
	score: number;
	max_score: number;
	percentage: number;
	tier: 'Minimal' | 'Basic' | 'Good' | 'Excellent';
	message: string;
}

/**
 * Profile input data for create/update operations
 */
export interface ProfileInput {
	// Phase 1 fields
	first_name: string;
	middle_name?: string;
	last_name: string;
	email: string;
	date_of_birth?: string; // ISO 8601 date string (YYYY-MM-DD)
	address_line1: string;
	address_line2?: string;
	city: string;
	state: string; // US state code (e.g., "CA")
	zip_code: string;

	// Phase 2 fields
	phone_numbers?: PhoneNumber[];
	email_addresses?: EmailAddress[];
	previous_addresses?: PreviousAddress[];
	aliases?: Alias[];
	relatives?: Relative[];
}

/**
 * Complete profile data returned from backend
 */
export interface ProfileOutput {
	id: string;
	first_name: string;
	middle_name?: string;
	last_name: string;
	email: string;
	date_of_birth?: string; // ISO 8601 date string (YYYY-MM-DD)
	address_line1: string;
	address_line2?: string;
	city: string;
	state: string;
	zip_code: string;
	created_at: string; // RFC3339 timestamp
	updated_at: string; // RFC3339 timestamp
}

/**
 * Lightweight profile summary for listings
 */
export interface ProfileSummary {
	id: string;
	full_name: string;
	email: string;
	created_at: string; // RFC3339 timestamp
}

/**
 * Profile API for CRUD operations
 */
export const profileAPI = {
	/**
	 * Create a new profile
	 *
	 * @param vaultId - The vault ID to create the profile in
	 * @param input - Profile data to create
	 * @returns {ProfileOutput} The created profile with generated ID and timestamps
	 * @throws {CommandError} If validation fails or vault is not unlocked
	 */
	async create(vaultId: string, input: ProfileInput): Promise<ProfileOutput> {
		return await invoke<ProfileOutput>('profile_create', { vaultId, input });
	},

	/**
	 * Get a profile by ID
	 *
	 * @param vaultId - The vault ID containing the profile
	 * @param profileId - The profile ID to retrieve
	 * @returns {ProfileOutput} The profile data with decrypted fields
	 * @throws {CommandError} If profile not found or vault is not unlocked
	 */
	async get(vaultId: string, profileId: string): Promise<ProfileOutput> {
		return await invoke<ProfileOutput>('profile_get', { vaultId, profileId });
	},

	/**
	 * Update an existing profile
	 *
	 * @param vaultId - The vault ID containing the profile
	 * @param profileId - The profile ID to update
	 * @param input - Updated profile data
	 * @returns {ProfileOutput} The updated profile with new timestamp
	 * @throws {CommandError} If validation fails, profile not found, or vault is not unlocked
	 */
	async update(vaultId: string, profileId: string, input: ProfileInput): Promise<ProfileOutput> {
		return await invoke<ProfileOutput>('profile_update', {
			vaultId,
			profileId,
			input
		});
	},

	/**
	 * List all profiles in the vault
	 *
	 * @param vaultId - The vault ID to list profiles from
	 * @returns {ProfileSummary[]} Array of profile summaries
	 * @throws {CommandError} If vault is not unlocked
	 */
	async list(vaultId: string): Promise<ProfileSummary[]> {
		return await invoke<ProfileSummary[]>('profile_list', { vaultId });
	}
};

/**
 * Get profile completeness score
 *
 * @returns {ProfileCompleteness} Completeness metrics for the profile
 * @throws {CommandError} If vault is not unlocked or no profile exists
 */
export async function getProfileCompleteness(): Promise<ProfileCompleteness> {
	return invoke<ProfileCompleteness>('get_profile_completeness');
}
