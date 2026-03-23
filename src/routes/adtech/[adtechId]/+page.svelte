<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { onDestroy } from 'svelte';
	import { brokerAPI, type BrokerDetail } from '$lib/api/brokers';
	import { vaultStore, profileStore } from '$lib/stores';
	import { startScan, scanAPI, type ScanJobStatus } from '$lib/api/scan';
	import { getDifficultyColor, getCategoryDisplay } from '$lib/utils/broker';
	import {
		getRemovalMethodDisplay,
		getScanStatusDisplay,
		formatDate
	} from '$lib/utils/broker-display';
	import EmailTemplatePreview from '$lib/components/broker/EmailTemplatePreview.svelte';
	import EmailFallbackDisplay from '$lib/components/broker/EmailFallbackDisplay.svelte';

	const adtechId = $derived($page.params.adtechId);

	let adtech = $state<BrokerDetail | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Inline targeted scan state
	let scanStatus = $state<ScanJobStatus | null>(null);
	let scanStarting = $state(false);
	let scanError = $state<string | null>(null);
	let pollingInterval: ReturnType<typeof setInterval> | null = null;

	onDestroy(() => {
		if (pollingInterval !== null) clearInterval(pollingInterval);
	});

	// Load adtech detail using $effect
	$effect(() => {
		async function loadAdtechDetail() {
			if (!adtechId) {
				error = 'No company ID provided';
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
				adtech = await brokerAPI.getBrokerDetail(adtechId, vaultStore.currentVaultId);
			} catch (err) {
				error = 'Failed to load company details. Please try again.';
				console.error('Failed to load adtech detail:', err);
			} finally {
				loading = false;
			}
		}

		loadAdtechDetail();
	});

	const scanProgressPercent = $derived(
		scanStatus && scanStatus.total_brokers > 0
			? Math.round((scanStatus.completed_brokers / scanStatus.total_brokers) * 100)
			: 0
	);

	const scanIsComplete = $derived(scanStatus?.status === 'Completed');
	const scanIsFailed = $derived(scanStatus?.status === 'Failed');
	const scanInProgress = $derived(scanStatus?.status === 'InProgress');

	async function handleTargetedScan() {
		const vaultId = vaultStore.currentVaultId;
		if (!vaultId || !adtechId) return;

		scanStarting = true;
		scanError = null;
		scanStatus = null;

		try {
			await profileStore.loadProfiles(vaultId);
			if (profileStore.profiles.length === 0) {
				scanError = 'No profile found. Please set up a profile first.';
				return;
			}
			const profileId = profileStore.profiles[0].id;
			const brokerId = adtechId;

			console.log(`[scan] Starting targeted scan for broker: ${brokerId} (profile: ${profileId})`);

			const jobId = await startScan(vaultId, profileId, { brokerIds: [brokerId] });
			console.log(`[scan] Job created: ${jobId}`);

			// Fetch initial status
			scanStatus = await scanAPI.getStatus(vaultId, jobId);
			console.log(`[scan] Initial status:`, scanStatus);

			// Poll for updates
			pollingInterval = setInterval(async () => {
				try {
					const status = await scanAPI.getStatus(vaultId, jobId);
					scanStatus = status;

					const brokerLabel = status.current_broker_name ?? brokerId;
					console.log(
						`[scan] Poll: ${status.status} — ${status.completed_brokers}/${status.total_brokers} brokers` +
							(status.current_broker_name ? ` (scanning: ${brokerLabel})` : '')
					);

					if (
						status.status === 'Completed' ||
						status.status === 'Failed' ||
						status.status === 'Cancelled'
					) {
						clearInterval(pollingInterval!);
						pollingInterval = null;

						if (status.status === 'Completed') {
							console.log(`[scan] Completed — reloading broker detail for updated findings count`);
							adtech = await brokerAPI.getBrokerDetail(brokerId, vaultId);
							console.log(`[scan] Broker finding_count now: ${adtech?.finding_count ?? 'unknown'}`);
						} else {
							console.warn(
								`[scan] Ended with status: ${status.status}`,
								status.error_message ?? ''
							);
						}
					}
				} catch (err) {
					console.error('[scan] Polling error:', err);
				}
			}, 2000);
		} catch (err) {
			console.error('[scan] Failed to start:', err);
			scanError = err instanceof Error ? err.message : String(err);
		} finally {
			scanStarting = false;
		}
	}
