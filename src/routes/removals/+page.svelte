<script lang="ts">
	import { vaultStore } from '$lib/stores/vault.svelte';
	import { getJobHistory, type RemovalJobSummary } from '$lib/api/removal';

	let jobs = $state<RemovalJobSummary[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let expandedJob = $state<string | null>(null);

	$effect(() => {
		const vid = vaultStore.currentVaultId;
		if (!vid) return;
		loading = true;
		error = null;
		getJobHistory(vid)
			.then((data) => {
				jobs = data;
			})
			.catch((err) => {
				error = err instanceof Error ? err.message : String(err);
			})
			.finally(() => {
				loading = false;
			});
	});

	function formatDate(iso: string) {
		return new Date(iso).toLocaleDateString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div class="mx-auto max-w-3xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-bold text-gray-900">Removal History</h1>

	{#if loading}
		<div class="flex justify-center py-12">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600 mx-auto"></div>
		</div>
	{:else if error}
		<div class="rounded-md bg-red-50 p-4 text-sm text-red-700">{error}</div>
	{:else if jobs.length === 0}
		<div class="rounded-lg border border-dashed border-gray-300 py-12 text-center">
			<p class="text-gray-500">No removal jobs yet.</p>
			<a href="/" class="mt-2 inline-block text-sm text-primary-600 hover:underline"
				>Start a scan to find your data</a
			>
		</div>
	{:else}
		<div class="space-y-4">
			{#each jobs as job (job.scan_job_id)}
				<div class="rounded-lg border border-gray-200 bg-white shadow-sm">
					<button
						onclick={() => (expandedJob = expandedJob === job.scan_job_id ? null : job.scan_job_id)}
						class="flex w-full items-center justify-between p-4 text-left hover:bg-gray-50"
					>
						<div>
							<p class="font-medium text-gray-900">{formatDate(job.submitted_at)}</p>
							<p class="text-sm text-gray-500">{job.total} broker{job.total !== 1 ? 's' : ''}</p>
						</div>
						<div class="flex items-center gap-3 text-xs">
							{#if job.submitted_count > 0}
								<span class="rounded-full bg-green-100 px-2 py-0.5 text-green-700"
									>{job.submitted_count} submitted</span
								>
							{/if}
							{#if job.completed_count > 0}
								<span class="rounded-full bg-blue-100 px-2 py-0.5 text-blue-700"
									>{job.completed_count} confirmed</span
								>
							{/if}
							{#if job.failed_count > 0}
								<span class="rounded-full bg-red-100 px-2 py-0.5 text-red-700"
									>{job.failed_count} failed</span
								>
							{/if}
							{#if job.pending_count > 0}
								<span class="rounded-full bg-yellow-100 px-2 py-0.5 text-yellow-700"
									>{job.pending_count} pending</span
								>
							{/if}
							<svg
								class="h-4 w-4 text-gray-400 transition-transform {expandedJob === job.scan_job_id
									? 'rotate-180'
									: ''}"
								xmlns="http://www.w3.org/2000/svg"
								fill="none"
								viewBox="0 0 24 24"
								stroke="currentColor"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M19 9l-7 7-7-7"
								/>
							</svg>
						</div>
					</button>
					{#if expandedJob === job.scan_job_id}
						<div class="border-t border-gray-100 px-4 pb-4 pt-2">
							<a
								href="/removals/progress/{job.scan_job_id}"
								class="inline-block rounded-md bg-primary-600 px-4 py-2 text-sm font-medium text-white hover:bg-primary-700"
							>
								View full progress dashboard →
							</a>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
