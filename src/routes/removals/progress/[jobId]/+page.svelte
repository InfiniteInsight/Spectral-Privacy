<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { vaultStore, removalStore } from '$lib/stores';
	import OverviewTab from '$lib/components/removals/OverviewTab.svelte';
	import CaptchaQueueTab from '$lib/components/removals/CaptchaQueueTab.svelte';
	import FailedQueueTab from '$lib/components/removals/FailedQueueTab.svelte';

	const scanJobId = $derived($page.params.jobId);

	let activeTab = $state<'overview' | 'captcha' | 'failed'>('overview');

	onMount(async () => {
		// Validate scan job ID
		if (!scanJobId) {
			goto('/');
			return;
		}

		// Validate vault is unlocked
		if (!vaultStore.currentVaultId) {
			goto('/');
			return;
		}

		// Load removal attempts
		await removalStore.loadRemovalAttempts(vaultStore.currentVaultId, scanJobId);

		// Set up event listeners
		await removalStore.setupEventListeners();
	});

	onDestroy(async () => {
		// Clean up event listeners
		await removalStore.cleanupEventListeners();
	});

	async function handleRetry(attemptId: string) {
		if (!vaultStore.currentVaultId) return;

		try {
			await removalStore.retryRemoval(vaultStore.currentVaultId, attemptId);
		} catch (err) {
			console.error('Retry failed:', err);
		}
	}

	async function handleRetryAll() {
		if (!vaultStore.currentVaultId) return;

		for (const attempt of removalStore.failedQueue) {
			try {
				await removalStore.retryRemoval(vaultStore.currentVaultId, attempt.id);
			} catch (err) {
				console.error('Retry failed for', attempt.id, err);
			}
		}
	}
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 p-4">
	<div class="max-w-6xl mx-auto">
		<div class="bg-white rounded-lg shadow-xl overflow-hidden">
			<!-- Header -->
			<div class="px-8 py-6 border-b border-gray-200">
				<h1 class="text-3xl font-bold text-gray-900">Removal Progress</h1>
				<p class="text-gray-600 mt-2">
					Monitoring {removalStore.removalAttempts.length} removal request{removalStore
						.removalAttempts.length !== 1
						? 's'
						: ''}
				</p>
			</div>

			<!-- Tab Navigation -->
			<div class="border-b border-gray-200 px-8">
				<nav class="flex gap-8">
					<button
						onclick={() => (activeTab = 'overview')}
						class="py-4 border-b-2 font-medium text-sm transition-colors {activeTab === 'overview'
							? 'border-blue-600 text-blue-600'
							: 'border-transparent text-gray-600 hover:text-gray-900 hover:border-gray-300'}"
					>
						Overview
					</button>
					<button
						onclick={() => (activeTab = 'captcha')}
						class="py-4 border-b-2 font-medium text-sm transition-colors flex items-center gap-2 {activeTab ===
						'captcha'
							? 'border-blue-600 text-blue-600'
							: 'border-transparent text-gray-600 hover:text-gray-900 hover:border-gray-300'}"
					>
						CAPTCHA Queue
						{#if removalStore.captchaQueue.length > 0}
							<span
								class="px-2 py-0.5 bg-yellow-100 text-yellow-800 rounded-full text-xs font-semibold"
							>
								{removalStore.captchaQueue.length}
							</span>
						{/if}
					</button>
					<button
						onclick={() => (activeTab = 'failed')}
						class="py-4 border-b-2 font-medium text-sm transition-colors flex items-center gap-2 {activeTab ===
						'failed'
							? 'border-blue-600 text-blue-600'
							: 'border-transparent text-gray-600 hover:text-gray-900 hover:border-gray-300'}"
					>
						Failed Queue
						{#if removalStore.failedQueue.length > 0}
							<span class="px-2 py-0.5 bg-red-100 text-red-800 rounded-full text-xs font-semibold">
								{removalStore.failedQueue.length}
							</span>
						{/if}
					</button>
				</nav>
			</div>

			<!-- Tab Content -->
			<div class="p-8">
				{#if removalStore.loading}
					<div class="text-center py-12">
						<div
							class="inline-block w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"
						></div>
						<p class="text-gray-600 mt-4">Loading removal attempts...</p>
					</div>
				{:else if removalStore.error}
					<div class="bg-red-50 border border-red-200 rounded-lg p-6">
						<p class="text-red-900 font-medium">{removalStore.error}</p>
						<button
							onclick={() => {
								if (vaultStore.currentVaultId && scanJobId) {
									removalStore.loadRemovalAttempts(vaultStore.currentVaultId, scanJobId);
								}
							}}
							class="mt-4 px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors"
						>
							Retry
						</button>
					</div>
				{:else if activeTab === 'overview'}
					<OverviewTab
						removalAttempts={removalStore.removalAttempts}
						allComplete={removalStore.allComplete}
					/>
				{:else if activeTab === 'captcha'}
					<CaptchaQueueTab captchaQueue={removalStore.captchaQueue} />
				{:else if activeTab === 'failed'}
					<FailedQueueTab
						failedQueue={removalStore.failedQueue}
						onRetry={handleRetry}
						onRetryAll={handleRetryAll}
					/>
				{/if}
			</div>

			<!-- Footer -->
			<div class="px-8 py-6 border-t border-gray-200 bg-gray-50">
				<button
					onclick={() => goto('/')}
					class="px-6 py-3 bg-gray-600 text-white rounded-lg hover:bg-gray-700 transition-colors font-medium"
				>
					Return to Dashboard
				</button>
			</div>
		</div>
	</div>
</div>