</script>

<div class="min-h-screen bg-gradient-to-br from-orange-50 to-orange-100 p-4">
	<div class="max-w-4xl mx-auto">
		<div class="bg-white rounded-lg shadow-xl p-8">
			<!-- Back Button -->
			<button
				onclick={() => goto('/adtech')}
				class="mb-6 text-gray-600 hover:text-gray-900 transition-colors flex items-center gap-2"
			>
				← Back to AdTech List
			</button>

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
			{:else if adtech}
				<!-- AdTech Details -->
				<div>
					<!-- Header -->
					<div class="mb-8">
						<h1 class="text-3xl font-bold text-gray-900 mb-2">{adtech.name}</h1>
						<a
							href={adtech.url}
							target="_blank"
							rel="noopener noreferrer"
							class="text-orange-600 hover:text-orange-700 hover:underline"
						>
							{adtech.domain} ↗
						</a>
					</div>

					<!-- Key Information Grid -->
					<div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
						<!-- Category -->
						<div class="p-4 bg-gray-50 rounded-lg">
							<h3 class="text-sm font-medium text-gray-700 mb-1">Category</h3>
							<p class="text-lg font-semibold text-gray-900">
								{getCategoryDisplay(adtech.category)}
							</p>
						</div>

						<!-- Difficulty -->
						<div class="p-4 bg-gray-50 rounded-lg">
							<h3 class="text-sm font-medium text-gray-700 mb-1">Removal Difficulty</h3>
							<span
								class="inline-block px-3 py-1 rounded text-sm font-medium {getDifficultyColor(
									adtech.difficulty
								)}"
							>
								{adtech.difficulty}
							</span>
						</div>

						<!-- Removal Method -->
						<div class="p-4 bg-gray-50 rounded-lg">
							<h3 class="text-sm font-medium text-gray-700 mb-1">Removal Method</h3>
							<p class="text-lg font-semibold text-gray-900">
								{getRemovalMethodDisplay(adtech.removal_method)}
							</p>
						</div>

						<!-- Typical Removal Time -->
						<div class="p-4 bg-gray-50 rounded-lg">
							<h3 class="text-sm font-medium text-gray-700 mb-1">Typical Removal Time</h3>
							<p class="text-lg font-semibold text-gray-900">
								{adtech.typical_removal_days}
								{adtech.typical_removal_days === 1 ? 'day' : 'days'}
							</p>
						</div>

						<!-- Recheck Interval -->
						<div class="p-4 bg-gray-50 rounded-lg">
							<h3 class="text-sm font-medium text-gray-700 mb-1">Recheck Interval</h3>
							<p class="text-lg font-semibold text-gray-900">
								Every {adtech.recheck_interval_days}
								{adtech.recheck_interval_days === 1 ? 'day' : 'days'}
							</p>
						</div>

						<!-- Last Verified -->
						<div class="p-4 bg-gray-50 rounded-lg">
							<h3 class="text-sm font-medium text-gray-700 mb-1">Last Verified</h3>
							<p class="text-lg font-semibold text-gray-900">{formatDate(adtech.last_verified)}</p>
						</div>
					</div>

					<!-- Your Scan Status -->
					{#if adtech.scan_status}
						{@const statusDisplay = getScanStatusDisplay(adtech.scan_status)}
						<div class="mb-8 p-6 border-2 border-gray-200 rounded-lg">
							<h2 class="text-xl font-bold text-gray-900 mb-4">Your Scan Status</h2>
							<div class="flex items-center justify-between">
								<div>
									<p class="text-sm text-gray-600 mb-2">Status on this company</p>
									<span
										class="inline-block px-3 py-1 rounded text-sm font-medium {statusDisplay.color}"
									>
										{statusDisplay.text}
									</span>
								</div>
								{#if adtech.finding_count !== null && adtech.finding_count > 0}
									<div class="text-right">
										<p class="text-3xl font-bold text-red-600">{adtech.finding_count}</p>
										<p class="text-sm text-gray-600">
											{adtech.finding_count === 1 ? 'finding' : 'findings'}
										</p>
									</div>
								{/if}
							</div>
						</div>
					{:else}
						<div class="mb-8 p-6 bg-blue-50 border border-blue-200 rounded-lg">
							<h2 class="text-xl font-bold text-gray-900 mb-2">No Scan Data</h2>
							<p class="text-sm text-gray-700">
								You haven't scanned this company yet. Use the buttons below to check if your
								information appears on this site.
							</p>
						</div>
					{/if}

					<!-- Zendesk Warning -->
					{#if adtech.id === 'zendesk'}
						<div class="mb-8 p-6 bg-yellow-50 border-2 border-yellow-400 rounded-lg">
							<h2 class="text-xl font-bold text-yellow-900 mb-2 flex items-center gap-2">
								<span class="text-2xl">⚠️</span>
								Important Warning
							</h2>
							<p class="text-sm text-yellow-900 font-medium mb-2">
								Requesting deletion from Zendesk may affect your support tickets with companies that
								use Zendesk.
							</p>
							<p class="text-sm text-yellow-800">
								If you have open support tickets with any companies (e.g., customer support, help
								desk tickets), requesting data deletion from Zendesk could cancel or close those
								tickets. Consider resolving your open support issues before proceeding with this
								removal request.
							</p>
						</div>
					{/if}

					<!-- Email Template Preview -->
					{#if adtech.email_template}
						<EmailTemplatePreview template={adtech.email_template} />
					{/if}

					<!-- Email Fallback Section -->
					{#if adtech.email_fallback && adtech.email_fallback.enabled}
						<EmailFallbackDisplay emailFallback={adtech.email_fallback} />
					{/if}

					<!-- Inline Scan Progress -->
					{#if scanStatus || scanStarting}
						<div class="mb-6 p-6 border-2 border-orange-200 rounded-lg bg-orange-50">
							<h3 class="text-lg font-semibold text-gray-900 mb-3">
								{#if scanIsComplete}
									Scan Complete
								{:else if scanIsFailed}
									Scan Failed
								{:else}
									Scanning {adtech.name}...
								{/if}
							</h3>

							{#if scanInProgress || scanStarting}
								<div class="mb-3">
									<div class="flex items-center justify-between mb-1">
										<span class="text-sm text-gray-600">
											{#if scanStatus}
												{scanStatus.completed_brokers} of {scanStatus.total_brokers} brokers
											{:else}
												Starting scan...
											{/if}
										</span>
										<span class="text-sm font-medium text-gray-700">{scanProgressPercent}%</span>
									</div>
									<div class="w-full bg-gray-200 rounded-full h-2">
										<div
											class="h-2 rounded-full transition-all duration-500 bg-orange-500"
											style="width: {scanProgressPercent}%"
										></div>
									</div>
								</div>
								<div class="flex items-center gap-2 text-sm text-gray-600">
									<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-orange-600"></div>
									{scanStatus?.current_broker_name
										? `Checking ${scanStatus.current_broker_name}...`
										: 'Initializing...'}
								</div>
							{:else if scanIsComplete}
								<p class="text-sm text-green-700">
									✓ Done — scan status and findings above have been updated.
								</p>
							{:else if scanIsFailed}
								<p class="text-sm text-red-700">
									✗ {scanStatus?.error_message || 'Scan failed. Please try again.'}
								</p>
							{/if}
						</div>
					{/if}

					{#if scanError}
						<div class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg">
							<p class="text-sm text-red-700">{scanError}</p>
						</div>
					{/if}

					<!-- Action Buttons -->
					<div class="flex flex-col sm:flex-row gap-4">
						<a
							href={adtech.url}
							target="_blank"
							rel="noopener noreferrer"
							class="flex-1 px-6 py-3 bg-orange-600 text-white rounded-lg font-medium hover:bg-orange-700 transition-colors text-center"
						>
							Visit Company Website ↗
						</a>
						<button
							onclick={handleTargetedScan}
							disabled={scanStarting || scanInProgress}
							class="flex-1 px-6 py-3 border-2 border-orange-600 text-orange-700 rounded-lg font-medium hover:bg-orange-50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
						>
							{scanStarting || scanInProgress ? 'Scanning...' : 'Scan This Company'}
						</button>
						<button
							onclick={() => goto('/scan')}
							class="flex-1 px-6 py-3 border border-gray-300 text-gray-700 rounded-lg font-medium hover:bg-gray-50 transition-colors"
						>
							Full Scan Center
						</button>
					</div>
				</div>
			{:else}
				<!-- No Data -->
				<div class="text-center py-12">
					<p class="text-gray-600">Company not found</p>
				</div>
			{/if}
		</div>
	</div>
</div>
