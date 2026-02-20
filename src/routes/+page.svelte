<script lang="ts">
	import { UnlockScreen } from '$lib/components';
	import { vaultStore, profileStore } from '$lib/stores';
	import { goto } from '$app/navigation';
	import { getDashboardSummary, type DashboardSummary } from '$lib/api/dashboard';
	import { startScan } from '$lib/api/scan';

	// Reactive effect: Load profiles when vault is unlocked
	$effect(() => {
		if (vaultStore.isCurrentVaultUnlocked && vaultStore.currentVaultId) {
			// Load profiles whenever vault unlock state changes to true
			profileStore.loadProfiles(vaultStore.currentVaultId);
		}
	});

	// Get current profile (first profile for now)
	const currentProfile = $derived(
		profileStore.profiles.length > 0 ? profileStore.profiles[0] : null
	);

	const vaultId = $derived(vaultStore.currentVaultId);

	let dashboard = $state<DashboardSummary | null>(null);
	let dashboardError = $state<string | null>(null);
	let scanStarting = $state(false);

	// Detect first-run: no scans have ever been run
	const isFirstRun = $derived(dashboard !== null && dashboard.last_scan_at === null);

	$effect(() => {
		if (!vaultId) {
			dashboard = null;
			return;
		}
		getDashboardSummary(vaultId)
			.then((d) => {
				dashboard = d;
				dashboardError = null;
			})
			.catch((e) => {
				console.error('Failed to load dashboard:', e);
				dashboardError = e instanceof Error ? e.message : String(e);
			});
	});

	async function handleFirstRunScan() {
		if (!vaultId || !currentProfile) return;
		scanStarting = true;
		try {
			const jobId = await startScan(vaultId, currentProfile.id, { tier: 'Tier1' });
			goto(`/scan/progress/${jobId}`);
		} catch (err) {
			console.error('Failed to start scan:', err);
			dashboardError = err instanceof Error ? err.message : 'Failed to start scan';
		} finally {
			scanStarting = false;
		}
	}

	async function handleFullScan() {
		if (!vaultId || !currentProfile) return;
		scanStarting = true;
		try {
			const jobId = await startScan(vaultId, currentProfile.id, { tier: 'All' });
			goto(`/scan/progress/${jobId}`);
		} catch (err) {
			console.error('Failed to start scan:', err);
			dashboardError = err instanceof Error ? err.message : 'Failed to start scan';
		} finally {
			scanStarting = false;
		}
	}
</script>

