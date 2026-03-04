<script lang="ts">
	import { vaultStore } from '$lib/stores/vault.svelte';
	import { cookiesAPI, type CookieScanResponse, type ScannedCookie } from '$lib/api/cookies';
	import { brokerAPI, type BrokerSummary } from '$lib/api/brokers';
	import { goto } from '$app/navigation';

	let recentScans = $state<CookieScanResponse[]>([]);
	let unmatchedCookies = $state<ScannedCookie[]>([]);
	let brokerMap = $state<Map<string, BrokerSummary>>(new Map());
	let loading = $state(true);
	let error = $state<string | null>(null);
	let removingBroker = $state<string | null>(null);
	let showUnmatched = $state(false);
	let expandedBroker = $state<string | null>(null);
	let brokerCookies = $state<Map<string, ScannedCookie[]>>(new Map());
	let loadingBrokerCookies = $state(false);
	let removingAll = $state(false);
	let removingAllTracking = $state(false);
	let removingCookie = $state<string | null>(null);

	// Load data on mount
	$effect(() => {
		async function loadData() {
			if (!vaultStore.currentVaultId) {
				error = 'Please unlock a vault first';
				loading = false;
				return;
			}

			loading = true;
			error = null;

			try {
				// Load recent cookie scans
				const scans = await cookiesAPI.getRecentCookieScans(vaultStore.currentVaultId, 10);
				recentScans = scans;

				// Load unmatched cookies
				const unmatched = await cookiesAPI.getUnmatchedCookies(vaultStore.currentVaultId);
				unmatchedCookies = unmatched;

				// Load all brokers to map IDs to names
				const brokers = await brokerAPI.listBrokers();
				brokerMap = new Map(brokers.map((b) => [b.id, b]));
				console.log('[DEBUG] Loaded broker map with', brokerMap.size, 'brokers');
			} catch (err) {
				error = err instanceof Error ? err.message : String(err);
				console.error('Failed to load cookie data:', err);
			} finally {
				loading = false;
			}
		}

		loadData();
	});

	// Get the most recent scan
	const latestScan = $derived.by(() => {
		const scan = recentScans.length > 0 ? recentScans[0] : null;
		if (scan) {
			console.log('[DEBUG] latestScan updated:', {
				scanId: scan.scanId,
				totalCookies: scan.totalCookies,
				matchedCookies: scan.matchedCookies,
				cookiesByBroker: scan.cookiesByBroker
			});
		} else {
			console.log('[DEBUG] latestScan is null, recentScans.length:', recentScans.length);
		}
		return scan;
	});

	// Get brokers that have cookies in the latest scan
	const brokersWithCookies = $derived.by(() => {
		if (!latestScan) {
			console.log('[DEBUG] brokersWithCookies: No latest scan');
			return [];
		}

		console.log('[DEBUG] latestScan.cookiesByBroker:', latestScan.cookiesByBroker);
		console.log('[DEBUG] cookiesByBroker type:', typeof latestScan.cookiesByBroker);
		console.log('[DEBUG] cookiesByBroker entries:', Object.entries(latestScan.cookiesByBroker));

		const result = Object.entries(latestScan.cookiesByBroker)
			.map(([brokerId, count]) => ({
				broker: brokerMap.get(brokerId),
				brokerId,
				count
			}))
			// Don't filter out unknown brokers - show them with ID
			.sort((a, b) => b.count - a.count);

		console.log('[DEBUG] brokersWithCookies result:', result);
		return result;
	});

	// Group unmatched cookies by domain
	const cookiesByDomain = $derived.by(() => {
		const grouped = new Map<string, ScannedCookie[]>();

		for (const cookie of unmatchedCookies) {
			const domain = cookie.cookieDomain;
			if (!grouped.has(domain)) {
				grouped.set(domain, []);
			}
			grouped.get(domain)!.push(cookie);
		}

		// Convert to sorted array
		return Array.from(grouped.entries())
			.map(([domain, cookies]) => ({ domain, cookies, count: cookies.length }))
			.sort((a, b) => b.count - a.count);
	});

	async function handleRemoveCookies(brokerId: string) {
		if (!vaultStore.currentVaultId || !confirm('Remove all cookies for this broker?')) return;

		removingBroker = brokerId;
		try {
			await cookiesAPI.removeCookiesForBroker(vaultStore.currentVaultId, brokerId);
			// Reload scans and clear expanded state
			const scans = await cookiesAPI.getRecentCookieScans(vaultStore.currentVaultId, 10);
			recentScans = scans;
			expandedBroker = null;
			brokerCookies.delete(brokerId);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
			console.error('Failed to remove cookies:', err);
		} finally {
			removingBroker = null;
		}
	}

	async function handleRemoveAllCookies() {
		if (
			!vaultStore.currentVaultId ||
			!confirm('Remove ALL cookies (including unmatched)? This cannot be undone.')
		)
			return;

		removingAll = true;
		try {
			await cookiesAPI.removeAllCookies(vaultStore.currentVaultId);
			// Reload scans
			const scans = await cookiesAPI.getRecentCookieScans(vaultStore.currentVaultId, 10);
			recentScans = scans;
			expandedBroker = null;
			brokerCookies.clear();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
			console.error('Failed to remove all cookies:', err);
		} finally {
			removingAll = false;
		}
	}

	async function handleRemoveAllTrackingCookies() {
		if (
			!vaultStore.currentVaultId ||
			!confirm('Remove all tracking cookies? This cannot be undone.')
		)
			return;

		removingAllTracking = true;
		try {
			await cookiesAPI.removeAllTrackingCookies(vaultStore.currentVaultId);
			// Reload scans
			const scans = await cookiesAPI.getRecentCookieScans(vaultStore.currentVaultId, 10);
			recentScans = scans;
			expandedBroker = null;
			brokerCookies.clear();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
			console.error('Failed to remove tracking cookies:', err);
		} finally {
			removingAllTracking = false;
		}
	}

	async function handleRemoveSingleCookie(cookieId: string, cookieName: string) {
		if (!vaultStore.currentVaultId || !confirm(`Remove cookie "${cookieName}"?`)) return;

		removingCookie = cookieId;
		try {
			await cookiesAPI.removeSingleCookie(vaultStore.currentVaultId, cookieId);

			// Remove from local state
			if (expandedBroker) {
				const cookies = brokerCookies.get(expandedBroker);
				if (cookies) {
					brokerCookies.set(
						expandedBroker,
						cookies.filter((c) => c.id !== cookieId)
					);
				}
			}

			// Reload scans to update counts
			const scans = await cookiesAPI.getRecentCookieScans(vaultStore.currentVaultId, 10);
			recentScans = scans;
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
			console.error('Failed to remove cookie:', err);
		} finally {
			removingCookie = null;
		}
	}

	async function handleOpenCookieLocation(browserType: string, profileName: string) {
		try {
			await cookiesAPI.openCookieLocation(browserType, profileName);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
			console.error('Failed to open cookie location:', err);
		}
	}

	async function toggleBrokerCookies(brokerId: string) {
		if (expandedBroker === brokerId) {
			expandedBroker = null;
			return;
		}

		if (!vaultStore.currentVaultId) return;

		expandedBroker = brokerId;

		// Load cookies if not already loaded
		if (!brokerCookies.has(brokerId)) {
			loadingBrokerCookies = true;
			try {
				const cookies = await cookiesAPI.getCookiesForBroker(vaultStore.currentVaultId, brokerId);
				brokerCookies.set(brokerId, cookies);
			} catch (err) {
				error = err instanceof Error ? err.message : String(err);
				console.error('Failed to load broker cookies:', err);
				expandedBroker = null;
			} finally {
				loadingBrokerCookies = false;
			}
		}
	}

	function formatDate(timestamp: string): string {
		return new Date(timestamp).toLocaleString();
	}
