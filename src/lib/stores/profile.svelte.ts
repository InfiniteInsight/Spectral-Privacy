/**
 * Profile Store - Reactive state management for profile operations
 *
 * Manages profile state including:
 * - Current profile selection
 * - Profiles list
 * - Loading and error states
 * - CRUD operations
 *
 * Uses Svelte 5 runes for reactive state management.
 *
 * @module $lib/stores/profile
 */

import { profileAPI, type ProfileInput, type ProfileOutput, type ProfileSummary } from '$lib/api';

/**
 * Profile state interface
 */
interface ProfileStoreState {
	profiles: ProfileSummary[];
	currentProfile: ProfileOutput | null;
	loading: boolean;
	error: string | null;
}

/**
 * Create a reactive profile store using Svelte 5 runes
 *
 * @returns Profile store with getters and actions
 */
function createProfileStore() {
	// State using Svelte 5 runes
	let state = $state<ProfileStoreState>({
		profiles: [],
		currentProfile: null,
		loading: false,
		error: null
	});

	return {
		// Getters for reactive access
		get profiles() {
			return state.profiles;
		},
		get currentProfile() {
			return state.currentProfile;
		},
		get loading() {
			return state.loading;
		},
		get error() {
			return state.error;
		},

		/**
		 * Load all profiles from the current vault
		 * Requires a vault to be selected and unlocked
		 *
		 * @param vaultId - The vault ID to load profiles from
		 */
		async loadProfiles(vaultId: string): Promise<void> {
			state.loading = true;
			state.error = null;

			try {
				state.profiles = await profileAPI.list(vaultId);
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to load profiles';
				console.error('Load profiles error:', err);
			} finally {
				state.loading = false;
			}
		},

		/**
		 * Load a specific profile by ID
		 *
		 * @param vaultId - The vault ID
		 * @param profileId - The profile ID to load
		 */
		async loadProfile(vaultId: string, profileId: string): Promise<void> {
			state.loading = true;
			state.error = null;

			try {
				state.currentProfile = await profileAPI.get(vaultId, profileId);
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to load profile';
				console.error('Load profile error:', err);
			} finally {
				state.loading = false;
			}
		},

		/**
		 * Create a new profile in the current vault
		 *
		 * @param vaultId - The vault ID to create the profile in
		 * @param input - Profile data to create
		 * @returns {ProfileOutput | null} The created profile or null on error
		 */
		async createProfile(vaultId: string, input: ProfileInput): Promise<ProfileOutput | null> {
			state.loading = true;
			state.error = null;

			try {
				const profile = await profileAPI.create(vaultId, input);
				state.currentProfile = profile;

				// Reload profiles list to include the new profile
				await this.loadProfiles(vaultId);

				return profile;
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to create profile';
				console.error('Create profile error:', err);
				return null;
			} finally {
				state.loading = false;
			}
		},

		/**
		 * Update an existing profile
		 *
		 * @param vaultId - The vault ID
		 * @param profileId - The profile ID to update
		 * @param input - Updated profile data
		 * @returns {ProfileOutput | null} The updated profile or null on error
		 */
		async updateProfile(
			vaultId: string,
			profileId: string,
			input: ProfileInput
		): Promise<ProfileOutput | null> {
			state.loading = true;
			state.error = null;

			try {
				const profile = await profileAPI.update(vaultId, profileId, input);
				state.currentProfile = profile;

				// Reload profiles list to reflect the update
				await this.loadProfiles(vaultId);

				return profile;
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to update profile';
				console.error('Update profile error:', err);
				return null;
			} finally {
				state.loading = false;
			}
		},

		/**
		 * Clear any error state
		 */
		clearError(): void {
			state.error = null;
		},

		/**
		 * Clear the currently selected profile
		 */
		clearCurrentProfile(): void {
			state.currentProfile = null;
		},

		/**
		 * Reset store state — call when switching vaults
		 * to prevent stale data from previous vault accumulating
		 */
		reset(): void {
			state.profiles = [];
			state.currentProfile = null;
			state.loading = false;
			state.error = null;
		}
	};
}

/**
 * Global profile store instance
 */
export const profileStore = createProfileStore();
