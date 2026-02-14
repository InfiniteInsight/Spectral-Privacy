<script lang="ts">
	import { scanStore, vaultStore } from '$lib/stores';
	import { goto } from '$app/navigation';
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';

	const scanJobId = $derived($page.params.id);

	onMount(() => {
		if (!scanJobId) {
			goto('/scan/start');
			return;
		}

		if (!vaultStore.currentVaultId) {
			goto('/');
			return;
		}

		// Start polling
		scanStore.startPolling(vaultStore.currentVaultId, scanJobId);
	});

	onDestroy(() => {
		// Stop polling when leaving page
		scanStore.stopPolling();
	});

	// Auto-navigate when complete
	$effect(() => {
		let navigationTimer: ReturnType<typeof setTimeout> | undefined;

		if (scanStore.scanStatus?.status === 'Completed') {
			// Wait 2 seconds then navigate
			navigationTimer = setTimeout(() => {
				goto(`/scan/review/${scanJobId}`);
			}, 2000);
		}

		// Cleanup function runs when effect is destroyed
		return () => {
			if (navigationTimer !== undefined) {
				clearTimeout(navigationTimer);
			}
		};
	});

	const status = $derived(scanStore.scanStatus);
	const isComplete = $derived(status?.status === 'Completed');
	const isFailed = $derived(status?.status === 'Failed');
	const isInProgress = $derived(status?.status === 'InProgress');

	// Calculate progress percentage
	const progressPercent = $derived(
		status && status.total_brokers > 0
			? Math.round((status.completed_brokers / status.total_brokers) * 100)
			: 0
	);
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 p-4">
	<div class="max-w-4xl mx-auto">
		<div class="bg-white rounded-lg shadow-xl p-8">
			<!-- Header -->
			<div class="mb-8">
				<h1 class="text-3xl font-bold text-gray-900 mb-2">
					{#if isComplete}
						Scan Complete!
					{:else if isFailed}
						Scan Failed
					{:else}
						Scanning Data Brokers...
					{/if}
				</h1>
				{#if status}
					<p class="text-gray-600">
						{#if isComplete}
							Found results from {status.completed_brokers} of {status.total_brokers} brokers
						{:else}
							Checking {status.completed_brokers} of {status.total_brokers} brokers
						{/if}
					</p>
				{/if}
			</div>

			<!-- Progress Bar -->
			{#if status}
				<div class="mb-8">
					<div class="flex items-center justify-between mb-2">
						<span class="text-sm font-medium text-gray-700">Progress</span>
						<span class="text-sm font-medium text-gray-700">{progressPercent}%</span>
					</div>
					<div class="w-full bg-gray-200 rounded-full h-3">
						<div
							class="bg-primary-600 h-3 rounded-full transition-all duration-500"
							style="width: {progressPercent}%; background-color: #0284c7;"
						></div>
					</div>
				</div>
			{/if}

			<!-- Store Error Display -->
			{#if scanStore.error && !isFailed}
				<div class="mb-6 p-4 bg-yellow-50 border border-yellow-200 rounded-lg">
					<p class="text-sm text-yellow-700">⚠ {scanStore.error}</p>
				</div>
			{/if}

			<!-- Status Messages -->
			{#if isComplete}
				<div class="mb-6 p-4 bg-green-50 border border-green-200 rounded-lg">
					<p class="text-sm text-green-700">✓ Scan complete! Redirecting to review findings...</p>
				</div>
			{:else if isFailed}
				<div class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg">
					<p class="text-sm text-red-700">
						✗ Scan failed: {status?.error_message || 'Unknown error'}
					</p>
				</div>
				<button
					onclick={() => goto('/scan/start')}
					class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors"
					style="background-color: #0284c7; color: white;"
				>
					Try Again
				</button>
			{/if}

			<!-- Broker Status List (Placeholder - Task 7 will add BrokerStatusList component) -->
			{#if isInProgress}
				<div class="text-center py-8">
					<div
						class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto"
					></div>
					<p class="text-sm text-gray-600 mt-4">Scanning brokers...</p>
				</div>
			{/if}

			<!-- Manual Navigate (for testing) -->
			{#if isComplete}
				<div class="mt-6 text-center">
					<button
						onclick={() => goto(`/scan/review/${scanJobId}`)}
						class="text-sm text-primary-600 hover:text-primary-700 transition-colors"
					>
						View Results Now →
					</button>
				</div>
			{/if}
		</div>
	</div>
</div>
