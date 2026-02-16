<script lang="ts">
	import type { RemovalAttempt } from '$lib/api/removal';
	import { removalStore } from '$lib/stores/removal.svelte';
	import { vaultStore } from '$lib/stores/vault.svelte';

	interface Props {
		failedQueue: RemovalAttempt[];
	}

	let { failedQueue }: Props = $props();

	let retryingIds = $state<Set<string>>(new Set());

	async function handleRetry(attemptId: string) {
		const vaultId = vaultStore.currentVaultId;
		if (!vaultId) {
			console.error('No vault selected');
			return;
		}

		retryingIds.add(attemptId);

		try {
			await removalStore.retryRemoval(vaultId, attemptId);
		} catch (err) {
			console.error('Retry failed:', err);
			alert('Failed to retry removal');
		} finally {
			retryingIds.delete(attemptId);
		}
	}

	async function handleRetryAll() {
		const vaultId = vaultStore.currentVaultId;
		if (!vaultId) {
			console.error('No vault selected');
			return;
		}

		for (const attempt of failedQueue) {
			retryingIds.add(attempt.id);
		}

		try {
			const promises = failedQueue.map((attempt) => removalStore.retryRemoval(vaultId, attempt.id));
			await Promise.all(promises);
		} catch (err) {
			console.error('Batch retry failed:', err);
			alert('Some retries failed');
		} finally {
			retryingIds.clear();
		}
	}

	function formatTime(isoString: string) {
		const date = new Date(isoString);
		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffMins = Math.floor(diffMs / 60000);

		if (diffMins < 1) return 'Just now';
		if (diffMins < 60) return `${diffMins}m ago`;
		const diffHours = Math.floor(diffMins / 60);
		if (diffHours < 24) return `${diffHours}h ago`;
		const diffDays = Math.floor(diffHours / 24);
		return `${diffDays}d ago`;
	}
</script>

<div class="space-y-4">
	{#if failedQueue.length === 0}
		<!-- Empty State -->
		<div class="bg-white rounded-lg border border-gray-200 p-12 text-center">
			<div class="inline-flex items-center justify-center w-16 h-16 bg-green-100 rounded-full mb-4">
				<span class="text-3xl text-green-600">✓</span>
			</div>
			<h3 class="text-lg font-semibold text-gray-900 mb-2">No failed attempts</h3>
			<p class="text-sm text-gray-600">All removals succeeded or need manual attention</p>
		</div>
	{:else}
		<!-- Failed Queue List -->
		<div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
			<div class="px-6 py-4 border-b border-gray-200 bg-gray-50">
				<div class="flex items-center justify-between">
					<div>
						<h2 class="text-lg font-semibold text-gray-900">
							Failed Queue ({failedQueue.length})
						</h2>
						<p class="text-sm text-gray-600 mt-1">These removals failed and can be retried</p>
					</div>

					<button
						onclick={handleRetryAll}
						disabled={retryingIds.size > 0}
						class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm font-medium"
					>
						Retry All
					</button>
				</div>
			</div>

			<div class="divide-y divide-gray-200">
				{#each failedQueue as attempt}
					{@const isRetrying = retryingIds.has(attempt.id)}
					<div class="p-6 hover:bg-gray-50">
						<div class="flex items-start justify-between">
							<div class="flex-1">
								<div class="flex items-center gap-2 mb-2">
									<span class="text-sm font-semibold text-gray-900">{attempt.broker_id}</span>
									<span class="px-2 py-1 rounded-full text-xs font-medium bg-red-100 text-red-800">
										Failed
									</span>
								</div>

								<div class="text-sm text-gray-600 mb-2">
									<span class="font-medium">Error:</span>
									<span class="ml-2">{attempt.error_message || 'Unknown error'}</span>
								</div>

								<div class="text-xs text-gray-500">
									Failed {formatTime(attempt.created_at)}
								</div>
							</div>

							<button
								onclick={() => handleRetry(attempt.id)}
								disabled={isRetrying}
								class="ml-4 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm font-medium"
							>
								{isRetrying ? 'Retrying...' : 'Retry'}
							</button>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