{#if vaultStore.isCurrentVaultUnlocked}
	<!-- Dashboard Content -->
	<div
		class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 flex items-center justify-center p-4"
	>
		<div class="bg-white rounded-lg shadow-xl p-8 max-w-2xl w-full">
			<h1 class="text-3xl font-bold text-gray-900 mb-4">Spectral Dashboard</h1>
			<p class="text-gray-600 mb-6">Automated data broker removal</p>

			{#if profileStore.loading}
				<div class="flex items-center justify-center py-8">
					<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
				</div>
			{:else if currentProfile}
				<!-- Profile Info -->
				<div class="mb-6 p-4 bg-blue-50 border border-blue-200 rounded-md">
					<h2 class="text-lg font-semibold text-gray-900 mb-2">Your Profile</h2>
					<dl class="space-y-1 text-sm">
						<div>
							<dt class="text-gray-600 inline">Name:</dt>
							<dd class="text-gray-900 inline ml-2">{currentProfile.full_name}</dd>
						</div>
						<div>
							<dt class="text-gray-600 inline">Email:</dt>
							<dd class="text-gray-900 inline ml-2">{currentProfile.email}</dd>
						</div>
					</dl>
				</div>

				<!-- Status Badge -->
				<div
					class="inline-flex items-center px-4 py-2 bg-primary-100 text-primary-700 rounded-full text-sm font-medium mb-4"
					style="background-color: #e0f2fe; color: #0369a1; padding: 0.5rem 1rem; border-radius: 9999px; font-size: 0.875rem; font-weight: 500; display: inline-flex; align-items: center;"
				>
					✓ Vault Unlocked
				</div>

				<!-- First-Run Prompt or Scan Button -->
				{#if isFirstRun}
					<div
						class="bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 rounded-xl p-6 text-center mt-6"
					>
						<h2 class="text-xl font-semibold mb-2">Start your first privacy scan</h2>
						<p class="text-gray-600 dark:text-gray-400 mb-4">
							Check the ~10 most common data brokers for your region.
						</p>
						<div class="flex gap-3 justify-center">
							<button
								onclick={handleFirstRunScan}
								disabled={scanStarting}
								class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
								style="background-color: #0284c7; color: white;"
							>
								{scanStarting ? 'Starting...' : 'Start Tier 1 Scan'}
							</button>
							<button
								onclick={handleFullScan}
								disabled={scanStarting}
								class="px-6 py-3 border border-gray-300 dark:border-gray-600 rounded-lg font-medium hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
							>
								Full Scan (all brokers)
							</button>
						</div>
					</div>
				{:else}
					<div class="mt-6">
						<a
							href="/scan/start"
							class="block px-6 py-4 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors text-center"
							style="background-color: #0284c7; color: white; display: block; text-align: center;"
						>
							Scan for Your Data
						</a>
						<p class="text-sm text-gray-500 mt-2 text-center">
							Search data brokers for your information
						</p>
					</div>
				{/if}

				{#if dashboardError}
					<p class="mt-4 text-sm text-red-500">Failed to load dashboard data.</p>
				{/if}

				{#if dashboard}
					<div class="mt-6 grid grid-cols-1 gap-4 sm:grid-cols-3">
						<!-- Privacy Score Card -->
						<a
							href="/score"
							aria-label="View Privacy Score details"
							class="rounded-lg border border-gray-200 bg-white p-4 hover:border-primary-300 hover:shadow-sm"
						>
							<p class="text-xs font-medium uppercase text-gray-400">Privacy Score</p>
							{#if dashboard.privacy_score !== null}
								<p class="mt-1 text-3xl font-bold text-primary-700">{dashboard.privacy_score}</p>
								<p class="text-xs text-gray-500">
									{dashboard.privacy_score >= 90
										? 'Well Protected'
										: dashboard.privacy_score >= 70
											? 'Good'
											: dashboard.privacy_score >= 40
												? 'Improving'
												: 'At Risk'}
								</p>
							{:else}
								<p class="mt-1 text-sm text-gray-400">No data yet</p>
							{/if}
						</a>

						<!-- Scan Coverage Card -->
						<div class="rounded-lg border border-gray-200 bg-white p-4">
							<p class="text-xs font-medium uppercase text-gray-400">Brokers Scanned</p>
							<p class="mt-1 text-3xl font-bold text-gray-900">{dashboard.brokers_scanned}</p>
							{#if dashboard.last_scan_at}
								<p class="text-xs text-gray-500">
									Last: {new Date(dashboard.last_scan_at).toLocaleDateString()}
								</p>
							{:else}
								<p class="text-xs text-gray-400">Never scanned</p>
							{/if}
						</div>

						<!-- Active Removals Card -->
						<a
							href="/removals"
							aria-label="View Active Removals"
							class="rounded-lg border border-gray-200 bg-white p-4 hover:border-primary-300 hover:shadow-sm"
						>
							<p class="text-xs font-medium uppercase text-gray-400">Active Removals</p>
							<p class="mt-1 text-3xl font-bold text-gray-900">
								{dashboard.active_removals.submitted + dashboard.active_removals.pending}
							</p>
							{#if dashboard.active_removals.failed > 0}
								<p class="text-xs text-red-500">{dashboard.active_removals.failed} failed</p>
							{/if}
						</a>
					</div>

					<!-- Recent Activity -->
					{#if dashboard.recent_events.length > 0}
						<div class="mt-6 rounded-lg border border-gray-200 bg-white">
							<h3 class="border-b border-gray-100 px-4 py-3 text-sm font-medium text-gray-700">
								Recent Activity
							</h3>
							<ul class="divide-y divide-gray-50">
								{#each dashboard.recent_events as event (event.id)}
									<li class="flex items-center gap-3 px-4 py-2.5">
										<time datetime={event.timestamp} class="text-xs text-gray-400"
											>{new Date(event.timestamp).toLocaleDateString()}</time
										>
										<span class="text-sm text-gray-700">{event.description}</span>
									</li>
								{/each}
							</ul>
						</div>
					{/if}
				{/if}
			{:else}
				<!-- No Profile State -->
				<div class="text-center py-8">
					<p class="text-gray-600 mb-4">No profile found</p>
					<button
						onclick={() => goto('/profile/setup')}
						class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors"
						style="background-color: #0284c7; color: white;"
					>
						Create Profile
					</button>
				</div>
			{/if}

			<!-- Lock Button -->
			<div class="mt-6 pt-6 border-t border-gray-200">
				<button
					onclick={() => vaultStore.currentVaultId && vaultStore.lock(vaultStore.currentVaultId)}
					class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 transition-colors"
					style="background-color: #0284c7; color: white; padding: 0.75rem 1.5rem; border-radius: 0.5rem; font-weight: 500;"
				>
					Lock Vault
				</button>
			</div>
		</div>
	</div>
{:else}
	<UnlockScreen />
{/if}
