<script lang="ts">
	import type { RemovalAttempt } from '$lib/api/removal';

	interface Props {
		removalAttempts: RemovalAttempt[];
		allComplete: boolean;
	}

	let { removalAttempts, allComplete }: Props = $props();

	// Counts
	const total = $derived(removalAttempts.length);
	const submitted = $derived(
		removalAttempts.filter((r) => r.status === 'Submitted' || r.status === 'Completed').length
	);
	const inProgress = $derived(removalAttempts.filter((r) => r.status === 'Processing').length);
	const captcha = $derived(
		removalAttempts.filter(
			(r) => r.status === 'Pending' && r.error_message?.startsWith('CAPTCHA_REQUIRED')
		).length
	);
	const failed = $derived(removalAttempts.filter((r) => r.status === 'Failed').length);

	// Progress percentage
	const progressPercent = $derived(
		total > 0 ? Math.round(((submitted + captcha + failed) / total) * 100) : 0
	);

	// Recent activity (last 10 items)
	const recentActivity = $derived(
		[...removalAttempts]
			.sort((a, b) => {
				const timeA = a.submitted_at || a.created_at;
				const timeB = b.submitted_at || b.created_at;
				return new Date(timeB).getTime() - new Date(timeA).getTime();
			})
			.slice(0, 10)
	);

	function getStatusBadge(attempt: RemovalAttempt) {
		if (attempt.status === 'Submitted' || attempt.status === 'Completed') {
			return { text: 'Submitted', color: 'bg-green-100 text-green-800' };
		} else if (attempt.status === 'Processing') {
			return { text: 'Processing', color: 'bg-blue-100 text-blue-800' };
		} else if (attempt.error_message?.startsWith('CAPTCHA_REQUIRED')) {
			return { text: 'CAPTCHA', color: 'bg-yellow-100 text-yellow-800' };
		} else if (attempt.status === 'Failed') {
			return { text: 'Failed', color: 'bg-red-100 text-red-800' };
		} else {
			return { text: 'Pending', color: 'bg-gray-100 text-gray-800' };
		}
	}

	function formatTime(isoString: string) {
		const date = new Date(isoString);
		return date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
	}
</script>

<div class="space-y-6">
	<!-- Batch Statistics -->
	<div class="bg-white rounded-lg border border-gray-200 p-6">
		<h2 class="text-lg font-semibold text-gray-900 mb-4">Batch Statistics</h2>

		{#if allComplete}
			<div class="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg">
				<p class="text-sm font-medium text-green-900">✓ Batch Complete</p>
				<p class="text-xs text-green-700 mt-1">All removals processed</p>
			</div>
		{/if}

		<div class="grid grid-cols-2 md:grid-cols-5 gap-4">
			<div class="text-center">
				<div class="text-3xl font-bold text-gray-900">{total}</div>
				<div class="text-sm text-gray-600 mt-1">Total</div>
			</div>
			<div class="text-center">
				<div class="text-3xl font-bold text-green-600">{submitted}</div>
				<div class="text-sm text-gray-600 mt-1">Submitted</div>
			</div>
			<div class="text-center">
				<div class="text-3xl font-bold text-blue-600">{inProgress}</div>
				<div class="text-sm text-gray-600 mt-1">In Progress</div>
			</div>
			<div class="text-center">
				<div class="text-3xl font-bold text-yellow-600">{captcha}</div>
				<div class="text-sm text-gray-600 mt-1">CAPTCHA</div>
			</div>
			<div class="text-center">
				<div class="text-3xl font-bold text-red-600">{failed}</div>
				<div class="text-sm text-gray-600 mt-1">Failed</div>
			</div>
		</div>
	</div>

	<!-- Progress Bar -->
	<div class="bg-white rounded-lg border border-gray-200 p-6">
		<div class="flex items-center justify-between mb-2">
			<h2 class="text-lg font-semibold text-gray-900">Progress</h2>
			<span class="text-sm font-medium text-gray-700">{progressPercent}%</span>
		</div>
		<div class="w-full bg-gray-200 rounded-full h-4">
			<div
				class="bg-blue-600 h-4 rounded-full transition-all duration-300"
				style="width: {progressPercent}%"
			></div>
		</div>
	</div>

	<!-- Recent Activity -->
	<div class="bg-white rounded-lg border border-gray-200 p-6">
		<h2 class="text-lg font-semibold text-gray-900 mb-4">Recent Activity</h2>

		{#if recentActivity.length === 0}
			<p class="text-sm text-gray-500">No activity yet</p>
		{:else}
			<div class="space-y-3">
				{#each recentActivity as attempt}
					{@const badge = getStatusBadge(attempt)}
					<div
						class="flex items-center justify-between py-2 border-b border-gray-100 last:border-0"
					>
						<div class="flex-1">
							<div class="text-sm font-medium text-gray-900">{attempt.broker_id}</div>
							<div class="text-xs text-gray-500 mt-1">
								{formatTime(attempt.submitted_at || attempt.created_at)}
							</div>
						</div>
						<span class="px-3 py-1 rounded-full text-xs font-medium {badge.color}">
							{badge.text}
						</span>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>
