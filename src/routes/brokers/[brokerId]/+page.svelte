<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { brokerAPI, type BrokerDetail } from '$lib/api/brokers';
	import { vaultStore } from '$lib/stores';
	import { getDifficultyColor, getCategoryDisplay } from '$lib/utils/broker';

	const brokerId = $derived($page.params.brokerId);

	let broker = $state<BrokerDetail | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Load broker detail using $effect
	$effect(() => {
		async function loadBrokerDetail() {
			if (!brokerId) {
				error = 'No broker ID provided';
				loading = false;
				return;
			}

			if (!vaultStore.currentVaultId) {
				error = 'No vault selected. Please unlock a vault first.';
				loading = false;
				return;
			}

			loading = true;
			error = null;
			try {
				broker = await brokerAPI.getBrokerDetail(brokerId, vaultStore.currentVaultId);
			} catch (err) {
				error = 'Failed to load broker details. Please try again.';
				console.error('Failed to load broker detail:', err);
			} finally {
				loading = false;
			}
		}

		loadBrokerDetail();
	});

	function getRemovalMethodDisplay(method: string): string {
		// Convert PascalCase to readable format
		return method.replace(/([A-Z])/g, ' $1').trim();
	}

	function getScanStatusDisplay(status: string | null): { text: string; color: string } {
		if (!status) {
			return { text: 'Not Scanned', color: 'text-gray-700 bg-gray-100' };
		}

		switch (status) {
			case 'Found':
				return { text: 'Found', color: 'text-red-700 bg-red-100' };
			case 'NotFound':
				return { text: 'Not Found', color: 'text-green-700 bg-green-100' };
			default:
				return { text: status, color: 'text-gray-700 bg-gray-100' };
		}
	}

	function formatDate(dateString: string): string {
		try {
			const date = new Date(dateString);
			return date.toLocaleDateString('en-US', {
				year: 'numeric',
				month: 'long',
				day: 'numeric'
			});
		} catch {
			return dateString;
		}
	}
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 p-4">
	<div class="max-w-4xl mx-auto">
		<div class="bg-white rounded-lg shadow-xl p-8">
			<!-- Back Button -->
			<button
				onclick={() => goto('/brokers')}
				class="mb-6 text-gray-600 hover:text-gray-900 transition-colors flex items-center gap-2"
			>
				← Back to Broker List
			</button>

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
			{:else if broker}
				<!-- Broker Details -->
				<div>
					<!-- Header -->
					<div class="mb-8">
						<h1 class="text-3xl font-bold text-gray-900 mb-2">{broker.name}</h1>
						<a
							href={broker.url}
							target="_blank"
							rel="noopener noreferrer"
							class="text-primary-600 hover:text-primary-700 hover:underline"
						>
							{broker.domain} ↗
						</a>
					</div>

					<!-- Key Information Grid -->
					<div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
						<!-- Category -->
						<div class="p-4 bg-gray-50 rounded-lg">
							<h3 class="text-sm font-medium text-gray-700 mb-1">Category</h3>
							<p class="text-lg font-semibold text-gray-900">
								{getCategoryDisplay(broker.category)}
							</p>
						</div>

						<!-- Difficulty -->
						<div class="p-4 bg-gray-50 rounded-lg">
							<h3 class="text-sm font-medium text-gray-700 mb-1">Removal Difficulty</h3>
							<span
								class="inline-block px-3 py-1 rounded text-sm font-medium {getDifficultyColor(
									broker.difficulty
								)}"
							>
								{broker.difficulty}
							</span>
						</div>

						<!-- Removal Method -->
						<div class="p-4 bg-gray-50 rounded-lg">
							<h3 class="text-sm font-medium text-gray-700 mb-1">Removal Method</h3>
							<p class="text-lg font-semibold text-gray-900">
								{getRemovalMethodDisplay(broker.removal_method)}
							</p>
						</div>

						<!-- Typical Removal Time -->
						<div class="p-4 bg-gray-50 rounded-lg">
							<h3 class="text-sm font-medium text-gray-700 mb-1">Typical Removal Time</h3>
							<p class="text-lg font-semibold text-gray-900">
								{broker.typical_removal_days}
								{broker.typical_removal_days === 1 ? 'day' : 'days'}
							</p>
						</div>

						<!-- Recheck Interval -->
						<div class="p-4 bg-gray-50 rounded-lg">
							<h3 class="text-sm font-medium text-gray-700 mb-1">Recheck Interval</h3>
							<p class="text-lg font-semibold text-gray-900">
								Every {broker.recheck_interval_days}
								{broker.recheck_interval_days === 1 ? 'day' : 'days'}
							</p>
						</div>

						<!-- Last Verified -->
						<div class="p-4 bg-gray-50 rounded-lg">
							<h3 class="text-sm font-medium text-gray-700 mb-1">Last Verified</h3>
							<p class="text-lg font-semibold text-gray-900">{formatDate(broker.last_verified)}</p>
						</div>
					</div>

					<!-- Your Scan Status -->
					{#if broker.scan_status}
						{@const statusDisplay = getScanStatusDisplay(broker.scan_status)}
						<div class="mb-8 p-6 border-2 border-gray-200 rounded-lg">
							<h2 class="text-xl font-bold text-gray-900 mb-4">Your Scan Status</h2>
							<div class="flex items-center justify-between">
								<div>
									<p class="text-sm text-gray-600 mb-2">Status on this broker</p>
									<span
										class="inline-block px-3 py-1 rounded text-sm font-medium {statusDisplay.color}"
									>
										{statusDisplay.text}
									</span>
								</div>
								{#if broker.finding_count !== null && broker.finding_count > 0}
									<div class="text-right">
										<p class="text-3xl font-bold text-red-600">{broker.finding_count}</p>
										<p class="text-sm text-gray-600">
											{broker.finding_count === 1 ? 'finding' : 'findings'}
										</p>
									</div>
								{/if}
							</div>
						</div>
					{:else}
						<div class="mb-8 p-6 bg-blue-50 border border-blue-200 rounded-lg">
							<h2 class="text-xl font-bold text-gray-900 mb-2">No Scan Data</h2>
							<p class="text-sm text-gray-700">
								You haven't scanned this broker yet. Start a scan to check if your information
								appears on this site.
							</p>
						</div>
					{/if}

					<!-- Action Buttons -->
					<div class="flex flex-col sm:flex-row gap-4">
						<a
							href={broker.url}
							target="_blank"
							rel="noopener noreferrer"
							class="flex-1 px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors text-center"
							style="background-color: #0284c7; color: white;"
						>
							Visit Broker Website ↗
						</a>
						<button
							onclick={() => goto('/scan/start')}
							class="flex-1 px-6 py-3 border border-gray-300 text-gray-700 rounded-lg font-medium hover:bg-gray-50 transition-colors"
						>
							Scan This Broker
						</button>
					</div>
				</div>
			{:else}
				<!-- No Data -->
				<div class="text-center py-12">
					<p class="text-gray-600">Broker not found</p>
				</div>
			{/if}
		</div>
	</div>
</div>
