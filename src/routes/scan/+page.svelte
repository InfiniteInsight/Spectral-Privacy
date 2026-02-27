<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { vaultStore } from '$lib/stores/vault.svelte';
	import { profileStore } from '$lib/stores/profile.svelte';
	import { scanAPI } from '$lib/api/scan';
	import { cookiesAPI } from '$lib/api/cookies';
	import { startDiscoveryScan } from '$lib/api/discovery';
	import { goto } from '$app/navigation';

	let startingBrokerScan = $state(false);
	let startingCookieScan = $state(false);
	let startingDiscoveryScan = $state(false);
	let error = $state<string | null>(null);

	// Get current profile from store
	const currentProfile = $derived(profileStore.currentProfile);

	// Load profiles and then load the first profile's full data
	$effect(() => {
		const vaultId = vaultStore.currentVaultId;
		if (vaultId) {
			// Load profiles list first, then load full profile data
			profileStore.loadProfiles(vaultId).then(() => {
				if (profileStore.profiles.length > 0) {
					profileStore.loadProfile(vaultId, profileStore.profiles[0].id);
				}
			});
		}
	});

	async function handleStartBrokerScan() {
		if (!vaultStore.currentVaultId || !currentProfile?.id) {
			error = 'Please select a vault and create a profile first';
			return;
		}

		try {
			startingBrokerScan = true;
			error = null;
			await scanAPI.start(vaultStore.currentVaultId, currentProfile.id);
			// Navigate to brokers page where scan progress is shown
			goto('/brokers');
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			startingBrokerScan = false;
		}
	}

	async function handleStartCookieScan() {
		if (!vaultStore.currentVaultId) {
			error = 'Please select a vault first';
			return;
		}

		try {
			startingCookieScan = true;
			error = null;
			console.log('Starting cookie scan for vault:', vaultStore.currentVaultId);

			// Run diagnostics first
			try {
				const diagnostics = await invoke<[string, string, boolean][]>('diagnose_browser_detection');
				console.log('=== BROWSER DETECTION DIAGNOSTICS ===');
				diagnostics.forEach(([name, value, exists]) => {
					console.log(`${name}: ${value} [${exists ? 'EXISTS' : 'NOT FOUND'}]`);
				});
				console.log('=== END DIAGNOSTICS ===');
			} catch (diagErr) {
				console.error('Diagnostics failed:', diagErr);
			}

			const result = await cookiesAPI.scanCookies(vaultStore.currentVaultId);
			console.log('Cookie scan completed:', result);
			// Navigate to cookies results page
			goto('/cookies');
		} catch (err) {
			console.error('Cookie scan error:', err);
			error = err instanceof Error ? err.message : String(err);
		} finally {
			startingCookieScan = false;
		}
	}

	async function handleStartDiscoveryScan() {
		if (!vaultStore.currentVaultId) {
			error = 'Please select a vault first';
			return;
		}

		try {
			startingDiscoveryScan = true;
			error = null;
			await startDiscoveryScan(vaultStore.currentVaultId);
			// Navigate to discovery page where results are shown
			goto('/discovery');
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			startingDiscoveryScan = false;
		}
	}
</script>

