<script lang="ts">
	import { vaultStore } from '$lib/stores/vault.svelte';
	import { profileStore } from '$lib/stores/profile.svelte';
	import { mapBlurAPI, type MapBlurRequest } from '$lib/api/map_blur';
	import { goto } from '$app/navigation';

	let mapBlurRequests = $state<MapBlurRequest[]>([]);
	let loadingMapBlur = $state(false);
	let mapBlurError = $state<string | null>(null);
	let generatingRequests = $state(false);

	// Load profiles when vault is unlocked
	$effect(() => {
		if (vaultStore.isCurrentVaultUnlocked && vaultStore.currentVaultId) {
			profileStore.loadProfiles(vaultStore.currentVaultId);
		}
	});

	// Load map blur requests when component mounts
	$effect(() => {
		async function loadData() {
			if (!vaultStore.currentVaultId || !profileStore.currentProfile?.id) {
				loadingMapBlur = false;
				return;
			}

			loadingMapBlur = true;
			mapBlurError = null;

			try {
				mapBlurRequests = await mapBlurAPI.getRequests(
					vaultStore.currentVaultId,
					profileStore.currentProfile.id
				);
			} catch (err) {
				mapBlurError = err instanceof Error ? err.message : String(err);
				console.error('Failed to load map blur requests:', err);
			} finally {
				loadingMapBlur = false;
			}
		}

		loadData();
	});

	async function handleGenerateMapBlurRequests() {
		if (!vaultStore.currentVaultId || !profileStore.currentProfile?.id) return;

		generatingRequests = true;
		mapBlurError = null;

		try {
			mapBlurRequests = await mapBlurAPI.generateRequests(
				vaultStore.currentVaultId,
				profileStore.currentProfile.id
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
				if (profileStore.currentProfile?.id) {
					mapBlurRequests = await mapBlurAPI.getRequests(
						vaultStore.currentVaultId,
						profileStore.currentProfile.id
					);
				}
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

<div class="min-h-screen bg-gradient-to-br from-purple-50 to-purple-100 p-4">
	<div class="mx-auto max-w-7xl">
		<div class="rounded-lg bg-white p-8 shadow-xl">
			<!-- Header -->
			<div class="mb-8">
				<div class="mb-4 flex items-center justify-between">
					<div>
						<h1 class="mb-2 text-3xl font-bold text-gray-900">Map Address Blurring</h1>
						<p class="text-gray-600">
							Request to blur your home address on Google Maps, Apple Maps, and Bing Maps
						</p>
					</div>
					<button
						onclick={() => goto('/')}
						class="cursor-pointer px-4 py-2 text-gray-600 transition-colors hover:text-gray-900"
					>
						← Back
					</button>
				</div>
			</div>

			{#if mapBlurError}
				<div class="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-900">
					{mapBlurError}
				</div>
			{/if}

			{#if !vaultStore.currentVaultId}
				<div class="rounded-lg border border-blue-200 bg-blue-50 p-8 text-center">
					<h3 class="mb-2 text-lg font-semibold text-blue-900">No Vault Unlocked</h3>
					<p class="text-blue-700">Please unlock a vault to continue.</p>
				</div>
			{:else if !profileStore.currentProfile}
				<div class="rounded-lg border border-blue-200 bg-blue-50 p-8 text-center">
					<h3 class="mb-2 text-lg font-semibold text-blue-900">No Profile Found</h3>
					<p class="mb-4 text-blue-700">
						Create a profile with your current address to request map blurring.
					</p>
					<button
						onclick={() => goto('/profile')}
						class="cursor-pointer rounded-lg bg-purple-600 px-6 py-3 text-white hover:bg-purple-700"
					>
						Create Profile
					</button>
				</div>
			{:else if loadingMapBlur}
				<div class="rounded-lg border border-gray-200 bg-white p-8 text-center">
					<div
						class="mx-auto h-8 w-8 animate-spin rounded-full border-4 border-gray-200 border-t-purple-600"
					></div>
					<p class="mt-2 text-sm text-gray-500">Loading map blur requests...</p>
				</div>
			{:else if mapBlurRequests.length === 0}
				<div class="rounded-lg border border-gray-200 bg-gray-50 p-8 text-center">
					<h3 class="mb-2 text-lg font-semibold text-gray-900">Request Address Blurring on Maps</h3>
					<p class="mb-6 text-gray-600">
						Protect your privacy by requesting that map services blur your home address in Street
						View, satellite imagery, and other public map features.
					</p>

					<div class="mb-6 grid grid-cols-1 gap-4 md:grid-cols-3">
						<div class="rounded-lg border border-blue-200 bg-blue-50 p-4">
							<h4 class="mb-2 font-semibold text-blue-900">🗺️ Google Maps</h4>
							<p class="text-sm text-blue-700">Street View & Satellite</p>
						</div>
						<div class="rounded-lg border border-green-200 bg-green-50 p-4">
							<h4 class="mb-2 font-semibold text-green-900">🍎 Apple Maps</h4>
							<p class="text-sm text-green-700">Look Around & Imagery</p>
						</div>
						<div class="rounded-lg border border-orange-200 bg-orange-50 p-4">
							<h4 class="mb-2 font-semibold text-orange-900">🔍 Bing Maps</h4>
							<p class="text-sm text-orange-700">Streetside & Aerial</p>
						</div>
					</div>

					<button
						onclick={handleGenerateMapBlurRequests}
						disabled={generatingRequests}
						class="cursor-pointer rounded-lg bg-purple-600 px-6 py-3 text-white hover:bg-purple-700 disabled:cursor-not-allowed disabled:opacity-50"
					>
						{generatingRequests ? 'Generating Requests...' : 'Generate Blur Requests'}
					</button>
				</div>
			{:else}
				<!-- Service Cards -->
				<div class="mb-8 space-y-4">
					{#each mapBlurRequests as request}
						<div class="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
							<div class="mb-4 flex items-center justify-between">
								<div>
									<h3 class="text-lg font-semibold text-gray-900">
										{getServiceName(request.service)}
									</h3>
									<p class="text-sm text-gray-500">{request.streetAddress}</p>
								</div>
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
								class="w-full cursor-pointer rounded-lg px-4 py-3 text-sm font-medium transition-colors {request.status ===
								'URLGenerated'
									? 'bg-orange-600 text-white hover:bg-orange-700'
									: 'bg-green-100 text-green-700 hover:bg-green-200'}"
							>
								{request.status === 'URLGenerated'
									? '🔍 Open ' + getServiceName(request.service) + ' to Request Blur'
									: '✓ Submitted to ' + getServiceName(request.service)}
							</button>
						</div>
					{/each}
				</div>

				<!-- Progress Summary -->
				<div class="mb-6 rounded-lg border border-purple-200 bg-purple-50 p-4">
					<h4 class="mb-2 font-medium text-purple-900">Progress</h4>
					<p class="text-sm text-purple-700">
						{mapBlurRequests.filter((r) => r.status === 'Submitted').length} of{' '}
						{mapBlurRequests.length} requests submitted
					</p>
				</div>

				<!-- Important Notes -->
				<div class="rounded-lg border border-blue-200 bg-blue-50 p-4">
					<h4 class="mb-2 font-medium text-blue-900">Important Information</h4>
					<ul class="space-y-1 text-sm text-blue-700">
						<li>• Each service has its own manual submission process</li>
						<li>• Blurring is not instant - it may take several weeks to process</li>
						<li>• You may need to provide additional verification in some cases</li>
						<li>• Satellite imagery blurring availability varies by service</li>
						<li>• Some services blur the entire property, others just the building</li>
					</ul>
				</div>
			{/if}
		</div>
	</div>
</div>
