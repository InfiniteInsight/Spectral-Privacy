<script lang="ts">
	import { vaultStore } from '$lib/stores/vault.svelte';
	import {
		getDiscoveryFindings,
		markFindingRemediated,
		startDiscoveryScan,
		type DiscoveryFinding
	} from '$lib/api/discovery';
	import { listen } from '@tauri-apps/api/event';

	let findings = $state<DiscoveryFinding[]>([]);
	let loading = $state(true);
	let scanning = $state(false);
	let error = $state<string | null>(null);
	let successMessage = $state<string | null>(null);
	let currentDirectory = $state<string | null>(null);

	// Filter state
	let piiTypeFilter = $state<Set<string>>(new Set());
	let riskLevelFilter = $state<Set<string>>(new Set());

	// Filtered findings based on active filters
	const filteredFindings = $derived(
		findings.filter((f) => {
			// Never show remediated findings
			if (f.remediated) return false;

			// PII type filter (OR logic within group)
			const piiMatch = piiTypeFilter.size === 0 || (f.pii_type && piiTypeFilter.has(f.pii_type));

			// Risk level filter (OR logic within group)
			const riskMatch = riskLevelFilter.size === 0 || riskLevelFilter.has(f.risk_level);

			// Must match ALL filter groups (AND logic between groups)
			return piiMatch && riskMatch;
		})
	);

	// Computed summary counts from filtered findings
	const criticalCount = $derived(
		filteredFindings.filter((f) => f.risk_level === 'critical').length
	);
	const mediumCount = $derived(filteredFindings.filter((f) => f.risk_level === 'medium').length);
	const informationalCount = $derived(
		filteredFindings.filter((f) => f.risk_level === 'informational').length
	);

	// Group filtered findings by source
	const filesystemFindings = $derived(filteredFindings.filter((f) => f.source === 'filesystem'));
	const browserFindings = $derived(filteredFindings.filter((f) => f.source === 'browser'));
	const emailFindings = $derived(filteredFindings.filter((f) => f.source === 'email'));

	// Toggle PII type filter
	function togglePiiType(type: string) {
		const newFilter = new Set(piiTypeFilter);
		if (newFilter.has(type)) {
			newFilter.delete(type);
		} else {
			newFilter.add(type);
		}
		piiTypeFilter = newFilter;
	}

	// Toggle risk level filter
	function toggleRiskLevel(level: string) {
		const newFilter = new Set(riskLevelFilter);
		if (newFilter.has(level)) {
			newFilter.delete(level);
		} else {
			newFilter.add(level);
		}
		riskLevelFilter = newFilter;
	}

	// Load findings when vault changes
	async function loadFindings() {
		const vid = vaultStore.currentVaultId;
		if (!vid) {
			loading = false;
			return;
		}

		try {
			loading = true;
			error = null;
			findings = await getDiscoveryFindings(vid);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	// Start scan
	async function startScan() {
		const vid = vaultStore.currentVaultId;
		if (!vid) return;

		try {
			scanning = true;
			error = null;
			await startDiscoveryScan(vid);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			scanning = false;
		}
	}

	// Mark finding as remediated
	async function markRemediated(findingId: string) {
		const vid = vaultStore.currentVaultId;
		if (!vid) return;

		try {
			await markFindingRemediated(vid, findingId);
			// Reload findings
			await loadFindings();
			// Show success message
			successMessage = 'Finding marked as remediated';
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	// Set up event listeners and load findings
	$effect(() => {
		loadFindings();

		// Listen for scan progress
		const unlistenProgress = listen('discovery:progress', (event: any) => {
			currentDirectory = event.payload.directory;
		});

		// Listen for scan completion
		const unlistenComplete = listen('discovery:complete', () => {
			scanning = false;
			currentDirectory = null;
			loadFindings();
		});

		// Clean up listeners on unmount
		return () => {
			unlistenProgress.then((fn) => fn());
			unlistenComplete.then((fn) => fn());
		};
	});

	function riskBadgeClass(level: string): string {
		switch (level) {
			case 'critical':
				return 'bg-red-100 text-red-800';
			case 'medium':
				return 'bg-yellow-100 text-yellow-800';
			case 'informational':
				return 'bg-blue-100 text-blue-800';
			default:
				return 'bg-gray-100 text-gray-800';
		}
	}

	function formatDate(isoDate: string): string {
		try {
			return new Date(isoDate).toLocaleDateString('en-US', {
				year: 'numeric',
				month: 'short',
				day: 'numeric'
			});
		} catch {
			return isoDate;
		}
	}

	function chipClass(isSelected: boolean): string {
		if (isSelected) {
			return 'px-3 py-1 rounded-full text-sm font-medium bg-indigo-600 text-white cursor-pointer hover:bg-indigo-700 transition-colors';
		}
		return 'px-3 py-1 rounded-full text-sm font-medium bg-gray-200 text-gray-700 cursor-pointer hover:bg-gray-300 transition-colors';
	}
</script>

<div class="mx-auto max-w-6xl px-4 py-8">
	<div class="mb-6 flex items-center justify-between">
		<h1 class="text-2xl font-bold text-gray-900">Local PII Discovery</h1>
		<button
			onclick={startScan}
			disabled={scanning || loading}
			class="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed"
		>
			{#if scanning}
				<span class="flex items-center gap-2">
					<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
					{#if currentDirectory}
						Scanning {currentDirectory}...
					{:else}
						Scanning...
					{/if}
				</span>
			{:else}
				Run Discovery Scan
			{/if}
		</button>
	</div>

	{#if error}
		<div class="mb-4 rounded-md bg-red-50 p-4 text-sm text-red-700">{error}</div>
	{/if}

	{#if successMessage}
		<div class="mb-4 rounded-md bg-green-50 p-4 text-sm text-green-700">{successMessage}</div>
	{/if}

	{#if !loading && findings.length > 0}
		<!-- Filter Chips -->
		<div class="mb-6 space-y-3">
			<!-- PII Type Filters -->
			<div class="flex items-center gap-2 flex-wrap">
				<span class="text-sm font-medium text-gray-700">PII Type:</span>
				<button
					onclick={() => togglePiiType('email')}
					class={chipClass(piiTypeFilter.has('email'))}
				>
					Email
				</button>
				<button
					onclick={() => togglePiiType('phone')}
					class={chipClass(piiTypeFilter.has('phone'))}
				>
					Phone
				</button>
				<button onclick={() => togglePiiType('ssn')} class={chipClass(piiTypeFilter.has('ssn'))}>
					SSN
				</button>
			</div>

			<!-- Risk Level Filters -->
			<div class="flex items-center gap-2 flex-wrap">
				<span class="text-sm font-medium text-gray-700">Risk Level:</span>
				<button
					onclick={() => toggleRiskLevel('critical')}
					class={chipClass(riskLevelFilter.has('critical'))}
				>
					Critical
				</button>
				<button
					onclick={() => toggleRiskLevel('medium')}
					class={chipClass(riskLevelFilter.has('medium'))}
				>
					Medium
				</button>
				<button
					onclick={() => toggleRiskLevel('informational')}
					class={chipClass(riskLevelFilter.has('informational'))}
				>
					Informational
				</button>
			</div>

			<!-- Findings count -->
			<div class="text-sm text-gray-600">
				Showing {filteredFindings.length} of {findings.filter((f) => !f.remediated).length} findings
			</div>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center py-12">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600"></div>
		</div>
	{:else if findings.length === 0}
		<div class="rounded-md bg-gray-50 p-8 text-center">
			<p class="text-gray-600">
				No findings yet. Click "Run Discovery Scan" to scan your local files for PII.
			</p>
		</div>
	{:else}
		<!-- Summary Cards -->
		<div class="mb-6 grid grid-cols-1 gap-4 md:grid-cols-3">
			<div class="rounded-lg border border-red-200 bg-red-50 p-4">
				<div class="text-2xl font-bold text-red-900">{criticalCount}</div>
				<div class="text-sm text-red-700">Critical Issues</div>
			</div>
			<div class="rounded-lg border border-yellow-200 bg-yellow-50 p-4">
				<div class="text-2xl font-bold text-yellow-900">{mediumCount}</div>
				<div class="text-sm text-yellow-700">Medium Risk</div>
			</div>
			<div class="rounded-lg border border-blue-200 bg-blue-50 p-4">
				<div class="text-2xl font-bold text-blue-900">{informationalCount}</div>
				<div class="text-sm text-blue-700">Informational</div>
			</div>
		</div>

		<!-- Empty state for filtered results -->
		{#if findings.length > 0 && filteredFindings.length === 0}
			<div class="rounded-md bg-gray-50 p-8 text-center">
				<p class="text-gray-600">
					No findings match the selected filters. Try adjusting your selection.
				</p>
			</div>
		{/if}

		<!-- Filesystem Findings -->
		{#if filesystemFindings.length > 0}
			<div class="mb-6">
				<h2 class="mb-3 text-lg font-semibold text-gray-900">Filesystem</h2>
				<div class="space-y-3">
					{#each filesystemFindings as finding}
						<div class="rounded-lg border border-gray-200 bg-white p-4">
							<div class="mb-2 flex items-start justify-between">
								<div class="flex-1">
									<div class="mb-1 flex items-center gap-2">
										<span class="text-sm font-medium text-gray-900">{finding.description}</span>
										<span
											class="inline-flex rounded-full px-2 py-1 text-xs font-medium {riskBadgeClass(
												finding.risk_level
											)}"
										>
											{finding.risk_level}
										</span>
										{#if finding.remediated}
											<span
												class="inline-flex rounded-full bg-green-100 px-2 py-1 text-xs font-medium text-green-800"
											>
												Remediated
											</span>
										{/if}
									</div>
									<div class="text-xs text-gray-500">{finding.source_detail}</div>
									{#if finding.recommended_action}
										<div class="mt-2 text-sm text-gray-600">
											<strong>Recommended action:</strong>
											{finding.recommended_action}
										</div>
									{/if}
									<div class="mt-1 text-xs text-gray-400">
										Found {formatDate(finding.found_at)}
									</div>
								</div>
								{#if !finding.remediated}
									<button
										onclick={() => markRemediated(finding.id)}
										class="ml-4 rounded-md bg-gray-100 px-3 py-1 text-sm text-gray-700 hover:bg-gray-200"
									>
										Mark as Remediated
									</button>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Browser Findings (Stub for Phase 7) -->
		{#if browserFindings.length > 0}
			<div class="mb-6">
				<h2 class="mb-3 text-lg font-semibold text-gray-900">Browser</h2>
				<div class="space-y-3">
					{#each browserFindings as finding}
						<div class="rounded-lg border border-gray-200 bg-white p-4">
							<div class="mb-2 flex items-start justify-between">
								<div class="flex-1">
									<div class="mb-1 flex items-center gap-2">
										<span class="text-sm font-medium text-gray-900">{finding.description}</span>
										<span
											class="inline-flex rounded-full px-2 py-1 text-xs font-medium {riskBadgeClass(
												finding.risk_level
											)}"
										>
											{finding.risk_level}
										</span>
										{#if finding.remediated}
											<span
												class="inline-flex rounded-full bg-green-100 px-2 py-1 text-xs font-medium text-green-800"
											>
												Remediated
											</span>
										{/if}
									</div>
									<div class="text-xs text-gray-500">{finding.source_detail}</div>
									{#if finding.recommended_action}
										<div class="mt-2 text-sm text-gray-600">
											<strong>Recommended action:</strong>
											{finding.recommended_action}
										</div>
									{/if}
									<div class="mt-1 text-xs text-gray-400">
										Found {formatDate(finding.found_at)}
									</div>
								</div>
								{#if !finding.remediated}
									<button
										onclick={() => markRemediated(finding.id)}
										class="ml-4 rounded-md bg-gray-100 px-3 py-1 text-sm text-gray-700 hover:bg-gray-200"
									>
										Mark as Remediated
									</button>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Email Findings (Stub for Phase 7) -->
		{#if emailFindings.length > 0}
			<div class="mb-6">
				<h2 class="mb-3 text-lg font-semibold text-gray-900">Email</h2>
				<div class="space-y-3">
					{#each emailFindings as finding}
						<div class="rounded-lg border border-gray-200 bg-white p-4">
							<div class="mb-2 flex items-start justify-between">
								<div class="flex-1">
									<div class="mb-1 flex items-center gap-2">
										<span class="text-sm font-medium text-gray-900">{finding.description}</span>
										<span
											class="inline-flex rounded-full px-2 py-1 text-xs font-medium {riskBadgeClass(
												finding.risk_level
											)}"
										>
											{finding.risk_level}
										</span>
										{#if finding.remediated}
											<span
												class="inline-flex rounded-full bg-green-100 px-2 py-1 text-xs font-medium text-green-800"
											>
												Remediated
											</span>
										{/if}
									</div>
									<div class="text-xs text-gray-500">{finding.source_detail}</div>
									{#if finding.recommended_action}
										<div class="mt-2 text-sm text-gray-600">
											<strong>Recommended action:</strong>
											{finding.recommended_action}
										</div>
									{/if}
									<div class="mt-1 text-xs text-gray-400">
										Found {formatDate(finding.found_at)}
									</div>
								</div>
								{#if !finding.remediated}
									<button
										onclick={() => markRemediated(finding.id)}
										class="ml-4 rounded-md bg-gray-100 px-3 py-1 text-sm text-gray-700 hover:bg-gray-200"
									>
										Mark as Remediated
									</button>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>
