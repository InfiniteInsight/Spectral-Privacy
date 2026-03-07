<script lang="ts">
	import { vaultStore } from '$lib/stores/vault.svelte';
	import { profileStore } from '$lib/stores/profile.svelte';
	import { scanAPI, type ScanHistoryEntry } from '$lib/api/scan';
	import Spinner from '$lib/components/Spinner.svelte';

	let history = $state<ScanHistoryEntry[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Load scan history when vault changes
	$effect(() => {
		if (vaultStore.currentVaultId) {
			loadScanHistory();
		} else {
			loading = false;
			history = [];
		}
	});

	async function loadScanHistory() {
		if (!vaultStore.currentVaultId) return;

		try {
			loading = true;
			error = null;
			history = await scanAPI.getUnifiedScanHistory(
				vaultStore.currentVaultId,
				profileStore.currentProfile?.id
			);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
			console.error('Failed to load scan history:', err);
		} finally {
			loading = false;
		}
	}

	function formatDate(isoDate: string): string {
		try {
			return new Date(isoDate).toLocaleDateString('en-US', {
				year: 'numeric',
				month: 'short',
				day: 'numeric',
				hour: '2-digit',
				minute: '2-digit'
			});
		} catch {
			return isoDate;
		}
	}

	function calculateDuration(startedAt: string, completedAt?: string): string {
		try {
			const start = new Date(startedAt);
			const end = completedAt ? new Date(completedAt) : new Date();
			const durationMs = end.getTime() - start.getTime();

			const hours = Math.floor(durationMs / (1000 * 60 * 60));
			const minutes = Math.floor((durationMs % (1000 * 60 * 60)) / (1000 * 60));
			const seconds = Math.floor((durationMs % (1000 * 60)) / 1000);

			if (hours > 0) {
				return `${hours}h ${minutes}m`;
			} else if (minutes > 0) {
				return `${minutes}m ${seconds}s`;
			} else {
				return `${seconds}s`;
			}
		} catch {
			return 'Unknown';
		}
	}

	function statusBadgeClass(status: string): string {
		switch (status) {
			case 'Completed':
				return 'bg-green-100 text-green-800';
			case 'InProgress':
				return 'bg-blue-100 text-blue-800';
			case 'Failed':
				return 'bg-red-100 text-red-800';
			case 'Cancelled':
				return 'bg-gray-100 text-gray-800';
			default:
				return 'bg-gray-100 text-gray-800';
		}
	}
</script>

<div class="mx-auto max-w-6xl px-4 py-8">
	<div class="mb-6">
		<h1 class="mb-2 text-2xl font-bold text-gray-900">Scan History</h1>
		<p class="text-gray-600">
			View previous data broker scans with statistics on findings, reviews, and removal requests.
		</p>
	</div>

	{#if error}
		<div class="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-900">
			{error}
		</div>
	{/if}

	{#if !vaultStore.currentVaultId}
		<div class="rounded-lg border border-blue-200 bg-blue-50 p-4">
			<p class="text-sm text-blue-700">
				Create or unlock a vault to view scan history.
				<a href="/people" class="underline">Go to People page</a>
			</p>
		</div>
	{:else if loading}
		<div class="flex justify-center py-12">
			<Spinner color="indigo" />
		</div>
	{:else if history.length === 0}
		<div class="rounded-lg border border-gray-200 bg-gray-50 p-8 text-center">
			<p class="mb-4 text-gray-600">No scan history yet.</p>
			<a
				href="/scan"
				class="inline-block rounded-lg bg-indigo-600 px-6 py-3 text-white hover:bg-indigo-700"
			>
				Start Your First Scan
			</a>
		</div>
	{:else}
		<!-- Scan History Cards -->
		<div class="space-y-4">
			{#each history as item}
				{#if item.scan_type === 'DataBroker'}
					<!-- Data Broker Scan Card -->
					<div class="rounded-lg border border-gray-200 bg-white p-6">
						<div class="mb-4 flex items-start justify-between">
							<div>
								<div class="mb-1 flex items-center gap-2">
									<h3 class="text-lg font-semibold text-gray-900">Data Broker Scan</h3>
									<span
										class="inline-flex rounded-full px-2 py-1 text-xs font-medium {statusBadgeClass(
											item.status
										)}"
									>
										{item.status}
									</span>
								</div>
								<div class="text-sm text-gray-500">Started {formatDate(item.started_at)}</div>
								{#if item.completed_at}
									<div class="text-sm text-gray-500">
										Duration: {calculateDuration(item.started_at, item.completed_at)}
									</div>
								{/if}
							</div>
						</div>

						{#if item.error_message}
							<div class="mb-4 rounded border border-red-200 bg-red-50 p-3 text-sm text-red-700">
								Error: {item.error_message}
							</div>
						{/if}

						<div class="grid grid-cols-2 gap-4 md:grid-cols-5">
							<div class="rounded-lg bg-gray-50 p-3">
								<div class="text-2xl font-bold text-gray-900">
									{item.completed_brokers}/{item.total_brokers}
								</div>
								<div class="text-xs text-gray-600">Brokers Scanned</div>
							</div>
							<div class="rounded-lg bg-blue-50 p-3">
								<div class="text-2xl font-bold text-blue-900">{item.total_findings}</div>
								<div class="text-xs text-blue-700">Total Findings</div>
							</div>
							<div class="rounded-lg bg-green-50 p-3">
								<div class="text-2xl font-bold text-green-900">{item.confirmed_findings}</div>
								<div class="text-xs text-green-700">Confirmed</div>
							</div>
							<div class="rounded-lg bg-orange-50 p-3">
								<div class="text-2xl font-bold text-orange-900">{item.rejected_findings}</div>
								<div class="text-xs text-orange-700">Rejected</div>
							</div>
							<div class="rounded-lg bg-indigo-50 p-3">
								<div class="text-2xl font-bold text-indigo-900">{item.removal_requests}</div>
								<div class="text-xs text-indigo-700">Removal Requests</div>
							</div>
						</div>

						{#if item.status === 'InProgress'}
							<div class="mt-4">
								<div class="mb-1 flex justify-between text-sm text-gray-600">
									<span>Scan in progress...</span>
									<span>{Math.round((item.completed_brokers / item.total_brokers) * 100)}%</span>
								</div>
								<div class="h-2 w-full rounded-full bg-gray-200">
									<div
										class="h-2 rounded-full bg-indigo-600 transition-all"
										style="width: {(item.completed_brokers / item.total_brokers) * 100}%"
									></div>
								</div>
							</div>
						{/if}
					</div>
				{:else if item.scan_type === 'Cookie'}
					<!-- Cookie Scan Card -->
					<div class="rounded-lg border border-gray-200 bg-white p-6">
						<div class="mb-4 flex items-start justify-between">
							<div>
								<div class="mb-1 flex items-center gap-2">
									<h3 class="text-lg font-semibold text-gray-900">Cookie Scan</h3>
									<span
										class="inline-flex rounded-full px-2 py-1 text-xs font-medium {statusBadgeClass(
											item.status
										)}"
									>
										{item.status}
									</span>
								</div>
								<div class="text-sm text-gray-500">Started {formatDate(item.started_at)}</div>
								{#if item.completed_at}
									<div class="text-sm text-gray-500">
										Duration: {calculateDuration(item.started_at, item.completed_at)}
									</div>
								{/if}
							</div>
						</div>

						{#if item.error_message}
							<div class="mb-4 rounded border border-red-200 bg-red-50 p-3 text-sm text-red-700">
								Error: {item.error_message}
							</div>
						{/if}

						<div class="grid grid-cols-2 gap-4 md:grid-cols-4">
							<div class="rounded-lg bg-gray-50 p-3">
								<div class="text-2xl font-bold text-gray-900">{item.total_cookies_found}</div>
								<div class="text-xs text-gray-600">Total Cookies</div>
							</div>
							<div class="rounded-lg bg-orange-50 p-3">
								<div class="text-2xl font-bold text-orange-900">{item.matched_cookies}</div>
								<div class="text-xs text-orange-700">Tracking Cookies</div>
							</div>
							<div class="rounded-lg bg-blue-50 p-3">
								<div class="text-2xl font-bold text-blue-900">{item.browsers_scanned.length}</div>
								<div class="text-xs text-blue-700">Browsers Scanned</div>
							</div>
							<div class="rounded-lg bg-purple-50 p-3">
								<div class="text-2xl font-bold text-purple-900">{item.brokers_matched.length}</div>
								<div class="text-xs text-purple-700">Brokers Matched</div>
							</div>
						</div>

						{#if item.browsers_scanned.length > 0}
							<div class="mt-3 text-sm text-gray-600">
								Browsers: {item.browsers_scanned.join(', ')}
							</div>
						{/if}
					</div>
				{:else if item.scan_type === 'Discovery'}
					<!-- Local PII Discovery Card -->
					<div class="rounded-lg border border-gray-200 bg-white p-6">
						<div class="mb-4">
							<div class="mb-1 flex items-center gap-2">
								<h3 class="text-lg font-semibold text-gray-900">Local PII Discovery</h3>
								<span
									class="inline-flex rounded-full bg-green-100 px-2 py-1 text-xs font-medium text-green-800"
								>
									Completed
								</span>
							</div>
							<div class="text-sm text-gray-500">Found {formatDate(item.started_at)}</div>
						</div>

						<div class="grid grid-cols-2 gap-4 md:grid-cols-4">
							<div class="rounded-lg bg-gray-50 p-3">
								<div class="text-2xl font-bold text-gray-900">{item.total_findings}</div>
								<div class="text-xs text-gray-600">Total Findings</div>
							</div>
							<div class="rounded-lg bg-red-50 p-3">
								<div class="text-2xl font-bold text-red-900">{item.critical_findings}</div>
								<div class="text-xs text-red-700">Critical</div>
							</div>
							<div class="rounded-lg bg-yellow-50 p-3">
								<div class="text-2xl font-bold text-yellow-900">{item.medium_findings}</div>
								<div class="text-xs text-yellow-700">Medium</div>
							</div>
							<div class="rounded-lg bg-blue-50 p-3">
								<div class="text-2xl font-bold text-blue-900">{item.informational_findings}</div>
								<div class="text-xs text-blue-700">Informational</div>
							</div>
						</div>
					</div>
				{/if}
			{/each}
		</div>

		<!-- Summary Stats -->
		<div class="mt-6 rounded-lg border border-gray-200 bg-gray-50 p-4">
			<h4 class="mb-3 font-medium text-gray-900">Overall Statistics</h4>
			<div class="grid grid-cols-2 gap-4 md:grid-cols-4">
				<div>
					<div class="text-2xl font-bold text-gray-900">{history.length}</div>
					<div class="text-sm text-gray-600">Total Scans</div>
				</div>
				<div>
					<div class="text-2xl font-bold text-gray-900">
						{history.reduce((sum, h) => {
							if (h.scan_type === 'DataBroker') return sum + h.total_findings;
							if (h.scan_type === 'Discovery') return sum + h.total_findings;
							if (h.scan_type === 'Cookie') return sum + h.matched_cookies;
							return sum;
						}, 0)}
					</div>
					<div class="text-sm text-gray-600">Total Findings</div>
				</div>
				<div>
					<div class="text-2xl font-bold text-gray-900">
						{history.filter((h) => h.scan_type === 'DataBroker').length}
					</div>
					<div class="text-sm text-gray-600">Broker Scans</div>
				</div>
				<div>
					<div class="text-2xl font-bold text-gray-900">
						{history.reduce((sum, h) => {
							if (h.scan_type === 'DataBroker') return sum + h.removal_requests;
							return sum;
						}, 0)}
					</div>
					<div class="text-sm text-gray-600">Removal Requests</div>
				</div>
			</div>
		</div>
	{/if}
</div>
