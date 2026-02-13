<script lang="ts">
	import { goto } from '$app/navigation';
	import { scanStore } from '$lib/stores';
	import { profileStore } from '$lib/stores';

	let isStarting = $state(false);
	let errorMessage = $state<string | null>(null);

	async function handleStartScan() {
		const profile = profileStore.currentProfile;
		if (!profile) {
			errorMessage = 'No profile selected. Please create a profile first.';
			return;
		}

		isStarting = true;
		errorMessage = null;

		try {
			const scanJobId = await scanStore.startScan(profile.id);

			if (!scanJobId) {
				errorMessage = scanStore.error || 'Failed to start scan';
				return;
			}

			// Redirect to progress page
			await goto(`/scan/progress/${scanJobId}`);
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : 'Failed to start scan';
		} finally {
			isStarting = false;
		}
	}
</script>

<div class="min-h-screen bg-gray-50 flex items-center justify-center p-4">
	<div class="max-w-md w-full bg-white rounded-lg shadow-md p-8">
		<h1 class="text-2xl font-bold mb-2">Start Data Broker Scan</h1>
		<p class="text-gray-600 mb-6">
			Scan data brokers to find and remove your personal information from the web.
		</p>

		{#if profileStore.currentProfile}
			<div class="mb-6 p-4 bg-blue-50 rounded-md">
				<p class="text-sm text-gray-700">
					<span class="font-medium">Profile:</span>
					{profileStore.currentProfile.first_name}
					{profileStore.currentProfile.last_name}
				</p>
			</div>
		{/if}

		{#if errorMessage}
			<div class="mb-4 p-4 bg-red-50 border border-red-200 rounded-md">
				<p class="text-sm text-red-700">{errorMessage}</p>
			</div>
		{/if}

		<button
			onclick={handleStartScan}
			disabled={isStarting || !profileStore.currentProfile}
			class="w-full py-3 rounded-md font-medium transition-colors"
			style="background-color: {isStarting || !profileStore.currentProfile
				? '#9ca3af'
				: '#0284c7'}; color: white;"
		>
			{#if isStarting}
				<span class="flex items-center justify-center gap-2">
					<svg class="animate-spin h-5 w-5" viewBox="0 0 24 24">
						<circle
							class="opacity-25"
							cx="12"
							cy="12"
							r="10"
							stroke="currentColor"
							stroke-width="4"
							fill="none"
						></circle>
						<path
							class="opacity-75"
							fill="currentColor"
							d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
						></path>
					</svg>
					Starting Scan...
				</span>
			{:else}
				Start Scan
			{/if}
		</button>

		<p class="mt-4 text-sm text-gray-500 text-center">
			This will search all registered data brokers for your information.
		</p>
	</div>
</div>
