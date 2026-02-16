<script lang="ts">
	import type { RemovalAttempt } from '$lib/api/removal';

	interface Props {
		failedQueue: RemovalAttempt[];
		// eslint-disable-next-line no-unused-vars
		onRetry: (attemptId: string) => void;
		onRetryAll: () => void;
	}

	let { failedQueue, onRetry, onRetryAll }: Props = $props();

	let expandedErrors = $state<Set<string>>(new Set());
	let retrying = $state<Set<string>>(new Set());

	function toggleError(attemptId: string) {
		const newSet = new Set(expandedErrors);
		if (newSet.has(attemptId)) {
			newSet.delete(attemptId);
		} else {
			newSet.add(attemptId);
		}
		expandedErrors = newSet;
	}

	async function handleRetry(attemptId: string) {
		retrying.add(attemptId);
		try {
			await onRetry(attemptId);
		} finally {
			const newSet = new Set(retrying);
			newSet.delete(attemptId);
			retrying = newSet;
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
			<p class="text-sm text-gray-600">All removals succeeded or need manual attention (CAPTCHA)</p>
		</div>
	{:else}
		<!-- Failed Queue List -->
		<div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
			<div class="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center justify-between">
				<div>
					<h2 class="text-lg font-semibold text-gray-900">Failed Queue ({failedQueue.length})</h2>
					<p class="text-sm text-gray-600 mt-1">These removals failed after all retry attempts</p>
				</div>
				{#if failedQueue.length > 1}
					<button
						onclick={onRetryAll}
						class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm font-medium"
					>
						Retry All
					</button>
				{/if}
			</div>

			<div class="divide-y divide-gray-200">
				{#each failedQueue as attempt}
					{@const isExpanded = expandedErrors.has(attempt.id)}
					{@const isRetrying = retrying.has(attempt.id)}
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
									<button
										onclick={() => toggleError(attempt.id)}
										class="text-blue-600 hover:text-blue-800 font-medium"
									>
										{isExpanded ? 'Hide' : 'Show'} Error Details
									</button>
								</div>

								{#if isExpanded && attempt.error_message}
									<div class="mt-2 p-3 bg-red-50 border border-red-200 rounded-lg">
										<p class="text-sm text-red-900 font-mono break-all">
											{attempt.error_message}
										</p>
									</div>
								{/if}

								<div class="text-xs text-gray-500 mt-2">
									Failed {formatTime(attempt.created_at)}
								</div>
							</div>

							<button
								onclick={() => handleRetry(attempt.id)}
								disabled={isRetrying}
								class="ml-4 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm font-medium flex items-center gap-2"
							>
								{#if isRetrying}
									<span
										class="inline-block w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"
									></span>
									Retrying...
								{:else}
									Retry
								{/if}
							</button>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
