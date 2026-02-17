<script lang="ts">
	import { scanStore, profileStore, vaultStore } from '$lib/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	let selectedProfileId = $state('');
	let error = $state('');

	onMount(async () => {
		// Ensure profiles are loaded
		if (profileStore.profiles.length === 0) {
			await profileStore.loadProfiles(vaultStore.currentVaultId!);

			// Handle loading errors
			if (profileStore.error) {
				error = profileStore.error;
			}
		}

		// Pre-select first profile
		if (profileStore.profiles.length > 0) {
			selectedProfileId = profileStore.profiles[0].id;
		}
	});

	async function handleStartScan() {
		if (!selectedProfileId) {
			error = 'Please select a profile';
			return;
		}

		if (!vaultStore.currentVaultId) {
			error = 'No vault is unlocked';
			return;
		}

		// Validate profile still exists
		const profileExists = profiles.some((p) => p.id === selectedProfileId);
		if (!profileExists) {
			error = 'Selected profile no longer exists. Please select another profile.';
			selectedProfileId = profiles.length > 0 ? profiles[0].id : '';
			return;
		}

		error = ''; // Clear previous errors
		const scanId = await scanStore.startScan(vaultStore.currentVaultId, selectedProfileId);

		if (scanId) {
			goto(`/scan/progress/${scanId}`);
		} else {
			// Format browser-specific errors with helpful instructions
			const scanError = scanStore.error || 'Failed to start scan';
			if (scanError.includes('Chrome') || scanError.includes('chrome executable')) {
				error = scanError; // Already formatted with install instructions
			} else {
				error = scanError;
			}
		}
	}

	const profiles = $derived(profileStore.profiles);
	const hasProfiles = $derived(profiles.length > 0);
</script>

<div
	class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 flex items-center justify-center p-4"
>
	<div class="bg-white rounded-lg shadow-xl p-8 max-w-2xl w-full">
		<h1 class="text-3xl font-bold text-gray-900 mb-2">Scan for Your Data</h1>
		<p class="text-gray-600 mb-8">
			Search data brokers to find where your personal information appears online
		</p>

		{#if profileStore.loading}
			<div class="flex items-center justify-center py-8">
				<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
			</div>
		{:else if !hasProfiles}
			<div class="text-center py-8">
				<p class="text-gray-600 mb-4">
					No profile found. Create a profile first to start scanning.
				</p>
				<button
					onclick={() => goto('/profile/setup')}
					class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors"
					style="background-color: #0284c7; color: white;"
				>
					Create Profile
				</button>
			</div>
		{:else}
			<!-- Profile Selection -->
			{#if profiles.length > 1}
				<div class="mb-6">
					<label for="profile" class="block text-sm font-medium text-gray-700 mb-2">
						Select Profile
					</label>
					<select
						id="profile"
						bind:value={selectedProfileId}
						class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
					>
						{#each profiles as profile}
							<option value={profile.id}>{profile.full_name}</option>
						{/each}
					</select>
				</div>
			{:else}
				<div class="mb-6 p-4 bg-blue-50 border border-blue-200 rounded-lg">
					<p class="text-sm text-gray-700">
						<strong>Profile:</strong>
						{profiles[0].full_name}
					</p>
				</div>
			{/if}

			<!-- Info Box -->
			<div class="mb-6 p-4 bg-gray-50 border border-gray-200 rounded-lg">
				<h3 class="text-sm font-semibold text-gray-900 mb-2">What happens next:</h3>
				<ul class="text-sm text-gray-600 space-y-1">
					<li>• We'll search multiple data broker sites for your information</li>
					<li>• This typically takes 2-5 minutes</li>
					<li>• You'll review results before any removal requests are sent</li>
				</ul>
			</div>

			<!-- Error Display -->
			{#if error}
				<div class="mb-4 p-4 bg-red-50 border border-red-200 rounded-lg">
					<pre class="text-sm text-red-700 whitespace-pre-wrap font-sans">{error}</pre>
				</div>
			{/if}

			<!-- Start Button -->
			<button
				onclick={handleStartScan}
				disabled={scanStore.loading || !selectedProfileId}
				class="w-full px-6 py-4 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-lg"
				style="background-color: #0284c7; color: white;"
			>
				{scanStore.loading ? 'Starting Scan...' : 'Start Scan'}
			</button>

			<!-- Back to Dashboard -->
			<div class="mt-4 text-center">
				<button
					onclick={() => goto('/')}
					class="text-sm text-gray-600 hover:text-gray-900 transition-colors"
				>
					← Back to Dashboard
				</button>
			</div>
		{/if}
	</div>
</div>
