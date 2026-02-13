<script lang="ts">
	import { UnlockScreen } from '$lib/components';
	import { vaultStore, profileStore } from '$lib/stores';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	onMount(async () => {
		// Load profiles when vault is unlocked
		if (vaultStore.isCurrentVaultUnlocked) {
			await profileStore.loadProfiles();

			// If no profile exists, redirect to setup
			if (profileStore.profiles.length === 0) {
				goto('/profile/setup');
			}
		}
	});

	// Get current profile (first profile for now)
	const currentProfile = $derived(
		profileStore.profiles.length > 0 ? profileStore.profiles[0] : null
	);
</script>

{#if vaultStore.isCurrentVaultUnlocked}
	<!-- Dashboard Content -->
	<div
		class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 flex items-center justify-center p-4"
	>
		<div class="bg-white rounded-lg shadow-xl p-8 max-w-2xl w-full">
			<h1 class="text-3xl font-bold text-gray-900 mb-4">Spectral Dashboard</h1>
			<p class="text-gray-600 mb-6">Automated data broker removal</p>

			{#if profileStore.loading}
				<div class="flex items-center justify-center py-8">
					<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
				</div>
			{:else if currentProfile}
				<!-- Profile Info -->
				<div class="mb-6 p-4 bg-blue-50 border border-blue-200 rounded-md">
					<h2 class="text-lg font-semibold text-gray-900 mb-2">Your Profile</h2>
					<dl class="space-y-1 text-sm">
						<div>
							<dt class="text-gray-600 inline">Name:</dt>
							<dd class="text-gray-900 inline ml-2">{currentProfile.full_name}</dd>
						</div>
						<div>
							<dt class="text-gray-600 inline">Email:</dt>
							<dd class="text-gray-900 inline ml-2">{currentProfile.email}</dd>
						</div>
					</dl>
				</div>

				<!-- Status Badge -->
				<div
					class="inline-flex items-center px-4 py-2 bg-primary-100 text-primary-700 rounded-full text-sm font-medium mb-4"
					style="background-color: #e0f2fe; color: #0369a1; padding: 0.5rem 1rem; border-radius: 9999px; font-size: 0.875rem; font-weight: 500; display: inline-flex; align-items: center;"
				>
					✓ Vault Unlocked
				</div>

				<!-- Scan for Data -->
				<div class="mt-6">
					<a
						href="/scan/start"
						class="block px-6 py-4 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors text-center"
						style="background-color: #0284c7; color: white; display: block; text-align: center;"
					>
						Scan for Your Data
					</a>
					<p class="text-sm text-gray-500 mt-2 text-center">
						Search data brokers for your information
					</p>
				</div>
			{:else}
				<!-- No Profile State -->
				<div class="text-center py-8">
					<p class="text-gray-600 mb-4">No profile found</p>
					<button
						onclick={() => goto('/profile/setup')}
						class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors"
						style="background-color: #0284c7; color: white;"
					>
						Create Profile
					</button>
				</div>
			{/if}

			<!-- Lock Button -->
			<div class="mt-6 pt-6 border-t border-gray-200">
				<button
					onclick={() => vaultStore.currentVaultId && vaultStore.lock(vaultStore.currentVaultId)}
					class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 transition-colors"
					style="background-color: #0284c7; color: white; padding: 0.75rem 1.5rem; border-radius: 0.5rem; font-weight: 500;"
				>
					Lock Vault
				</button>
			</div>
		</div>
	</div>
{:else}
	<UnlockScreen />
{/if}