<div class="mx-auto max-w-4xl px-4 py-8">
	<div class="mb-6">
		<h1 class="mb-2 text-2xl font-bold text-gray-900">Scan Center</h1>
		<p class="text-gray-600">
			Start a scan to find your personal information online, discover tracking cookies, or detect
			local PII exposures.
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
				Create or unlock a vault to start scanning. <a href="/people" class="underline"
					>Go to People page</a
				>
			</p>
		</div>
	{:else}
		<!-- Scan Type Cards -->
		<div class="space-y-4">
			<!-- Broker Scan Card -->
			<div class="rounded-lg border border-gray-200 bg-white p-6">
				<div class="mb-4">
					<h3 class="mb-2 text-lg font-semibold text-gray-900">Data Broker Scan</h3>
					<p class="text-sm text-gray-600">
						Search data broker websites for your personal information. Requires a profile with your
						details (name, age, location).
					</p>
				</div>

				<div class="mb-4 rounded-lg bg-gray-50 p-3">
					<h4 class="mb-2 text-sm font-medium text-gray-700">What this scan does:</h4>
					<ul class="space-y-1 text-sm text-gray-600">
						<li>• Searches 100+ data broker websites</li>
						<li>• Finds listings with your name, age, and location</li>
						<li>• Extracts phone numbers, addresses, and relatives</li>
						<li>• Generates removal request URLs</li>
					</ul>
				</div>

				{#if !currentProfile}
					<div
						class="mb-3 rounded border border-yellow-200 bg-yellow-50 p-3 text-sm text-yellow-700"
					>
						You need to create a profile before scanning data brokers.
						<a href="/people" class="underline">Create profile</a>
					</div>
				{/if}

				<button
					onclick={handleStartBrokerScan}
					disabled={startingBrokerScan || !currentProfile}
					class="w-full rounded-lg bg-indigo-600 px-4 py-3 text-sm font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-50"
				>
					{#if startingBrokerScan}
						<span class="flex items-center justify-center gap-2">
							<div
								class="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent"
							></div>
							Starting Broker Scan...
						</span>
					{:else}
						Start Broker Scan
					{/if}
				</button>
			</div>

			<!-- Cookie Scan Card -->
			<div class="rounded-lg border border-gray-200 bg-white p-6">
				<div class="mb-4">
					<h3 class="mb-2 text-lg font-semibold text-gray-900">Cookie Scanner</h3>
					<p class="text-sm text-gray-600">
						Scan your installed browsers for tracking cookies from data brokers and advertising
						companies.
					</p>
				</div>

				<div class="mb-4 rounded-lg bg-gray-50 p-3">
					<h4 class="mb-2 text-sm font-medium text-gray-700">What this scan does:</h4>
					<ul class="space-y-1 text-sm text-gray-600">
						<li>• Scans Chrome, Firefox, Edge, and other browsers</li>
						<li>• Identifies tracking cookies from known brokers</li>
						<li>• Shows which brokers are tracking you</li>
						<li>• Allows bulk removal of tracking cookies</li>
					</ul>
				</div>

				<button
					onclick={handleStartCookieScan}
					disabled={startingCookieScan}
					class="w-full rounded-lg bg-indigo-600 px-4 py-3 text-sm font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-50"
				>
					{#if startingCookieScan}
						<span class="flex items-center justify-center gap-2">
							<div
								class="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent"
							></div>
							Scanning Cookies...
						</span>
					{:else}
						Start Cookie Scan
					{/if}
				</button>
			</div>

			<!-- Discovery Scan Card -->
			<div class="rounded-lg border border-gray-200 bg-white p-6">
				<div class="mb-4">
					<h3 class="mb-2 text-lg font-semibold text-gray-900">Local PII Discovery</h3>
					<p class="text-sm text-gray-600">
						Scan your local files for exposed personal information like emails, phone numbers, and
						Social Security numbers.
					</p>
				</div>

				<div class="mb-4 rounded-lg bg-gray-50 p-3">
					<h4 class="mb-2 text-sm font-medium text-gray-700">What this scan does:</h4>
					<ul class="space-y-1 text-sm text-gray-600">
						<li>• Scans Documents, Downloads, and Desktop folders</li>
						<li>• Detects emails, phone numbers, and SSNs</li>
						<li>• Identifies plaintext credential files</li>
						<li>• Provides remediation recommendations</li>
					</ul>
				</div>

				<button
					onclick={handleStartDiscoveryScan}
					disabled={startingDiscoveryScan}
					class="w-full rounded-lg bg-indigo-600 px-4 py-3 text-sm font-medium text-white hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-50"
				>
					{#if startingDiscoveryScan}
						<span class="flex items-center justify-center gap-2">
							<div
								class="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent"
							></div>
							Scanning Files...
						</span>
					{:else}
						Start Discovery Scan
					{/if}
				</button>
			</div>
		</div>

		<div class="mt-6 rounded-lg border border-gray-200 bg-gray-50 p-4">
			<h4 class="mb-2 font-medium text-gray-900">Need Help?</h4>
			<ul class="space-y-1 text-sm text-gray-600">
				<li>
					• View previous scan results in <a href="/scan-history" class="underline">Scan History</a>
				</li>
				<li>
					• Monitor removal requests in <a href="/removals" class="underline">Removal Status</a>
				</li>
				<li>
					• Check your privacy score in <a href="/score" class="underline">Score</a>
				</li>
			</ul>
		</div>
	{/if}
</div>
