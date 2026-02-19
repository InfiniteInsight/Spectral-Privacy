<script lang="ts">
	import { goto } from '$app/navigation';
	import { brokerAPI, type BrokerSummary } from '$lib/api/brokers';
	import { getDifficultyColor, getCategoryDisplay } from '$lib/utils/broker';

	let brokers = $state<BrokerSummary[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let searchQuery = $state('');
	let categoryFilter = $state('all');
	let difficultyFilter = $state('all');

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

		// Apply search filter
		if (searchQuery.trim()) {
			const query = searchQuery.toLowerCase();
			result = result.filter(
				(b) => b.name.toLowerCase().includes(query) || b.domain.toLowerCase().includes(query)
			);
		}

		// Apply category filter
		if (categoryFilter !== 'all') {
			result = result.filter((b) => b.category === categoryFilter);
		}

		// Apply difficulty filter
		if (difficultyFilter !== 'all') {
			result = result.filter((b) => b.difficulty === difficultyFilter);
		}

		return result;
	});

	// Get unique categories and difficulties for filter dropdowns
	const categories = $derived(['all', ...new Set(brokers.map((b) => b.category))]);

	const difficulties = $derived(['all', ...new Set(brokers.map((b) => b.difficulty))]);

	function handleRowClick(brokerId: string) {
		goto(`/brokers/${brokerId}`);
	}
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
					<button
						onclick={() => goto('/')}
						class="px-4 py-2 text-gray-600 hover:text-gray-900 transition-colors"
					>
						← Back to Dashboard
					</button>
				</div>

				<!-- Search and Filters -->
				<div class="flex flex-col md:flex-row gap-4">
					<!-- Search Box -->
					<div class="flex-1">
						<input
							type="text"
							bind:value={searchQuery}
							placeholder="Search by name or domain..."
							aria-label="Search brokers by name or domain"
							class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
						/>
					</div>

					<!-- Category Filter -->
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

					<!-- Difficulty Filter -->
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

				<!-- Results Count -->
				{#if !loading && (searchQuery.trim() !== '' || categoryFilter !== 'all' || difficultyFilter !== 'all')}
					<p class="text-sm text-gray-600 mt-3">
						Showing {filteredBrokers.length} of {brokers.length} brokers
					</p>
				{/if}
			</div>

			{#if loading}
				<!-- Loading State -->
				<div class="flex items-center justify-center py-12">
					<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
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
						{searchQuery.trim() ? 'No brokers match your search criteria' : 'No brokers found'}
					</p>
				</div>
			{:else}
				<!-- Broker Table -->
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
