<script lang="ts">
	import { goto } from '$app/navigation';
	import { brokerAPI, type BrokerSummary } from '$lib/api/brokers';
	import { getDifficultyColor, getCategoryDisplay } from '$lib/utils/broker';

	let brokers = $state<BrokerSummary[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let searchQuery = $state('');
	let difficultyFilter = $state('all');

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
