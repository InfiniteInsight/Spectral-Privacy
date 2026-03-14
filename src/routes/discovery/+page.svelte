<script lang="ts">
	import { onMount } from 'svelte';
	import { vaultStore } from '$lib/stores/vault.svelte';
	import {
		getDiscoveryFindings,
		markFindingRemediated,
		markFindingIgnored,
		startDiscoveryScan,
		pauseDiscoveryScan,
		resumeDiscoveryScan,
		stopDiscoveryScan,
		openFileLocation,
		type DiscoveryFinding
	} from '$lib/api/discovery';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import Spinner from '$lib/components/Spinner.svelte';

	let findings = $state<DiscoveryFinding[]>([]);
	let loading = $state(true);
	let scanning = $state(false);
	let paused = $state(false);
	let error = $state<string | null>(null);
	let successMessage = $state<string | null>(null);
	let allScannedPaths = $state<Array<{ path: string; name: string }>>([]);
	let totalFilesScanned = $state(0); // Actual count from backend

	// Custom directory scanning
	let customDirectories = $state<string[]>([]);
	let newDirectory = $state<string>('');
	let scanMode = $state<'full' | 'custom'>('full');
	let showInfoSection = $state(true);

	// Filter state
	let piiTypeFilter = $state<Set<string>>(new Set());
	let riskLevelFilter = $state<Set<string>>(new Set());
	let showIgnored = $state(false);

	// Filtered findings based on active filters
	const filteredFindings = $derived(
		findings.filter((f) => {
			// Never show remediated findings
			if (f.remediated) return false;

			// Show ignored findings only if toggle is on
			if (!showIgnored && f.ignored) return false;

			// PII type filter (OR logic within group)
			const piiMatch = piiTypeFilter.size === 0 || (f.pii_type && piiTypeFilter.has(f.pii_type));

			// Risk level filter (OR logic within group)
			const riskMatch = riskLevelFilter.size === 0 || riskLevelFilter.has(f.risk_level);

			// Must match ALL filter groups (AND logic between groups)
			return piiMatch && riskMatch;
		})
	);

	// Computed summary counts from filtered findings
	const criticalCount = $derived(
		filteredFindings.filter((f) => f.risk_level === 'critical').length
	);
	const mediumCount = $derived(filteredFindings.filter((f) => f.risk_level === 'medium').length);
	const informationalCount = $derived(
		filteredFindings.filter((f) => f.risk_level === 'low').length
	);

	// Group filtered findings by source
	const filesystemFindings = $derived(filteredFindings.filter((f) => f.source === 'filesystem'));
	const browserFindings = $derived(filteredFindings.filter((f) => f.source === 'browser'));
	const emailFindings = $derived(filteredFindings.filter((f) => f.source === 'email'));

	// Toggle PII type filter
	function togglePiiType(type: string) {
		const newFilter = new Set(piiTypeFilter);
		if (newFilter.has(type)) {
			newFilter.delete(type);
		} else {
			newFilter.add(type);
		}
		piiTypeFilter = newFilter;
	}

	// Toggle risk level filter
	function toggleRiskLevel(level: string) {
		const newFilter = new Set(riskLevelFilter);
		if (newFilter.has(level)) {
			newFilter.delete(level);
		} else {
			newFilter.add(level);
		}
		riskLevelFilter = newFilter;
	}

	// Load findings when vault changes
	async function loadFindings() {
		const vid = vaultStore.currentVaultId;
		if (!vid) {
			loading = false;
			return;
		}

		try {
			loading = true;
			error = null;
			findings = await getDiscoveryFindings(vid, showIgnored);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	// Start scan
	async function startScan() {
		const vid = vaultStore.currentVaultId;
		if (!vid) return;

		try {
			scanning = true;
			error = null;
			allScannedPaths.splice(0, allScannedPaths.length); // Clear in-place to preserve reference for listeners
			totalFilesScanned = 0; // Reset total count

			// Default config: scan all PII types
			const config = {
				scan_emails: true,
				scan_phones: true,
				scan_ssn: true,
				scan_addresses: true,
				scan_names: true,
				scan_dob: true
			};
			await startDiscoveryScan(vid, config);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			scanning = false;
		}
	}

	// Add custom directory
	function addCustomDirectory() {
		if (newDirectory && !customDirectories.includes(newDirectory)) {
			customDirectories = [...customDirectories, newDirectory];
			newDirectory = '';
		}
	}

	// Remove custom directory
	function removeCustomDirectory(dir: string) {
		customDirectories = customDirectories.filter((d) => d !== dir);
	}

	// Mark finding as remediated
	async function markRemediated(findingId: string) {
		const vid = vaultStore.currentVaultId;
		if (!vid) return;

		try {
			await markFindingRemediated(vid, findingId);
			// Reload findings
			await loadFindings();
			// Show success message
			successMessage = 'Finding marked as remediated';
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	// Mark finding as ignored
	async function markIgnored(findingId: string) {
		const vid = vaultStore.currentVaultId;
		if (!vid) return;

		try {
			await markFindingIgnored(vid, findingId, true);
			// Reload findings
			await loadFindings();
			// Show success message
			successMessage = 'Finding marked as ignored (false positive)';
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	// Open the folder containing the file
	async function openFile(filePath: string) {
		try {
			await openFileLocation(filePath);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			console.error('Failed to open file location:', e);
		}
	}

	// Download complete list of scanned files
	function downloadScannedFilesList() {
		const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
		const filename = `scanned-files-${timestamp}.txt`;

		// Create text content
		const content = `PII Discovery Scan - Complete File List
Scan Date: ${new Date().toLocaleString()}
Total Files Scanned: ${allScannedPaths.length}

${allScannedPaths.map((file) => file.path).join('\n')}
`;

		// Create blob and download
		const blob = new Blob([content], { type: 'text/plain' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = filename;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);
	}

	// Pause scan
	async function pauseScan() {
		try {
			await pauseDiscoveryScan();
			paused = true;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			console.error('Failed to pause scan:', e);
		}
	}

	// Resume scan
	async function resumeScan() {
		try {
			await resumeDiscoveryScan();
			paused = false;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			console.error('Failed to resume scan:', e);
		}
	}

	// Stop scan
	async function stopScan() {
		try {
			await stopDiscoveryScan();
			scanning = false;
			paused = false;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
			console.error('Failed to stop scan:', e);
		}
	}

	// Load findings when vault changes (runs on mount and when vault ID changes)
	$effect(() => {
		// Track vault ID to reload findings when it changes
		const vid = vaultStore.currentVaultId;
		if (vid) {
			loadFindings();
		}
	});

	// Set up event listeners once on mount (not in $effect to avoid listener leaks)
	onMount(() => {
		// Store unlisten functions for cleanup
		const unlistenFns: UnlistenFn[] = [];

		// Listen for scan progress
		listen('discovery:progress', (event: any) => {
			// Only update if not paused
			if (!paused) {
				// Update actual count from backend
				totalFilesScanned = event.payload.files_scanned || 0;

				// Track all scanned files for download using push (avoids O(n²) array copies)
				if (event.payload.batch && Array.isArray(event.payload.batch)) {
					for (const file of event.payload.batch) {
						allScannedPaths.push({ path: file.path, name: file.name });
					}
				} else if (event.payload.path || event.payload.directory) {
					const filePath = event.payload.path || event.payload.directory;
					allScannedPaths.push({ path: filePath, name: event.payload.directory });
				}
			}
		}).then((unlisten) => {
			unlistenFns.push(unlisten);
		});

		// Listen for scan completion
		listen('discovery:complete', () => {
			scanning = false;
			paused = false;
			loadFindings();
		}).then((unlisten) => {
			unlistenFns.push(unlisten);
		});

		// Listen for scan stopped
		listen('discovery:stopped', () => {
			scanning = false;
			paused = false;
			loadFindings();
		}).then((unlisten) => {
			unlistenFns.push(unlisten);
		});

		// Clean up all listeners on unmount
		return () => {
			for (const unlisten of unlistenFns) {
				unlisten();
			}
		};
	});

	function riskBadgeClass(level: string): string {
		switch (level) {
			case 'critical':
				return 'bg-red-100 text-red-800';
			case 'medium':
				return 'bg-yellow-100 text-yellow-800';
			case 'informational':
				return 'bg-blue-100 text-blue-800';
			default:
				return 'bg-gray-100 text-gray-800';
		}
	}

	function formatDate(isoDate: string): string {
		try {
			return new Date(isoDate).toLocaleDateString('en-US', {
				year: 'numeric',
				month: 'short',
				day: 'numeric'
			});
		} catch {
			return isoDate;
		}
	}

	function chipClass(isSelected: boolean): string {
		if (isSelected) {
			return 'px-3 py-1 rounded-full text-sm font-medium bg-indigo-600 text-white cursor-pointer hover:bg-indigo-700 transition-colors';
		}
		return 'px-3 py-1 rounded-full text-sm font-medium bg-gray-200 text-gray-700 cursor-pointer hover:bg-gray-300 transition-colors';
	}
</script>

<div class="mx-auto max-w-6xl px-4 py-8">
	<div class="mb-6">
		<h1 class="text-2xl font-bold text-gray-900 mb-4">Local PII Discovery</h1>

		<!-- Info Section -->
		<div class="mb-6 rounded-lg border border-blue-200 bg-blue-50">
			<button
				onclick={() => (showInfoSection = !showInfoSection)}
				class="cursor-pointer w-full flex items-center justify-between p-4 text-left"
			>
				<div class="flex items-center gap-2">
					<span class="text-lg">ℹ️</span>
					<h2 class="text-base font-semibold text-blue-900">
						What is PII and how does Spectral find it?
					</h2>
				</div>
				<span class="text-blue-600 text-xl">{showInfoSection ? '▼' : '▶'}</span>
			</button>

			{#if showInfoSection}
				<div class="px-4 pb-4 text-sm text-blue-800 space-y-3">
					<div>
						<h3 class="font-semibold mb-1">What is Personally Identifiable Information (PII)?</h3>
						<p>
							PII is any information that can be used to identify, contact, or locate you. This
							includes sensitive data like Social Security numbers, email addresses, phone numbers,
							and more. When PII is exposed in files on your computer, it can put your privacy and
							security at risk.
						</p>
					</div>

					<div>
						<h3 class="font-semibold mb-1">How Spectral searches for PII:</h3>
						<ul class="list-disc list-inside space-y-1 ml-2">
							<li>
								<strong>Email Addresses:</strong> Scans for email patterns in documents, logs, configuration
								files, and browser data
							</li>
							<li>
								<strong>Phone Numbers:</strong> Detects US phone numbers in various formats (with/without
								area codes, dashes, parentheses)
							</li>
							<li>
								<strong>Social Security Numbers:</strong> Identifies SSN patterns (XXX-XX-XXXX) in text
								files and documents
							</li>
						</ul>
					</div>

					<div>
						<h3 class="font-semibold mb-1">Where Spectral looks:</h3>
						<p>
							Spectral scans readable text files including documents (.txt, .pdf, .docx),
							configuration files (.json, .xml, .yaml), code files, logs, browser data, and more. It
							skips system directories, caches, and development folders to focus on your personal
							files.
						</p>
					</div>

					<div class="mt-4 p-3 bg-blue-100 rounded-lg">
						<p class="font-medium text-blue-900">🔒 Your privacy is protected:</p>
						<p class="text-blue-700 text-xs mt-1">
							All scanning happens locally on your computer. No data is sent to external servers.
							Findings are encrypted and stored only in your vault.
						</p>
					</div>
				</div>
			{/if}
		</div>

		<!-- Scan Mode Selector -->
		<div class="mb-4 rounded-lg border border-gray-200 bg-white p-4">
			<div class="mb-3 flex gap-4">
				<label class="flex items-center gap-2 cursor-pointer">
					<input
						type="radio"
						name="scanMode"
						value="full"
						checked={scanMode === 'full'}
						onchange={() => (scanMode = 'full')}
						class="cursor-pointer"
					/>
					<span class="text-sm font-medium text-gray-700"
						>Scan entire user profile (%USERPROFILE%)</span
					>
				</label>
				<label class="flex items-center gap-2 cursor-pointer">
					<input
						type="radio"
						name="scanMode"
						value="custom"
						checked={scanMode === 'custom'}
						onchange={() => (scanMode = 'custom')}
						class="cursor-pointer"
					/>
					<span class="text-sm font-medium text-gray-700">Scan custom directories</span>
				</label>
			</div>

			<!-- Custom Directory Input -->
			{#if scanMode === 'custom'}
				<div class="border-t border-gray-200 pt-3">
					<div class="mb-2 flex gap-2">
						<input
							type="text"
							bind:value={newDirectory}
							placeholder="Enter directory path (e.g., C:\MyDocuments)"
							class="flex-1 rounded border border-gray-300 px-3 py-2 text-sm"
							onkeydown={(e) => {
								if (e.key === 'Enter') addCustomDirectory();
							}}
						/>
						<button
							onclick={addCustomDirectory}
							disabled={!newDirectory}
							class="cursor-pointer rounded bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed"
						>
							Add
						</button>
					</div>

					<!-- Custom Directory List -->
					{#if customDirectories.length > 0}
						<div class="space-y-1">
							{#each customDirectories as dir}
								<div class="flex items-center justify-between rounded bg-gray-50 px-3 py-2 text-sm">
									<span class="font-mono text-gray-700">{dir}</span>
									<button
										onclick={() => removeCustomDirectory(dir)}
										class="cursor-pointer text-red-600 hover:text-red-800"
										title="Remove directory"
									>
										✕
									</button>
								</div>
							{/each}
						</div>
					{:else}
						<p class="text-sm text-gray-500 italic">No custom directories added</p>
					{/if}
				</div>
			{/if}
		</div>

		<!-- Scan Control Buttons -->
		<div class="flex justify-end gap-2">
			{#if scanning}
				<!-- Pause/Resume Button -->
				{#if paused}
					<button
						onclick={resumeScan}
						class="cursor-pointer rounded-md bg-green-600 px-6 py-2 text-sm font-medium text-white hover:bg-green-700 min-w-[100px]"
					>
						▶ Resume
					</button>
				{:else}
					<button
						onclick={pauseScan}
						class="cursor-pointer rounded-md bg-yellow-600 px-6 py-2 text-sm font-medium text-white hover:bg-yellow-700 min-w-[100px]"
					>
						⏸ Pause
					</button>
				{/if}

				<!-- Stop Button -->
				<button
					onclick={stopScan}
					class="cursor-pointer rounded-md bg-red-600 px-6 py-2 text-sm font-medium text-white hover:bg-red-700 min-w-[100px]"
				>
					⏹ Stop
				</button>
			{:else}
				<!-- Start Scan Button -->
				<button
					onclick={startScan}
					disabled={loading || (scanMode === 'custom' && customDirectories.length === 0)}
					class="cursor-pointer rounded-md bg-indigo-600 px-6 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed"
				>
					Run PII Discovery Scan
				</button>
			{/if}
		</div>
	</div>

	{#if scanning}
		<div class="mb-6 rounded-lg border border-indigo-200 bg-indigo-50 p-4">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-2">
					{#if paused}
						<div class="rounded-full h-5 w-5 bg-yellow-600"></div>
						<h3 class="text-sm font-semibold text-yellow-900">
							Scan Paused
							{#if totalFilesScanned > 0}
								({totalFilesScanned.toLocaleString()} files scanned so far)
							{/if}
						</h3>
					{:else}
						<Spinner size="sm" color="indigo" inline />
						<h3 class="text-sm font-semibold text-indigo-900">
							Scanning for PII...
							{#if totalFilesScanned > 0}
								({totalFilesScanned.toLocaleString()} files scanned)
							{/if}
						</h3>
					{/if}
				</div>
				<div class="flex gap-2">
					{#if paused}
						<button
							onclick={resumeScan}
							class="px-3 py-1 bg-green-600 text-white text-xs font-medium rounded hover:bg-green-700"
						>
							Resume
						</button>
					{:else}
						<button
							onclick={pauseScan}
							class="px-3 py-1 bg-yellow-600 text-white text-xs font-medium rounded hover:bg-yellow-700"
						>
							Pause
						</button>
					{/if}
					<button
						onclick={stopScan}
						class="px-3 py-1 bg-red-600 text-white text-xs font-medium rounded hover:bg-red-700"
					>
						Cancel
					</button>
				</div>
			</div>
		</div>
	{/if}

	{#if error}
		<div class="mb-4 rounded-md bg-red-50 p-4 text-sm text-red-700">{error}</div>
	{/if}

	{#if successMessage}
		<div class="mb-4 rounded-md bg-green-50 p-4 text-sm text-green-700">{successMessage}</div>
	{/if}

	<!-- Scanned Files Download (shown after scan completes) -->
	{#if !scanning && !loading && allScannedPaths.length > 0}
		<div class="mb-6 rounded-lg border border-indigo-200 bg-indigo-50 p-4">
			<div class="flex items-center justify-between">
				<div>
					<h3 class="text-sm font-semibold text-indigo-900">Scan Complete</h3>
					<p class="text-xs text-indigo-700 mt-1">
						{allScannedPaths.length.toLocaleString()} files scanned
					</p>
				</div>
				<button
					onclick={downloadScannedFilesList}
					class="flex items-center gap-2 rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 transition-colors"
					title="Download complete list of all {allScannedPaths.length.toLocaleString()} scanned files"
				>
					<svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
						/>
					</svg>
					Download File List
				</button>
			</div>
		</div>
	{/if}

	{#if !loading && findings.length > 0}
		<!-- Filter Chips -->
		<div class="mb-6 space-y-3">
			<!-- PII Type Filters -->
			<div class="flex items-center gap-2 flex-wrap">
				<span class="text-sm font-medium text-gray-700">PII Type:</span>
				<button
					onclick={() => togglePiiType('email')}
					class={chipClass(piiTypeFilter.has('email'))}
				>
					Email
				</button>
				<button
					onclick={() => togglePiiType('phone')}
					class={chipClass(piiTypeFilter.has('phone'))}
				>
					Phone
				</button>
				<button onclick={() => togglePiiType('ssn')} class={chipClass(piiTypeFilter.has('ssn'))}>
					SSN
				</button>
			</div>

			<!-- Risk Level Filters -->
			<div class="flex items-center gap-2 flex-wrap">
				<span class="text-sm font-medium text-gray-700">Risk Level:</span>
				<button
					onclick={() => toggleRiskLevel('critical')}
					class={chipClass(riskLevelFilter.has('critical'))}
				>
					Critical
				</button>
				<button
					onclick={() => toggleRiskLevel('medium')}
					class={chipClass(riskLevelFilter.has('medium'))}
				>
					Medium
				</button>
				<button
					onclick={() => toggleRiskLevel('informational')}
					class={chipClass(riskLevelFilter.has('informational'))}
				>
					Informational
				</button>
			</div>

			<!-- Show Ignored Toggle -->
			<div class="flex items-center gap-2">
				<label class="flex items-center gap-2 cursor-pointer">
					<input
						type="checkbox"
						bind:checked={showIgnored}
						onchange={() => loadFindings()}
						class="h-4 w-4 rounded border-gray-300 text-indigo-600 focus:ring-indigo-500"
					/>
					<span class="text-sm font-medium text-gray-700">Show ignored findings</span>
				</label>
			</div>

			<!-- Findings count -->
			<div class="text-sm text-gray-600">
				Showing {filteredFindings.length} of {findings.filter((f) => !f.remediated).length} findings
			</div>
		</div>
	{/if}

	{#if loading}
		<div class="flex flex-col items-center justify-center py-12">
			<Spinner color="indigo" />
			<p class="mt-4 text-sm text-gray-600">Loading previous findings from database...</p>
		</div>
	{:else if findings.length === 0}
		<div class="rounded-md bg-gray-50 p-8 text-center">
			<p class="text-gray-600">
				No findings yet. Click "Run PII Discovery Scan" to scan your local files for PII.
			</p>
		</div>
	{:else}
		<!-- Summary Cards -->
		<div class="mb-6 grid grid-cols-1 gap-4 md:grid-cols-3">
			<div class="rounded-lg border border-red-200 bg-red-50 p-4">
				<div class="text-2xl font-bold text-red-900">{criticalCount}</div>
				<div class="text-sm text-red-700">Critical Issues</div>
			</div>
			<div class="rounded-lg border border-yellow-200 bg-yellow-50 p-4">
				<div class="text-2xl font-bold text-yellow-900">{mediumCount}</div>
				<div class="text-sm text-yellow-700">Medium Risk</div>
			</div>
			<div class="rounded-lg border border-blue-200 bg-blue-50 p-4">
				<div class="text-2xl font-bold text-blue-900">{informationalCount}</div>
				<div class="text-sm text-blue-700">Informational</div>
			</div>
		</div>

		<!-- Empty state for filtered results -->
		{#if findings.length > 0 && filteredFindings.length === 0}
			<div class="rounded-md bg-gray-50 p-8 text-center">
				<p class="text-gray-600">
					No findings match the selected filters. Try adjusting your selection.
				</p>
			</div>
		{/if}

		<!-- Filesystem Findings -->
		{#if filesystemFindings.length > 0}
			<div class="mb-6">
				<h2 class="mb-3 text-lg font-semibold text-gray-900">Filesystem</h2>
				<div class="space-y-3">
					{#each filesystemFindings as finding}
						<div class="rounded-lg border border-gray-200 bg-white p-4">
							<div class="mb-2 flex items-start justify-between">
								<div class="flex-1">
									<div class="mb-1 flex items-center gap-2">
										<span class="text-sm font-medium text-gray-900">{finding.description}</span>
										<span
											class="inline-flex rounded-full px-2 py-1 text-xs font-medium {riskBadgeClass(
												finding.risk_level
											)}"
										>
											{finding.risk_level}
										</span>
										{#if finding.remediated}
											<span
												class="inline-flex rounded-full bg-green-100 px-2 py-1 text-xs font-medium text-green-800"
											>
												Remediated
											</span>
										{/if}
										{#if finding.ignored}
											<span
												class="inline-flex rounded-full bg-gray-100 px-2 py-1 text-xs font-medium text-gray-800"
											>
												Ignored
											</span>
										{/if}
										{#if finding.still_present_after_remediation}
											<span
												class="inline-flex rounded-full bg-orange-100 px-2 py-1 text-xs font-medium text-orange-800"
											>
												⚠️ Still Present
											</span>
										{/if}
									</div>
									{#if finding.still_present_after_remediation}
										<div class="mb-2 rounded-md bg-orange-50 p-2 text-xs text-orange-800">
											<strong>Warning:</strong> This PII was marked as remediated but is still present
											in the file. Please verify the issue has been resolved.
										</div>
									{/if}
									<div class="flex items-center gap-2">
										<div class="text-xs text-gray-500 truncate">{finding.source_detail}</div>
										<button
											onclick={() => openFile(finding.source_detail)}
											class="flex-shrink-0 cursor-pointer rounded-md bg-blue-100 px-2 py-1 text-xs text-blue-700 hover:bg-blue-200"
											title="Open file"
										>
											📂 Open
										</button>
									</div>
									{#if finding.matched_value || finding.line_number}
										<div class="mt-1 rounded-md bg-gray-50 p-2 font-mono text-sm">
											{#if finding.matched_value}
												<div class="text-gray-900">
													<span class="font-semibold">Found:</span>
													{finding.matched_value}
												</div>
											{/if}
											{#if finding.line_number}
												<div class="text-xs text-gray-600">Line {finding.line_number}</div>
											{/if}
										</div>
									{/if}
									{#if finding.recommended_action}
										<div class="mt-2 text-sm text-gray-600">
											<strong>Recommended action:</strong>
											{finding.recommended_action}
										</div>
									{/if}
									<div class="mt-1 text-xs text-gray-400">
										Found {formatDate(finding.found_at)}
									</div>
								</div>
								{#if !finding.remediated}
									<div class="ml-4 flex flex-col gap-2">
										<button
											onclick={() => markRemediated(finding.id)}
											class="cursor-pointer rounded-md bg-green-100 px-3 py-1 text-sm text-green-700 hover:bg-green-200"
										>
											✓ Mark as Fixed
										</button>
										<button
											onclick={() => markIgnored(finding.id)}
											class="cursor-pointer rounded-md bg-gray-100 px-3 py-1 text-sm text-gray-700 hover:bg-gray-200"
										>
											Ignore
										</button>
									</div>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Browser Findings (Stub for Phase 7) -->
		{#if browserFindings.length > 0}
			<div class="mb-6">
				<h2 class="mb-3 text-lg font-semibold text-gray-900">Browser</h2>
				<div class="space-y-3">
					{#each browserFindings as finding}
						<div class="rounded-lg border border-gray-200 bg-white p-4">
							<div class="mb-2 flex items-start justify-between">
								<div class="flex-1">
									<div class="mb-1 flex items-center gap-2">
										<span class="text-sm font-medium text-gray-900">{finding.description}</span>
										<span
											class="inline-flex rounded-full px-2 py-1 text-xs font-medium {riskBadgeClass(
												finding.risk_level
											)}"
										>
											{finding.risk_level}
										</span>
										{#if finding.remediated}
											<span
												class="inline-flex rounded-full bg-green-100 px-2 py-1 text-xs font-medium text-green-800"
											>
												Remediated
											</span>
										{/if}
										{#if finding.ignored}
											<span
												class="inline-flex rounded-full bg-gray-100 px-2 py-1 text-xs font-medium text-gray-800"
											>
												Ignored
											</span>
										{/if}
										{#if finding.still_present_after_remediation}
											<span
												class="inline-flex rounded-full bg-orange-100 px-2 py-1 text-xs font-medium text-orange-800"
											>
												⚠️ Still Present
											</span>
										{/if}
									</div>
									{#if finding.still_present_after_remediation}
										<div class="mb-2 rounded-md bg-orange-50 p-2 text-xs text-orange-800">
											<strong>Warning:</strong> This PII was marked as remediated but is still present
											in the file. Please verify the issue has been resolved.
										</div>
									{/if}
									<div class="flex items-center gap-2">
										<div class="text-xs text-gray-500 truncate">{finding.source_detail}</div>
										<button
											onclick={() => openFile(finding.source_detail)}
											class="flex-shrink-0 cursor-pointer rounded-md bg-blue-100 px-2 py-1 text-xs text-blue-700 hover:bg-blue-200"
											title="Open file"
										>
											📂 Open
										</button>
									</div>
									{#if finding.matched_value || finding.line_number}
										<div class="mt-1 rounded-md bg-gray-50 p-2 font-mono text-sm">
											{#if finding.matched_value}
												<div class="text-gray-900">
													<span class="font-semibold">Found:</span>
													{finding.matched_value}
												</div>
											{/if}
											{#if finding.line_number}
												<div class="text-xs text-gray-600">Line {finding.line_number}</div>
											{/if}
										</div>
									{/if}
									{#if finding.recommended_action}
										<div class="mt-2 text-sm text-gray-600">
											<strong>Recommended action:</strong>
											{finding.recommended_action}
										</div>
									{/if}
									<div class="mt-1 text-xs text-gray-400">
										Found {formatDate(finding.found_at)}
									</div>
								</div>
								{#if !finding.remediated}
									<div class="ml-4 flex flex-col gap-2">
										<button
											onclick={() => markRemediated(finding.id)}
											class="cursor-pointer rounded-md bg-green-100 px-3 py-1 text-sm text-green-700 hover:bg-green-200"
										>
											✓ Mark as Fixed
										</button>
										<button
											onclick={() => markIgnored(finding.id)}
											class="cursor-pointer rounded-md bg-gray-100 px-3 py-1 text-sm text-gray-700 hover:bg-gray-200"
										>
											Ignore
										</button>
									</div>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Email Findings (Stub for Phase 7) -->
		{#if emailFindings.length > 0}
			<div class="mb-6">
				<h2 class="mb-3 text-lg font-semibold text-gray-900">Email</h2>
				<div class="space-y-3">
					{#each emailFindings as finding}
						<div class="rounded-lg border border-gray-200 bg-white p-4">
							<div class="mb-2 flex items-start justify-between">
								<div class="flex-1">
									<div class="mb-1 flex items-center gap-2">
										<span class="text-sm font-medium text-gray-900">{finding.description}</span>
										<span
											class="inline-flex rounded-full px-2 py-1 text-xs font-medium {riskBadgeClass(
												finding.risk_level
											)}"
										>
											{finding.risk_level}
										</span>
										{#if finding.remediated}
											<span
												class="inline-flex rounded-full bg-green-100 px-2 py-1 text-xs font-medium text-green-800"
											>
												Remediated
											</span>
										{/if}
									</div>
									<div class="text-xs text-gray-500">{finding.source_detail}</div>
									{#if finding.recommended_action}
										<div class="mt-2 text-sm text-gray-600">
											<strong>Recommended action:</strong>
											{finding.recommended_action}
										</div>
									{/if}
									<div class="mt-1 text-xs text-gray-400">
										Found {formatDate(finding.found_at)}
									</div>
								</div>
								{#if !finding.remediated}
									<button
										onclick={() => markRemediated(finding.id)}
										class="ml-4 rounded-md bg-gray-100 px-3 py-1 text-sm text-gray-700 hover:bg-gray-200"
									>
										Mark as Remediated
									</button>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>
