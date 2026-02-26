<script lang="ts">
	import { vaultStore } from '$lib/stores';
	import { profileStore } from '$lib/stores/profile.svelte';
	import { mapBlurAPI, type MapBlurRequest } from '$lib/api/map_blur';

	// Map blur state
	let mapBlurRequests = $state<MapBlurRequest[]>([]);
	let loadingMapBlur = $state(false);
	let mapBlurError = $state<string | null>(null);
	let generatingRequests = $state(false);

	// Get current profile from store
	const currentProfile = $derived(profileStore.currentProfile);

	// Load profile when vault changes
	$effect(() => {
		if (vaultStore.currentVaultId && profileStore.profiles.length > 0) {
			// Load full profile data (ProfileSummary doesn't have address fields)
			profileStore.loadProfile(vaultStore.currentVaultId, profileStore.profiles[0].id);
		}
	});

	// Load profiles list when vault changes
	$effect(() => {
		if (vaultStore.currentVaultId) {
			profileStore.loadProfiles(vaultStore.currentVaultId);
		}
	});

	// Load map blur requests when profile is available
	$effect(() => {
		if (vaultStore.currentVaultId && currentProfile?.id) {
			loadMapBlurRequests();
		}
	});

	// Check if current profile has a complete address
	const hasCompleteAddress = $derived(() => {
		if (!currentProfile) return false;
		return !!(
			currentProfile.address_line1 &&
			currentProfile.city &&
			currentProfile.state &&
			currentProfile.zip_code
		);
	});

	async function loadMapBlurRequests() {
		if (!vaultStore.currentVaultId || !currentProfile?.id) return;

		loadingMapBlur = true;
		mapBlurError = null;

		try {
			mapBlurRequests = await mapBlurAPI.getRequests(vaultStore.currentVaultId, currentProfile.id);
		} catch (err) {
			mapBlurError = err instanceof Error ? err.message : String(err);
			console.error('Failed to load map blur requests:', err);
		} finally {
			loadingMapBlur = false;
		}
	}

	async function handleGenerateMapBlurRequests() {
		if (!vaultStore.currentVaultId || !currentProfile?.id) return;

		generatingRequests = true;
		mapBlurError = null;

		try {
			mapBlurRequests = await mapBlurAPI.generateRequests(
				vaultStore.currentVaultId,
				currentProfile.id
			);
		} catch (err) {
			mapBlurError = err instanceof Error ? err.message : String(err);
			console.error('Failed to generate map blur requests:', err);
		} finally {
			generatingRequests = false;
		}
	}

	async function handleOpenMapBlurUrl(request: MapBlurRequest) {
		if (!vaultStore.currentVaultId) return;

		// Open URL in new window/tab
		window.open(request.requestUrl, '_blank');

		// Mark as submitted
		if (request.status === 'URLGenerated') {
			try {
				await mapBlurAPI.markSubmitted(vaultStore.currentVaultId, request.id, request.service);
				// Reload to reflect new status
				await loadMapBlurRequests();
			} catch (err) {
				console.error('Failed to mark submitted:', err);
			}
		}
	}

	function getServiceName(service: string): string {
		switch (service) {
			case 'GoogleMaps':
				return 'Google Maps';
			case 'AppleMaps':
				return 'Apple Maps';
			case 'BingMaps':
				return 'Bing Maps';
			default:
				return service;
		}
	}

	function getServiceInstructions(service: string): string {
		switch (service) {
			case 'GoogleMaps':
				return 'After clicking, look for "Report a Problem" in the bottom right and select "My home".';
			case 'AppleMaps':
				return 'After clicking, your email client will open. Review the pre-filled message and send it to Apple.';
			case 'BingMaps':
				return 'After clicking, look for "Report a privacy concern" in the lower left and select "House".';
			default:
				return 'Follow the instructions on the service website.';
		}
	}
</script>

<div class="mx-auto max-w-3xl px-4 py-8">
	<h1 class="mb-2 text-2xl font-bold text-gray-900">Map Address Blurring</h1>
	<p class="mb-6 text-gray-600">
		Request that your home address be blurred on Google Maps, Apple Maps, and Bing Maps.
	</p>

	{#if mapBlurError}
		<div class="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-900">
			{mapBlurError}
		</div>
	{/if}

	{#if !currentProfile}
		<div class="rounded-lg border border-blue-200 bg-blue-50 p-4">
			<p class="text-sm text-blue-700">
				Create a profile to request map blurring. <a href="/people" class="underline"
					>Go to People page</a
				>
			</p>
		</div>
	{:else if !hasCompleteAddress()}
		<div class="rounded-lg border border-blue-200 bg-blue-50 p-4">
			<p class="text-sm text-blue-700">
				Your profile needs a complete address (street, city, state, ZIP) to request map blurring.
				<a href="/people" class="underline">Edit your profile</a>
			</p>
		</div>
	{:else if loadingMapBlur}
		<div class="rounded-lg border border-gray-200 bg-white p-8 text-center">
			<div
				class="mx-auto h-8 w-8 animate-spin rounded-full border-4 border-gray-200 border-t-primary-600"
			></div>
			<p class="mt-2 text-sm text-gray-500">Loading map blur requests...</p>
		</div>
	{:else if mapBlurRequests.length === 0}
		<div class="rounded-lg border border-gray-200 bg-gray-50 p-8 text-center">
			<p class="mb-4 text-gray-600">No map blur requests generated yet.</p>
			<button
				onclick={handleGenerateMapBlurRequests}
				disabled={generatingRequests}
				class="rounded-lg bg-primary-600 px-6 py-3 text-white hover:bg-primary-700 disabled:opacity-50"
			>
				{generatingRequests ? 'Generating...' : 'Generate Blur Requests'}
			</button>
		</div>
	{:else}
		<!-- Service Cards -->
		<div class="space-y-4">
			{#each mapBlurRequests as request}
				<div class="rounded-lg border border-gray-200 bg-white p-6">
					<div class="mb-4 flex items-center justify-between">
						<h3 class="text-lg font-semibold text-gray-900">
							{getServiceName(request.service)}
						</h3>
						<span
							class="rounded px-3 py-1 text-sm font-medium {request.status === 'URLGenerated'
								? 'bg-orange-100 text-orange-700'
								: 'bg-green-100 text-green-700'}"
						>
							{request.status === 'URLGenerated' ? 'Ready to Submit' : 'Submitted'}
						</span>
					</div>

					<p class="mb-4 text-sm text-gray-600">
						{getServiceInstructions(request.service)}
					</p>

					<button
						onclick={() => handleOpenMapBlurUrl(request)}
						class="w-full rounded-lg px-4 py-2 text-sm font-medium transition-colors {request.status ===
						'URLGenerated'
							? 'bg-orange-100 text-orange-700 hover:bg-orange-200'
							: 'bg-green-100 text-green-700'}"
					>
						{request.status === 'URLGenerated'
							? '🔍 Request Blur on ' + getServiceName(request.service)
							: '✓ Submitted to ' + getServiceName(request.service)}
					</button>
				</div>
			{/each}
		</div>

		<div class="mt-6 rounded-lg border border-blue-200 bg-blue-50 p-4">
			<h4 class="mb-2 font-medium text-blue-900">Important Notes</h4>
			<ul class="space-y-1 text-sm text-blue-700">
				<li>• Each service has its own manual submission process</li>
				<li>• Blurring is not instant - it may take several weeks to process</li>
				<li>• You may need to provide additional verification in some cases</li>
				<li>• Satellite imagery blurring availability varies by service</li>
			</ul>
		</div>
	{/if}
</div>
