<script lang="ts">
	import { onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { brokerAPI, type BrokerSummary } from '$lib/api/brokers';
	import { removalAPI } from '$lib/api/removal';
	import { vaultStore, profileStore } from '$lib/stores';
	import { getDifficultyColor, getCategoryDisplay } from '$lib/utils/broker';

	let brokers = $state<BrokerSummary[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let searchQuery = $state('');
	let difficultyFilter = $state('all');

	// Vault / profile state
	const vaultId = $derived(vaultStore.currentVaultId ?? '');
	let profileId = $state('');

	// Bulk removal state
	let bulkRemoving = $state(false);
	let bulkTotal = $state(0);
	let bulkDoneCount = $state(0);
	let bulkFailCount = $state(0);
	let bulkError = $state<string | null>(null);
	let bulkComplete = $state(false);
	let bulkUnlistenFns: UnlistenFn[] = [];

	// Resolve profile ID from store
	$effect(() => {
		const profiles = profileStore.profiles;
		if (profiles.length > 0 && !profileId) {
			profileId = profiles[0].id;
		}
	});

	// Load adtech brokers (Marketing category) on mount using $effect
	$effect(() => {
		async function loadAdtechBrokers() {
			loading = true;
			error = null;
			try {
				const allBrokers = await brokerAPI.listBrokers();
				// Filter to only show Marketing category (AdTech companies)
				brokers = allBrokers.filter((b) => b.category === 'Marketing');
			} catch (err) {
				error = 'Failed to load adtech list. Please try again.';
				console.error('Failed to load adtech brokers:', err);
			} finally {
				loading = false;
			}
		}

		loadAdtechBrokers();
	});

	// Filtered brokers using $derived
	const filteredBrokers = $derived.by(() => {
		let result = brokers;

		// Apply search filter
		if (searchQuery.trim()) {
			const query = searchQuery.toLowerCase();
			result = result.filter(
				(b) => b.name.toLowerCase().includes(query) || b.domain.toLowerCase().includes(query)
			);
		}

		// Apply difficulty filter
		if (difficultyFilter !== 'all') {
			result = result.filter((b) => b.difficulty === difficultyFilter);
		}

		return result;
	});

	// Get unique difficulties for filter dropdown
	const difficulties = $derived(['all', ...new Set(brokers.map((b) => b.difficulty))]);

	function handleRowClick(brokerId: string) {
		goto(`/adtech/${brokerId}`);
	}

	async function handleRemoveAll() {
		if (!vaultId || !profileId || bulkRemoving) return;

		bulkRemoving = true;
		bulkComplete = false;
		bulkError = null;
		bulkDoneCount = 0;
		bulkFailCount = 0;

		// Clean up any previous listeners
		for (const u of bulkUnlistenFns) u();
		bulkUnlistenFns = [];

		try {
			const attemptIds = await removalAPI.initiateBulkRemoval(
				vaultId,
				profileId,
				brokers.map((b) => b.id)
			);
			bulkTotal = attemptIds.length;

			const attemptIdSet = new Set(attemptIds);

			const unlistenSuccess = await listen<{ removal_attempt_id: string }>(
				'removal:success',
				(event) => {
					if (attemptIdSet.has(event.payload.removal_attempt_id)) {
						bulkDoneCount++;
						if (bulkDoneCount + bulkFailCount >= bulkTotal) {
							bulkRemoving = false;
							bulkComplete = true;
						}
					}
				}
			);
			bulkUnlistenFns.push(unlistenSuccess);

			const unlistenFailed = await listen<{ removal_attempt_id: string }>(
				'removal:failed',
				(event) => {
					if (attemptIdSet.has(event.payload.removal_attempt_id)) {
						bulkFailCount++;
						if (bulkDoneCount + bulkFailCount >= bulkTotal) {
							bulkRemoving = false;
							bulkComplete = true;
						}
					}
				}
			);
			bulkUnlistenFns.push(unlistenFailed);
		} catch (err) {
			bulkError = err instanceof Error ? err.message : String(err);
			bulkRemoving = false;
		}
	}

	onDestroy(() => {
		for (const u of bulkUnlistenFns) u();
	});
</script>

<div class="min-h-screen bg-gradient-to-br from-orange-50 to-orange-100 p-4">
	<div class="max-w-7xl mx-auto">
		<div class="bg-white rounded-lg shadow-xl p-8">
			<!-- Header -->
			<div class="mb-8">
				<div class="flex items-center justify-between mb-4">
					<div>
						<h1 class="text-3xl font-bold text-gray-900 mb-2">AdTech Companies</h1>
						<p class="text-gray-600">
							Browse all {brokers.length} advertising technology and marketing data companies
						</p>
					</div>
					<div class="flex items-center gap-3">
						{#if vaultId && profileId && !loading}
							<button
								onclick={handleRemoveAll}
								disabled={bulkRemoving}
								class="px-5 py-2 bg-orange-600 text-white rounded-lg font-medium hover:bg-orange-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
							>
								{bulkRemoving ? 'Removing...' : 'Remove All'}
							</button>
						{/if}
						<button
							onclick={() => goto('/')}
							class="px-4 py-2 text-gray-600 hover:text-gray-900 transition-colors"
						>
							← Back to Dashboard
						</button>
					</div>
				</div>

				<!-- Search and Filters -->
				<div class="flex flex-col md:flex-row gap-4">
					<!-- Search Box -->
					<div class="flex-1">
						<input
							type="text"
							bind:value={searchQuery}
							placeholder="Search by name or domain..."
							aria-label="Search adtech companies by name or domain"
							class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent"
						/>
					</div>

					<!-- Difficulty Filter -->
					<div class="w-full md:w-48">
						<select
							bind:value={difficultyFilter}
							aria-label="Filter by difficulty"
							class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent"
						>
							{#each difficulties as difficulty}
								<option value={difficulty}>
									{difficulty === 'all' ? 'All Difficulties' : difficulty}
								</option>
							{/each}
						</select>
					</div>
				</div>

				<!-- Results Count -->
				{#if !loading && (searchQuery.trim() !== '' || difficultyFilter !== 'all')}
					<p class="text-sm text-gray-600 mt-3">
						Showing {filteredBrokers.length} of {brokers.length} companies
					</p>
				{/if}
			</div>

			<!-- Bulk Removal Progress -->
			{#if bulkRemoving || bulkComplete || bulkError}
				<div
					class="mb-6 p-4 border rounded-lg {bulkError
						? 'bg-red-50 border-red-200'
						: bulkComplete && bulkFailCount === 0
							? 'bg-green-50 border-green-200'
							: 'bg-orange-50 border-orange-200'}"
				>
					{#if bulkError}
						<p class="text-sm text-red-700 font-medium">Remove All failed: {bulkError}</p>
					{:else if bulkComplete}
						<p
							class="text-sm font-medium {bulkFailCount > 0 ? 'text-orange-700' : 'text-green-700'}"
						>
							Remove All complete — {bulkDoneCount} submitted{bulkFailCount > 0
								? `, ${bulkFailCount} failed`
								: ''}
						</p>
					{:else}
						<div>
							<div class="flex items-center justify-between mb-2">
								<p class="text-sm font-medium text-orange-700">Submitting removals…</p>
								<p class="text-sm text-orange-700">{bulkDoneCount + bulkFailCount} / {bulkTotal}</p>
							</div>
							<div class="w-full bg-orange-200 rounded-full h-2">
								<div
									class="bg-orange-600 h-2 rounded-full transition-all"
									style="width: {bulkTotal > 0
										? Math.round(((bulkDoneCount + bulkFailCount) / bulkTotal) * 100)
										: 0}%"
								></div>
							</div>
						</div>
					{/if}
				</div>
			{/if}

			{#if loading}
				<!-- Loading State -->
				<div class="flex items-center justify-center py-12">
					<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-orange-600"></div>
				</div>
			{:else if error}
				<!-- Error State -->
				<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
					<p class="text-sm text-red-700">{error}</p>
				</div>
			{:else if filteredBrokers.length === 0}
				<!-- No Results -->
				<div class="text-center py-12">
					<p class="text-gray-600">
						{searchQuery.trim()
							? 'No companies match your search criteria'
							: 'No adtech companies found'}
					</p>
				</div>
			{:else}
				<!-- AdTech Table -->
				<div class="overflow-x-auto">
					<table class="w-full">
						<thead>
							<tr class="border-b border-gray-200">
								<th class="text-left py-3 px-4 text-sm font-semibold text-gray-900">Name</th>
								<th class="text-left py-3 px-4 text-sm font-semibold text-gray-900">Domain</th>
								<th class="text-left py-3 px-4 text-sm font-semibold text-gray-900">Category</th>
								<th class="text-left py-3 px-4 text-sm font-semibold text-gray-900">Difficulty</th>
								<th class="text-left py-3 px-4 text-sm font-semibold text-gray-900">
									Removal Time
								</th>
							</tr>
						</thead>
						<tbody>
							{#each filteredBrokers as broker (broker.id)}
								<tr
									role="button"
									tabindex="0"
									onclick={() => handleRowClick(broker.id)}
									onkeypress={(e) => {
										if (e.key === 'Enter' || e.key === ' ') {
											e.preventDefault();
											handleRowClick(broker.id);
										}
									}}
									aria-label="View details for {broker.name}"
									class="border-b border-gray-100 hover:bg-orange-50 cursor-pointer transition-colors"
								>
									<td class="py-3 px-4 font-medium text-gray-900">{broker.name}</td>
									<td class="py-3 px-4 text-gray-600 text-sm">{broker.domain}</td>
									<td class="py-3 px-4 text-gray-600 text-sm">
										{getCategoryDisplay(broker.category)}
									</td>
									<td class="py-3 px-4">
										<span
											class="inline-block px-2 py-1 rounded text-xs font-medium {getDifficultyColor(
												broker.difficulty
											)}"
										>
											{broker.difficulty}
										</span>
									</td>
									<td class="py-3 px-4 text-gray-600 text-sm">
										{broker.typical_removal_days}
										{broker.typical_removal_days === 1 ? 'day' : 'days'}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</div>
	</div>
</div>
