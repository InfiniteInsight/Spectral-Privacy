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
import { vaultStore } from './vault.svelte';

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

	// Computed property for current vault ID
	const currentVaultId = $derived(vaultStore.currentVaultId);

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
		get currentVaultId() {
			return currentVaultId;
		},

		/**
		 * Load all profiles from the current vault
		 * Requires a vault to be selected and unlocked
		 */
		async loadProfiles(): Promise<void> {
			if (!currentVaultId) {
				state.error = 'No vault selected';
				return;
			}

			state.loading = true;
			state.error = null;

			try {
				state.profiles = await profileAPI.list(currentVaultId);
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
		 * @param profileId - The profile ID to load
		 */
		async loadProfile(profileId: string): Promise<void> {
			if (!currentVaultId) {
				state.error = 'No vault selected';
				return;
			}

			state.loading = true;
			state.error = null;

			try {
				state.currentProfile = await profileAPI.get(currentVaultId, profileId);
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
		 * @param input - Profile data to create
		 * @returns {ProfileOutput | null} The created profile or null on error
		 */
		async createProfile(input: ProfileInput): Promise<ProfileOutput | null> {
			if (!currentVaultId) {
				state.error = 'No vault selected';
				return null;
			}

			state.loading = true;
			state.error = null;

			try {
				const profile = await profileAPI.create(currentVaultId, input);
				state.currentProfile = profile;

				// Reload profiles list to include the new profile
				await this.loadProfiles();

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
		 * @param profileId - The profile ID to update
		 * @param input - Updated profile data
		 * @returns {ProfileOutput | null} The updated profile or null on error
		 */
		async updateProfile(profileId: string, input: ProfileInput): Promise<ProfileOutput | null> {
			if (!currentVaultId) {
				state.error = 'No vault selected';
				return null;
			}

			state.loading = true;
			state.error = null;

			try {
				const profile = await profileAPI.update(currentVaultId, profileId, input);
				state.currentProfile = profile;

				// Reload profiles list to reflect the update
				await this.loadProfiles();

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
