<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { vaultStore, removalStore } from '$lib/stores';
	import OverviewTab from '$lib/components/removals/OverviewTab.svelte';
	import CaptchaQueueTab from '$lib/components/removals/CaptchaQueueTab.svelte';
	import FailedQueueTab from '$lib/components/removals/FailedQueueTab.svelte';
	import { markAttemptVerified } from '$lib/api/verification';

	const scanJobId = $derived($page.params.jobId);

	let activeTab = $state<'overview' | 'captcha' | 'failed' | 'verification'>('overview');

	$effect(() => {
		// Validate scan job ID
		if (!scanJobId) {
			goto('/');
			return;
		}

		// Validate vault is unlocked
		const currentVaultId = vaultStore.currentVaultId;
		if (!currentVaultId) {
			goto('/');
			return;
		}

		// Load removal attempts and set up listeners
		(async () => {
			await removalStore.loadRemovalAttempts(currentVaultId, scanJobId);
			await removalStore.setupEventListeners();
		})();

		// Cleanup function (replaces onDestroy)
		return () => {
			removalStore.cleanupEventListeners();
		};
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

	async function handleMarkVerified(attemptId: string) {
		if (!vaultStore.currentVaultId) return;

		try {
			await markAttemptVerified(vaultStore.currentVaultId, attemptId);
			// removal:verified event will update store via listener
		} catch (err) {
			console.error('Failed to mark as verified:', err);
			// User will see the attempt remain in the queue if it fails
		}
	}

	function getBrokerName(brokerId: string): string {
		// Simple helper to display broker name
		return brokerId
			.split('.')
			.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
			.join(' ');
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
					{#if removalStore.awaitingVerification.length > 0}
						<button
							onclick={() => (activeTab = 'verification')}
							class="py-4 border-b-2 font-medium text-sm transition-colors flex items-center gap-2 {activeTab ===
							'verification'
								? 'border-blue-600 text-blue-600'
								: 'border-transparent text-gray-600 hover:text-gray-900 hover:border-gray-300'}"
						>
							Pending Verification
							<span
								class="px-2 py-0.5 bg-amber-100 text-amber-800 rounded-full text-xs font-semibold"
							>
								{removalStore.awaitingVerification.length}
							</span>
						</button>
					{/if}
				</nav>
			</div>

			<!-- Tab Content -->
			<div class="p-8">
				{#if removalStore.loading}
					<div class="text-center py-12">
						<div
							class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600 mx-auto"
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
				{:else if activeTab === 'verification'}
					<div class="space-y-3">
						{#each removalStore.awaitingVerification as attempt}
							{@const broker = getBrokerName(attempt.broker_id)}
							<div
								class="border border-amber-200 dark:border-amber-800 rounded-lg p-4 bg-amber-50 dark:bg-amber-900/10"
							>
								<div class="flex items-center justify-between">
									<div>
										<p class="font-medium text-gray-900 dark:text-gray-100">{broker}</p>
										<p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
											Check your inbox for a confirmation email
										</p>
									</div>
									<button
										onclick={() => handleMarkVerified(attempt.id)}
										class="px-4 py-2 bg-green-600 text-white rounded-lg text-sm hover:bg-green-700 transition-colors font-medium"
									>
										Mark as Verified
									</button>
								</div>
							</div>
						{/each}
					</div>
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