</script>

<div class="min-h-screen bg-gradient-to-br from-purple-50 to-purple-100 p-4">
	<div class="mx-auto max-w-7xl">
		<div class="rounded-lg bg-white p-8 shadow-xl">
			<!-- Header -->
			<div class="mb-8">
				<div class="mb-4 flex items-center justify-between">
					<div>
						<h1 class="mb-2 text-3xl font-bold text-gray-900">Cookie Scanner Results</h1>
						<p class="text-gray-600">View and manage tracking cookies from your browsers</p>
					</div>
					<div class="flex gap-2">
						{#if latestScan && latestScan.totalCookies > 0}
							<button
								onclick={handleRemoveAllTrackingCookies}
								disabled={removingAllTracking}
								class="cursor-pointer rounded-lg bg-orange-600 px-4 py-2 text-white transition-colors hover:bg-orange-700 disabled:cursor-not-allowed disabled:opacity-50"
								title="Remove all tracking cookies (matched to brokers)"
							>
								{removingAllTracking ? 'Removing...' : 'Delete All Tracking'}
							</button>
							<button
								onclick={handleRemoveAllCookies}
								disabled={removingAll}
								class="cursor-pointer rounded-lg bg-red-600 px-4 py-2 text-white transition-colors hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-50"
								title="Remove all scanned cookies (including unmatched)"
							>
								{removingAll ? 'Removing...' : 'Delete All Cookies'}
							</button>
						{/if}
						<button
							onclick={() => goto('/scan')}
							class="cursor-pointer rounded-lg bg-purple-600 px-4 py-2 text-white transition-colors hover:bg-purple-700"
						>
							Run New Scan
						</button>
						<button
							onclick={() => goto('/')}
							class="cursor-pointer px-4 py-2 text-gray-600 transition-colors hover:text-gray-900"
						>
							← Back
						</button>
					</div>
				</div>
			</div>

			{#if loading}
				<div class="rounded-lg border border-gray-200 bg-white p-8 text-center">
					<div
						class="mx-auto h-8 w-8 animate-spin rounded-full border-4 border-gray-200 border-t-purple-600"
					></div>
					<p class="mt-2 text-sm text-gray-500">Loading cookie scan results...</p>
				</div>
			{:else if error}
				<div class="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-900">
					{error}
				</div>
			{:else if !latestScan}
				<div class="rounded-lg border border-blue-200 bg-blue-50 p-8 text-center">
					<h3 class="mb-2 text-lg font-semibold text-blue-900">No Cookie Scans Yet</h3>
					<p class="mb-4 text-blue-700">
						Run your first cookie scan to see which tracking cookies are on your system.
					</p>
					<button
						onclick={() => goto('/scan')}
						class="cursor-pointer rounded-lg bg-purple-600 px-6 py-3 text-white hover:bg-purple-700"
					>
						Run Cookie Scan
					</button>
				</div>
			{:else}
				<!-- Latest Scan Summary -->
				<div class="mb-6 rounded-lg border border-purple-200 bg-purple-50 p-6">
					<div class="mb-4 flex items-center justify-between">
						<h2 class="text-xl font-semibold text-purple-900">Latest Scan</h2>
						<span class="text-sm text-purple-700">{formatDate(latestScan.timestamp)}</span>
					</div>

					<div class="grid grid-cols-1 gap-4 md:grid-cols-4">
						<div class="rounded-lg bg-white p-4">
							<div class="text-2xl font-bold text-purple-600">{latestScan.totalCookies}</div>
							<div class="text-sm text-gray-600">Total Cookies</div>
						</div>
						<div class="rounded-lg bg-white p-4">
							<div class="text-2xl font-bold text-orange-600">{latestScan.matchedCookies}</div>
							<div class="text-sm text-gray-600">Tracking Cookies</div>
						</div>
						<div class="rounded-lg bg-white p-4">
							<div class="text-2xl font-bold text-blue-600">
								{Object.keys(latestScan.cookiesByBroker).length}
							</div>
							<div class="text-sm text-gray-600">Brokers Detected</div>
						</div>
						<div class="rounded-lg bg-white p-4">
							<div class="mb-1 text-sm font-medium text-gray-700">Browsers Scanned</div>
							<div class="text-base font-semibold text-green-600">
								{latestScan.browsersScanned.length > 0
									? latestScan.browsersScanned.join(', ')
									: 'None'}
							</div>
						</div>
					</div>
				</div>

				<!-- Brokers with Cookies -->
				{#if latestScan.matchedCookies > 0}
					<div class="mb-6">
						<h2 class="mb-4 text-xl font-semibold text-gray-900">Tracking Cookies by Broker</h2>
						<div class="overflow-hidden rounded-lg border border-gray-200">
							<table class="min-w-full divide-y divide-gray-200">
								<thead class="bg-gray-50">
									<tr>
										<th
											class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
										>
											Broker
										</th>
										<th
											class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
										>
											Domain
										</th>
										<th
											class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
										>
											Cookies Found
										</th>
										<th
											class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
										>
											Actions
										</th>
									</tr>
								</thead>
								<tbody class="divide-y divide-gray-200 bg-white">
									{#each brokersWithCookies as { broker, brokerId, count }}
										<tr class="hover:bg-gray-50">
											<td class="whitespace-nowrap px-6 py-4">
												<div class="font-medium text-gray-900">{broker?.name || brokerId}</div>
												{#if !broker}
													<div class="text-xs text-gray-500">
														Broker ID: {brokerId}
													</div>
												{/if}
											</td>
											<td class="whitespace-nowrap px-6 py-4 text-sm text-gray-600">
												{broker?.domain || 'Unknown'}
											</td>
											<td class="whitespace-nowrap px-6 py-4">
												<button
													onclick={() => toggleBrokerCookies(brokerId)}
													class="inline-flex cursor-pointer rounded-full bg-orange-100 px-2 py-1 text-xs font-semibold text-orange-800 transition-colors hover:bg-orange-200"
													title="Click to view cookie details"
												>
													{count}
													{count === 1 ? 'cookie' : 'cookies'}
													{#if expandedBroker === brokerId}
														▲
													{:else}
														▼
													{/if}
												</button>
											</td>
											<td class="whitespace-nowrap px-6 py-4 text-sm">
												<button
													onclick={() => handleRemoveCookies(brokerId)}
													disabled={removingBroker === brokerId}
													class="cursor-pointer text-red-600 hover:text-red-900 disabled:cursor-not-allowed disabled:opacity-50"
												>
													{removingBroker === brokerId ? 'Removing...' : 'Remove Cookies'}
												</button>
											</td>
										</tr>

										<!-- Expanded cookie details -->
										{#if expandedBroker === brokerId}
											<tr>
												<td colspan="4" class="bg-gray-50 px-6 py-4">
													{#if loadingBrokerCookies}
														<div class="flex items-center justify-center py-4">
															<div
																class="h-6 w-6 animate-spin rounded-full border-2 border-gray-300 border-t-purple-600"
															></div>
															<span class="ml-2 text-sm text-gray-600">Loading cookies...</span>
														</div>
													{:else if brokerCookies.has(brokerId)}
														{@const cookies = brokerCookies.get(brokerId) || []}
														{#if cookies.length > 0}
															<div
																class="overflow-hidden rounded-lg border border-gray-200 bg-white"
															>
																<table class="min-w-full divide-y divide-gray-200">
																	<thead class="bg-gray-100">
																		<tr>
																			<th
																				class="px-4 py-2 text-left text-xs font-medium uppercase tracking-wider text-gray-600"
																			>
																				Cookie Name
																			</th>
																			<th
																				class="px-4 py-2 text-left text-xs font-medium uppercase tracking-wider text-gray-600"
																			>
																				Domain
																			</th>
																			<th
																				class="px-4 py-2 text-left text-xs font-medium uppercase tracking-wider text-gray-600"
																			>
																				Browser
																			</th>
																			<th
																				class="px-4 py-2 text-left text-xs font-medium uppercase tracking-wider text-gray-600"
																			>
																				Profile
																			</th>
																			<th
																				class="px-4 py-2 text-left text-xs font-medium uppercase tracking-wider text-gray-600"
																			>
																				Actions
																			</th>
																		</tr>
																	</thead>
																	<tbody class="divide-y divide-gray-200 bg-white">
																		{#each cookies as cookie}
																			<tr class="hover:bg-gray-50">
																				<td class="px-4 py-2 text-sm font-medium text-gray-900">
																					{cookie.cookieName}
																				</td>
																				<td class="px-4 py-2 text-sm text-gray-600">
																					{cookie.cookieDomain}
																				</td>
																				<td class="px-4 py-2 text-sm text-gray-600">
																					{cookie.browserType}
																				</td>
																				<td class="px-4 py-2 text-sm text-gray-600">
																					{cookie.profileName}
																				</td>
																				<td class="px-4 py-2 text-sm">
																					<div class="flex gap-2">
																						<button
																							onclick={() =>
																								handleOpenCookieLocation(
																									cookie.browserType,
																									cookie.profileName
																								)}
																							class="cursor-pointer text-blue-600 hover:text-blue-900"
																							title="Open cookie location in file browser"
																						>
																							📁
																						</button>
																						{#if cookie.id}
																							<button
																								onclick={() =>
																									handleRemoveSingleCookie(
																										cookie.id!,
																										cookie.cookieName
																									)}
																								disabled={removingCookie === cookie.id}
																								class="cursor-pointer text-red-600 hover:text-red-900 disabled:cursor-not-allowed disabled:opacity-50"
																							>
																								{removingCookie === cookie.id
																									? 'Deleting...'
																									: 'Delete'}
																							</button>
																						{:else}
																							<span class="text-gray-400">N/A</span>
																						{/if}
																					</div>
																				</td>
																			</tr>
																		{/each}
																	</tbody>
																</table>
															</div>
														{:else}
															<div class="py-4 text-center text-sm text-gray-600">
																No cookies found for this broker.
															</div>
														{/if}
													{/if}
												</td>
											</tr>
										{/if}
									{/each}
								</tbody>
							</table>
						</div>
					</div>
				{:else}
					<div class="mb-6 rounded-lg border border-green-200 bg-green-50 p-6 text-center">
						<h3 class="mb-2 text-lg font-semibold text-green-900">No Tracking Cookies Found</h3>
						<p class="text-green-700">
							Your browsers don't have any cookies from known data brokers.
						</p>
					</div>
				{/if}

				<!-- Unmatched Cookies -->
				{#if unmatchedCookies.length > 0}
					<div class="mb-6">
						<div class="mb-4 flex items-center justify-between">
							<div>
								<h2 class="text-xl font-semibold text-gray-900">Unmatched Cookies</h2>
								<p class="text-sm text-gray-600">
									{unmatchedCookies.length} cookies from unknown sources or not tracked by brokers
								</p>
							</div>
							<button
								onclick={() => (showUnmatched = !showUnmatched)}
								class="cursor-pointer rounded-lg border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
							>
								{showUnmatched ? 'Hide' : 'Show'} Details
							</button>
						</div>

						{#if showUnmatched}
							<div class="overflow-hidden rounded-lg border border-gray-200">
								<table class="min-w-full divide-y divide-gray-200">
									<thead class="bg-gray-50">
										<tr>
											<th
												class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
											>
												Domain
											</th>
											<th
												class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
											>
												Cookie Count
											</th>
											<th
												class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
											>
												Cookie Names
											</th>
										</tr>
									</thead>
									<tbody class="divide-y divide-gray-200 bg-white">
										{#each cookiesByDomain as { domain, cookies, count }}
											<tr class="hover:bg-gray-50">
												<td class="px-6 py-4">
													<div class="font-medium text-gray-900">{domain}</div>
												</td>
												<td class="whitespace-nowrap px-6 py-4">
													<span
														class="inline-flex rounded-full bg-blue-100 px-2 py-1 text-xs font-semibold text-blue-800"
													>
														{count}
														{count === 1 ? 'cookie' : 'cookies'}
													</span>
												</td>
												<td class="px-6 py-4">
													<div class="max-w-2xl text-sm text-gray-600">
														{cookies.map((c) => c.cookieName).join(', ')}
													</div>
												</td>
											</tr>
										{/each}
									</tbody>
								</table>
							</div>

							<div class="mt-4 rounded-lg border border-blue-200 bg-blue-50 p-4">
								<h4 class="mb-2 font-medium text-blue-900">About Unmatched Cookies</h4>
								<ul class="space-y-1 text-sm text-blue-700">
									<li>
										• These cookies don't match any known data broker patterns in our database
									</li>
									<li>• They may be legitimate website cookies (sessions, preferences, etc.)</li>
									<li>• Or they could be tracking cookies from services we haven't added yet</li>
									<li>
										• Check the domain names - common tracking domains include advertising and
										analytics services
									</li>
								</ul>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Scan History -->
				{#if recentScans.length > 1}
					<div>
						<h2 class="mb-4 text-xl font-semibold text-gray-900">Recent Scans</h2>
						<div class="overflow-hidden rounded-lg border border-gray-200">
							<table class="min-w-full divide-y divide-gray-200">
								<thead class="bg-gray-50">
									<tr>
										<th
											class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
										>
											Date
										</th>
										<th
											class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
										>
											Total Cookies
										</th>
										<th
											class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
										>
											Tracking Cookies
										</th>
										<th
											class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-gray-500"
										>
											Browsers
										</th>
									</tr>
								</thead>
								<tbody class="divide-y divide-gray-200 bg-white">
									{#each recentScans as scan}
										<tr class="hover:bg-gray-50">
											<td class="whitespace-nowrap px-6 py-4 text-sm text-gray-900">
												{formatDate(scan.timestamp)}
											</td>
											<td class="whitespace-nowrap px-6 py-4 text-sm text-gray-600">
												{scan.totalCookies}
											</td>
											<td class="whitespace-nowrap px-6 py-4">
												<span
													class="inline-flex rounded-full bg-orange-100 px-2 py-1 text-xs font-semibold text-orange-800"
												>
													{scan.matchedCookies}
												</span>
											</td>
											<td class="whitespace-nowrap px-6 py-4 text-sm text-gray-600">
												{scan.browsersScanned.length}
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					</div>
				{/if}
			{/if}
		</div>
	</div>
</div>
