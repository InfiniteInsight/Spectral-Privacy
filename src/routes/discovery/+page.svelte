<script lang="ts">
	import { onMount } from 'svelte';
	import { vaultStore } from '$lib/stores/vault.svelte';
	import {
		getDiscoveryFindings,
		markFindingRemediated,
		markFindingIgnored,
		clearDiscoveryResults,
		startDiscoveryScan,
		stopDiscoveryScan,
		pauseDiscoveryScan,
		resumeDiscoveryScan,
		deleteFile,
		openFileLocation,
		getScanLog,
		type DiscoveryFinding,
		type ScanConfig,
		type ScanProgress
	} from '$lib/api/discovery';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import Spinner from '$lib/components/Spinner.svelte';
	import PiiExplainer from '$lib/components/discovery/PiiExplainer.svelte';
	import ScanConfigComponent from '$lib/components/discovery/ScanConfig.svelte';
	import FindingCard from '$lib/components/discovery/FindingCard.svelte';
	import FindingsFilter from '$lib/components/discovery/FindingsFilter.svelte';

	// State
	let findings = $state<DiscoveryFinding[]>([]);
	let loading = $state(true);
	let scanning = $state(false);
	let paused = $state(false);
	let error = $state<string | null>(null);
	let successMessage = $state<string | null>(null);

	// Progress
	let sessionId = $state<string | null>(null);
	let filesScanned = $state(0);
	let filesWithFindings = $state(0);
	let currentDirectory = $state('');

	// Config
	let scanConfig = $state<ScanConfig>({
		scan_emails: true,
		scan_phones: true,
		scan_ssn: true,
		scan_addresses: true,
		scan_names: true,
		scan_dob: true
	});

	// Filters
	let piiTypeFilter = $state<Set<string>>(new Set());
	let riskLevelFilter = $state<Set<string>>(new Set());
	let showIgnored = $state(false);

	// Derived
	const filteredFindings = $derived(
		findings.filter((f) => {
			if (f.remediated && !f.still_present_after_remediation) return false;
			if (!showIgnored && f.ignored) return false;
			const piiMatch = piiTypeFilter.size === 0 || piiTypeFilter.has(f.pii_type);
			const riskMatch = riskLevelFilter.size === 0 || riskLevelFilter.has(f.risk_level);
			return piiMatch && riskMatch;
		})
	);

	const criticalCount = $derived(
		filteredFindings.filter((f) => f.risk_level === 'critical').length
	);
	const highCount = $derived(filteredFindings.filter((f) => f.risk_level === 'high').length);
	const mediumCount = $derived(filteredFindings.filter((f) => f.risk_level === 'medium').length);
	const lowCount = $derived(filteredFindings.filter((f) => f.risk_level === 'low').length);

	const canStartScan = $derived(
		scanConfig.scan_emails ||
			scanConfig.scan_phones ||
			scanConfig.scan_ssn ||
			scanConfig.scan_addresses ||
			scanConfig.scan_names ||
			scanConfig.scan_dob
	);

	// Functions
	async function loadFindings() {
		const vid = vaultStore.currentVaultId;
		if (!vid) {
			loading = false;
			return;
		}
		try {
			loading = true;
			error = null;
			// Always load all findings including ignored ones, filter locally
			findings = await getDiscoveryFindings(vid, true);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	async function handleStartScan() {
		const vid = vaultStore.currentVaultId;
		if (!vid) return;
		try {
			scanning = true;
			paused = false;
			error = null;
			filesScanned = 0;
			filesWithFindings = 0;
			sessionId = await startDiscoveryScan(vid, scanConfig);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			scanning = false;
		}
	}

	async function handleStopScan() {
		try {
			await stopDiscoveryScan();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function handlePauseScan() {
		try {
			await pauseDiscoveryScan();
			paused = true;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function handleResumeScan() {
		try {
			await resumeDiscoveryScan();
			paused = false;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function handleMarkFixed(id: string) {
		const vid = vaultStore.currentVaultId;
		if (!vid) return;
		try {
			await markFindingRemediated(vid, id);
			// Update local state instead of reloading everything
			// Clear still_present_after_remediation when marking fixed again
			findings = findings.map((f) =>
				f.id === id ? { ...f, remediated: true, still_present_after_remediation: false } : f
			);
			showSuccess('Marked as fixed');
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function handleIgnore(id: string) {
		const vid = vaultStore.currentVaultId;
		if (!vid) return;
		try {
			await markFindingIgnored(vid, id, true);
			// Update local state instead of reloading everything
			findings = findings.map((f) => (f.id === id ? { ...f, ignored: true } : f));
			showSuccess('Ignored');
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function handleDelete(id: string, path: string) {
		const vid = vaultStore.currentVaultId;
		if (!vid) return;
		try {
			await deleteFile(path);
			await markFindingRemediated(vid, id);
			// Update local state instead of reloading everything
			findings = findings.map((f) => (f.id === id ? { ...f, remediated: true } : f));
			showSuccess('File deleted');
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function handleOpenLocation(path: string) {
		try {
			await openFileLocation(path);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function handleClearResults() {
		const vid = vaultStore.currentVaultId;
		if (!vid) return;
		if (!confirm('Clear all scan results? This cannot be undone.')) return;
		try {
			await clearDiscoveryResults(vid);
			findings = [];
			sessionId = null;
			filesScanned = 0;
			filesWithFindings = 0;
			showSuccess('Scan results cleared');
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function handleDownloadLog() {
		const vid = vaultStore.currentVaultId;
		if (!vid || !sessionId) return;
		try {
			const log = await getScanLog(vid, sessionId);
			const blob = new Blob([log], { type: 'text/plain' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `scan-log-${sessionId}.txt`;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	function togglePiiType(type: string) {
		const f = new Set(piiTypeFilter);
		f.has(type) ? f.delete(type) : f.add(type);
		piiTypeFilter = f;
	}

	function toggleRiskLevel(level: string) {
		const f = new Set(riskLevelFilter);
		f.has(level) ? f.delete(level) : f.add(level);
		riskLevelFilter = f;
	}

	function showSuccess(msg: string) {
		successMessage = msg;
		setTimeout(() => {
			successMessage = null;
		}, 3000);
	}

	// Effects
	$effect(() => {
		const vid = vaultStore.currentVaultId;
		if (vid) loadFindings();
	});

	onMount(() => {
		const unlistenFns: UnlistenFn[] = [];

		listen<ScanProgress>('discovery:progress', (e) => {
			filesScanned = e.payload.files_scanned;
			filesWithFindings = e.payload.files_with_findings;
			currentDirectory = e.payload.current_directory;
		}).then((u) => unlistenFns.push(u));

		listen('discovery:complete', (e: any) => {
			scanning = false;
			paused = false;
			sessionId = e.payload.session_id;
			loadFindings();
		}).then((u) => unlistenFns.push(u));

		return () => {
			for (const u of unlistenFns) u();
		};
	});
</script>

<div class="mx-auto max-w-6xl px-4 py-8">
	<h1 class="text-2xl font-bold text-gray-900 mb-6">Local PII Discovery</h1>

	<PiiExplainer />

	<div class="mb-6">
		<ScanConfigComponent
			config={scanConfig}
			onConfigChange={(c) => (scanConfig = c)}
			disabled={scanning}
		/>
	</div>

	<div class="mb-6 flex items-center justify-end gap-2">
		{#if scanning}
			{#if paused}
				<button
					onclick={handleResumeScan}
					class="rounded-md bg-green-600 px-4 py-2 text-sm font-medium text-white hover:bg-green-700"
					>Resume</button
				>
			{:else}
				<button
					onclick={handlePauseScan}
					class="rounded-md bg-yellow-600 px-4 py-2 text-sm font-medium text-white hover:bg-yellow-700"
					>Pause</button
				>
			{/if}
			<button
				onclick={handleStopScan}
				class="rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700"
				>Stop</button
			>
		{:else}
			{#if sessionId}
				<button
					onclick={handleDownloadLog}
					class="rounded-md bg-gray-100 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-200"
					>Download Log</button
				>
				<button
					onclick={handleClearResults}
					class="rounded-md bg-gray-100 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-200"
					>Clear Results</button
				>
			{/if}
			<button
				onclick={handleStartScan}
				disabled={loading || !canStartScan}
				class="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50"
				>Run PII Scan</button
			>
		{/if}
	</div>

	{#if scanning}
		<div class="mb-6 rounded-lg border border-indigo-200 bg-indigo-50 p-4">
			<div class="flex items-center gap-3">
				{#if paused}
					<div class="h-5 w-5 rounded-full bg-yellow-500"></div>
					<span class="text-sm font-medium text-yellow-800">Paused</span>
				{:else}
					<Spinner size="sm" color="indigo" inline />
					<span class="text-sm font-medium text-indigo-800">Scanning...</span>
				{/if}
			</div>
			<div class="mt-2 text-sm text-indigo-700">
				{#if currentDirectory}
					<div class="mb-1">{currentDirectory}</div>
				{/if}
				<div>{filesScanned.toLocaleString()} files, {filesWithFindings} findings</div>
			</div>
		</div>
	{:else if sessionId}
		<div class="mb-6 rounded-lg border border-gray-200 bg-gray-50 p-4">
			<div class="text-sm text-gray-700">
				Last scan: {filesScanned.toLocaleString()} files, {filesWithFindings} findings
			</div>
		</div>
	{/if}

	{#if error}
		<div class="mb-4 rounded-md bg-red-50 p-4 text-sm text-red-700">{error}</div>
	{/if}
	{#if successMessage}
		<div class="mb-4 rounded-md bg-green-50 p-4 text-sm text-green-700">{successMessage}</div>
	{/if}

	{#if loading}
		<div class="flex flex-col items-center justify-center py-12">
			<Spinner color="indigo" />
			<p class="mt-4 text-sm text-gray-600">Loading...</p>
		</div>
	{:else if findings.length === 0}
		<div class="rounded-md bg-gray-50 p-8 text-center">
			<p class="text-gray-600">No findings. Click "Run PII Scan" to scan your files.</p>
		</div>
	{:else}
		<FindingsFilter
			{piiTypeFilter}
			{riskLevelFilter}
			{showIgnored}
			onPiiTypeToggle={togglePiiType}
			onRiskLevelToggle={toggleRiskLevel}
			onShowIgnoredChange={(s) => (showIgnored = s)}
			totalCount={findings.filter((f) => !f.remediated || f.still_present_after_remediation).length}
			filteredCount={filteredFindings.length}
		/>

		<div class="mb-6 grid grid-cols-2 md:grid-cols-4 gap-4">
			<div class="rounded-lg border border-red-200 bg-red-50 p-4">
				<div class="text-2xl font-bold text-red-900">{criticalCount}</div>
				<div class="text-sm text-red-700">Critical</div>
			</div>
			<div class="rounded-lg border border-orange-200 bg-orange-50 p-4">
				<div class="text-2xl font-bold text-orange-900">{highCount}</div>
				<div class="text-sm text-orange-700">High</div>
			</div>
			<div class="rounded-lg border border-yellow-200 bg-yellow-50 p-4">
				<div class="text-2xl font-bold text-yellow-900">{mediumCount}</div>
				<div class="text-sm text-yellow-700">Medium</div>
			</div>
			<div class="rounded-lg border border-blue-200 bg-blue-50 p-4">
				<div class="text-2xl font-bold text-blue-900">{lowCount}</div>
				<div class="text-sm text-blue-700">Low</div>
			</div>
		</div>

		{#if filteredFindings.length === 0}
			<div class="rounded-md bg-gray-50 p-8 text-center">
				<p class="text-gray-600">No findings match filters.</p>
			</div>
		{:else}
			<div class="space-y-3">
				{#each filteredFindings as finding (finding.id)}
					<FindingCard
						{finding}
						onMarkFixed={handleMarkFixed}
						onIgnore={handleIgnore}
						onDelete={handleDelete}
						onOpenLocation={handleOpenLocation}
					/>
				{/each}
			</div>
		{/if}
	{/if}
</div>
