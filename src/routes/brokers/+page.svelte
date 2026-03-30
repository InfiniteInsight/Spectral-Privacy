<script lang="ts">
	import { goto } from '$app/navigation';
	import { onDestroy } from 'svelte';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { brokerAPI, type BrokerSummary } from '$lib/api/brokers';
	import { removalAPI } from '$lib/api/removal';
	import { vaultStore, profileStore } from '$lib/stores';
	import { getDifficultyColor, getCategoryDisplay } from '$lib/utils/broker';

	let brokers = $state<BrokerSummary[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let searchQuery = $state('');
	let categoryFilter = $state('all');
	let difficultyFilter = $state('all');

	const vaultId = $derived(vaultStore.currentVaultId ?? '');
	let profileId = $state('');

	// Bulk email state
	let bulkEmailing = $state(false);
	let bulkTotal = $state(0);
	let bulkDoneCount = $state(0);
	let bulkFailCount = $state(0);
	let bulkError = $state<string | null>(null);
	let bulkComplete = $state(false);
	let bulkUnlistenFns: UnlistenFn[] = [];

	$effect(() => {
		const profiles = profileStore.profiles;
		if (profiles.length > 0 && !profileId) {
			profileId = profiles[0].id;
		}
	});

	// Load brokers on mount using $effect
	$effect(() => {
		async function loadBrokers() {
			loading = true;
			error = null;
			try {
				brokers = await brokerAPI.listBrokers();
			} catch (err) {
				error = 'Failed to load broker list. Please try again.';
				console.error('Failed to load brokers:', err);
			} finally {
				loading = false;
			}
		}

		loadBrokers();
	});

	// Filtered brokers using $derived
	const filteredBrokers = $derived.by(() => {
		let result = brokers;

		if (searchQuery.trim()) {
			const query = searchQuery.toLowerCase();
			result = result.filter(
				(b) => b.name.toLowerCase().includes(query) || b.domain.toLowerCase().includes(query)
			);
		}

		if (categoryFilter !== 'all') {
			result = result.filter((b) => b.category === categoryFilter);
		}

		if (difficultyFilter !== 'all') {
			result = result.filter((b) => b.difficulty === difficultyFilter);
		}

		return result;
	});

	const emailBrokers = $derived(brokers.filter((b) => b.removal_method.startsWith('Email')));

	const categories = $derived(['all', ...new Set(brokers.map((b) => b.category))]);
	const difficulties = $derived(['all', ...new Set(brokers.map((b) => b.difficulty))]);

	function handleRowClick(brokerId: string) {
		goto(`/brokers/${brokerId}`);
	}

	async function handleEmailAll() {
		if (!vaultId || !profileId || bulkEmailing) return;

		bulkEmailing = true;
		bulkComplete = false;
		bulkError = null;
		bulkDoneCount = 0;
		bulkFailCount = 0;

		for (const u of bulkUnlistenFns) u();
		bulkUnlistenFns = [];

		try {
			const attemptIds = await removalAPI.initiateBulkRemoval(
				vaultId,
				profileId,
				emailBrokers.map((b) => b.id)
			);
			bulkTotal = attemptIds.length;

			if (bulkTotal === 0) {
				bulkComplete = true;
				bulkEmailing = false;
				return;
			}

			const attemptIdSet = new Set(attemptIds);

			const unlistenSuccess = await listen<{ removal_attempt_id: string }>(
				'removal:success',
				(event) => {
					if (attemptIdSet.has(event.payload.removal_attempt_id)) {
						bulkDoneCount++;
						if (bulkDoneCount + bulkFailCount >= bulkTotal) {
							bulkEmailing = false;
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
							bulkEmailing = false;
							bulkComplete = true;
						}
					}
				}
			);
			bulkUnlistenFns.push(unlistenFailed);
		} catch (err) {
			bulkError = err instanceof Error ? err.message : String(err);
			bulkEmailing = false;
		}
	}

	onDestroy(() => {
		for (const u of bulkUnlistenFns) u();
	});
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 p-4">
	<div class="max-w-7xl mx-auto">
		<div class="bg-white rounded-lg shadow-xl p-8">
			<!-- Header -->
			<div class="mb-8">
				<div class="flex items-center justify-between mb-4">
					<div>
						<h1 class="text-3xl font-bold text-gray-900 mb-2">Broker Explorer</h1>
						<p class="text-gray-600">
							Browse all {brokers.length} data broker definitions in our database
						</p>
					</div>
					<div class="flex items-center gap-3">
						{#if vaultId && profileId && !loading && emailBrokers.length > 0}
							<button
								onclick={handleEmailAll}
								disabled={bulkEmailing}
								class="px-5 py-2 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
							>
								{bulkEmailing
									? `Emailing… ${bulkDoneCount + bulkFailCount}/${bulkTotal}`
									: `Email All (${emailBrokers.length})`}
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

				<!-- Bulk Email Progress -->
				{#if bulkEmailing || bulkComplete || bulkError}
					<div
						class="mb-6 p-4 rounded-lg border {bulkError
							? 'bg-red-50 border-red-200'
							: bulkComplete
								? 'bg-green-50 border-green-200'
								: 'bg-blue-50 border-blue-200'}"
					>
						{#if bulkError}
							<p class="text-sm text-red-700">Error: {bulkError}</p>
						{:else if bulkComplete}
							<p class="text-sm text-green-700">
								✓ Done — {bulkDoneCount} email{bulkDoneCount !== 1 ? 's' : ''} sent{bulkFailCount >
								0
									? `, ${bulkFailCount} failed`
									: ''}.
							</p>
						{:else}
							<div class="flex items-center gap-3">
								<div
									class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600 shrink-0"
								></div>
								<div class="flex-1">
									<div class="flex justify-between text-sm text-blue-800 mb-1">
										<span>Sending removal emails…</span>
										<span>{bulkDoneCount + bulkFailCount} / {bulkTotal}</span>
									</div>
									<div class="w-full bg-blue-200 rounded-full h-1.5">
										<div
											class="bg-blue-600 h-1.5 rounded-full transition-all"
											style="width: {bulkTotal > 0
												? Math.round(((bulkDoneCount + bulkFailCount) / bulkTotal) * 100)
												: 0}%"
										></div>
									</div>
								</div>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Search and Filters -->
				<div class="flex flex-col md:flex-row gap-4">
					<div class="flex-1">
						<input
							type="text"
							bind:value={searchQuery}
							placeholder="Search by name or domain..."
							aria-label="Search brokers by name or domain"
							class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
						/>
					</div>

					<div class="w-full md:w-48">
						<select
							bind:value={categoryFilter}
							aria-label="Filter by category"
							class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
						>
							{#each categories as category}
								<option value={category}>
									{category === 'all' ? 'All Categories' : getCategoryDisplay(category)}
								</option>
							{/each}
						</select>
					</div>

					<div class="w-full md:w-48">
						<select
							bind:value={difficultyFilter}
							aria-label="Filter by difficulty"
							class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
						>
							{#each difficulties as difficulty}
								<option value={difficulty}>
									{difficulty === 'all' ? 'All Difficulties' : difficulty}
								</option>
							{/each}
						</select>
					</div>
				</div>

				{#if !loading && (searchQuery.trim() !== '' || categoryFilter !== 'all' || difficultyFilter !== 'all')}
					<p class="text-sm text-gray-600 mt-3">
						Showing {filteredBrokers.length} of {brokers.length} brokers
					</p>
				{/if}
			</div>

			{#if loading}
				<div class="flex items-center justify-center py-12">
					<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
				</div>
			{:else if error}
				<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
					<p class="text-sm text-red-700">{error}</p>
				</div>
			{:else if filteredBrokers.length === 0}
				<div class="text-center py-12">
					<p class="text-gray-600">
						{searchQuery.trim() ? 'No brokers match your search criteria' : 'No brokers found'}
					</p>
				</div>
			{:else}
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
									class="border-b border-gray-100 hover:bg-gray-50 cursor-pointer transition-colors"
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
