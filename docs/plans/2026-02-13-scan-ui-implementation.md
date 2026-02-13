# Phase 3: Scan UI Components Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Build user-facing UI for scan workflow: initiate scans, track progress, review findings, queue removals.

**Architecture:** Multi-page SvelteKit app with route-based navigation (`/scan/start`, `/scan/progress/[id]`, `/scan/review/[id]`, `/removals`), Svelte stores for state management, polling for real-time updates, reusable components following ProfileWizard patterns.

**Tech Stack:** SvelteKit, TypeScript, Tailwind CSS, Tauri API integration

---

## Task Tracking

- [ ] Task 1: Create scan store
- [ ] Task 2: Create scan start page
- [ ] Task 3: Create progress page with polling
- [ ] Task 4: Create review page with grouped findings
- [ ] Task 5: Create removals placeholder page
- [ ] Task 6: Add dashboard integration
- [ ] Task 7: Create reusable components

---

## Task 1: Create Scan Store

**Files:**
- Create: `src/lib/stores/scan.ts`
- Create: `src/lib/stores/index.ts` (or modify if exists)

**Goal:** Centralized state management for scan workflow

**Step 1: Create scan store file**

Create `src/lib/stores/scan.ts`:

```typescript
import { scanAPI, type ScanJobStatus, type Finding } from '$lib/api/scan';

interface ScanState {
	currentScanId: string | null;
	scanStatus: ScanJobStatus | null;
	findings: Finding[];
	loading: boolean;
	error: string | null;
	pollingInterval: number | null;
}

function createScanStore() {
	let state = $state<ScanState>({
		currentScanId: null,
		scanStatus: null,
		findings: [],
		loading: false,
		error: null,
		pollingInterval: null
	});

	return {
		get currentScanId() {
			return state.currentScanId;
		},
		get scanStatus() {
			return state.scanStatus;
		},
		get findings() {
			return state.findings;
		},
		get loading() {
			return state.loading;
		},
		get error() {
			return state.error;
		},

		async startScan(profileId: string): Promise<string | null> {
			state.loading = true;
			state.error = null;

			try {
				const result = await scanAPI.start(profileId);
				state.currentScanId = result.id;
				state.scanStatus = result;
				return result.id;
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to start scan';
				return null;
			} finally {
				state.loading = false;
			}
		},

		async fetchStatus(scanJobId: string): Promise<void> {
			try {
				const status = await scanAPI.getStatus(scanJobId);
				state.scanStatus = status;
				state.currentScanId = scanJobId;
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to fetch status';
			}
		},

		startPolling(scanJobId: string, intervalMs: number = 2000): void {
			// Clear any existing interval
			this.stopPolling();

			// Initial fetch
			this.fetchStatus(scanJobId);

			// Set up polling
			state.pollingInterval = window.setInterval(() => {
				this.fetchStatus(scanJobId);

				// Auto-stop polling on terminal status
				if (
					state.scanStatus &&
					['Completed', 'Failed', 'Cancelled'].includes(state.scanStatus.status)
				) {
					this.stopPolling();
				}
			}, intervalMs);
		},

		stopPolling(): void {
			if (state.pollingInterval !== null) {
				clearInterval(state.pollingInterval);
				state.pollingInterval = null;
			}
		},

		async loadFindings(scanJobId: string, filter?: 'PendingVerification' | 'Confirmed' | 'Rejected'): Promise<void> {
			state.loading = true;
			state.error = null;

			try {
				const findings = await scanAPI.getFindings(scanJobId, filter);
				state.findings = findings;
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to load findings';
			} finally {
				state.loading = false;
			}
		},

		async verifyFinding(findingId: string, isMatch: boolean): Promise<void> {
			try {
				await scanAPI.verify(findingId, isMatch);

				// Update local state optimistically
				const finding = state.findings.find(f => f.id === findingId);
				if (finding) {
					finding.verification_status = isMatch ? 'Confirmed' : 'Rejected';
				}
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to verify finding';
				throw err;
			}
		},

		async submitRemovals(scanJobId: string): Promise<number> {
			state.loading = true;
			state.error = null;

			try {
				const removalIds = await scanAPI.submitRemovals(scanJobId);
				return removalIds.length;
			} catch (err) {
				state.error = err instanceof Error ? err.message : 'Failed to submit removals';
				throw err;
			} finally {
				state.loading = false;
			}
		},

		reset(): void {
			this.stopPolling();
			state.currentScanId = null;
			state.scanStatus = null;
			state.findings = [];
			state.loading = false;
			state.error = null;
		}
	};
}

export const scanStore = createScanStore();
```

**Step 2: Export from stores index**

Modify or create `src/lib/stores/index.ts`:

```typescript
export { vaultStore } from './vault';
export { profileStore } from './profile';
export { scanStore } from './scan';
```

**Step 3: Commit**

```bash
git add src/lib/stores/scan.ts src/lib/stores/index.ts
git commit -m "feat(stores): add scan store for state management

- Scan workflow state (scanId, status, findings)
- Actions: start, poll, loadFindings, verify, submitRemovals
- Polling with auto-stop on terminal status
- Optimistic updates for verify actions"
```

---

## Task 2: Create Scan Start Page

**Files:**
- Create: `src/routes/scan/start/+page.svelte`

**Goal:** Page to initiate new scan jobs

**Step 1: Create scan start page**

Create `src/routes/scan/start/+page.svelte`:

```svelte
<script lang="ts">
	import { scanStore, profileStore, vaultStore } from '$lib/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	let selectedProfileId = $state('');
	let error = $state('');

	onMount(async () => {
		// Ensure profiles are loaded
		if (profileStore.profiles.length === 0) {
			await profileStore.loadProfiles();
		}

		// Pre-select first profile
		if (profileStore.profiles.length > 0) {
			selectedProfileId = profileStore.profiles[0].id;
		}
	});

	async function handleStartScan() {
		if (!selectedProfileId) {
			error = 'Please select a profile';
			return;
		}

		const scanId = await scanStore.startScan(selectedProfileId);

		if (scanId) {
			goto(`/scan/progress/${scanId}`);
		} else {
			error = scanStore.error || 'Failed to start scan';
		}
	}

	const profiles = $derived(profileStore.profiles);
	const hasProfiles = $derived(profiles.length > 0);
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 flex items-center justify-center p-4">
	<div class="bg-white rounded-lg shadow-xl p-8 max-w-2xl w-full">
		<h1 class="text-3xl font-bold text-gray-900 mb-2">Scan for Your Data</h1>
		<p class="text-gray-600 mb-8">
			Search data brokers to find where your personal information appears online
		</p>

		{#if profileStore.loading}
			<div class="flex items-center justify-center py-8">
				<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
			</div>
		{:else if !hasProfiles}
			<div class="text-center py-8">
				<p class="text-gray-600 mb-4">No profile found. Create a profile first to start scanning.</p>
				<button
					onclick={() => goto('/profile/setup')}
					class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors"
					style="background-color: #0284c7; color: white;"
				>
					Create Profile
				</button>
			</div>
		{:else}
			<!-- Profile Selection -->
			{#if profiles.length > 1}
				<div class="mb-6">
					<label for="profile" class="block text-sm font-medium text-gray-700 mb-2">
						Select Profile
					</label>
					<select
						id="profile"
						bind:value={selectedProfileId}
						class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
					>
						{#each profiles as profile}
							<option value={profile.id}>{profile.full_name}</option>
						{/each}
					</select>
				</div>
			{:else}
				<div class="mb-6 p-4 bg-blue-50 border border-blue-200 rounded-lg">
					<p class="text-sm text-gray-700">
						<strong>Profile:</strong> {profiles[0].full_name}
					</p>
				</div>
			{/if}

			<!-- Info Box -->
			<div class="mb-6 p-4 bg-gray-50 border border-gray-200 rounded-lg">
				<h3 class="text-sm font-semibold text-gray-900 mb-2">What happens next:</h3>
				<ul class="text-sm text-gray-600 space-y-1">
					<li>• We'll search multiple data broker sites for your information</li>
					<li>• This typically takes 2-5 minutes</li>
					<li>• You'll review results before any removal requests are sent</li>
				</ul>
			</div>

			<!-- Error Display -->
			{#if error}
				<div class="mb-4 p-4 bg-red-50 border border-red-200 rounded-lg">
					<p class="text-sm text-red-700">{error}</p>
				</div>
			{/if}

			<!-- Start Button -->
			<button
				onclick={handleStartScan}
				disabled={scanStore.loading || !selectedProfileId}
				class="w-full px-6 py-4 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-lg"
				style="background-color: #0284c7; color: white;"
			>
				{scanStore.loading ? 'Starting Scan...' : 'Start Scan'}
			</button>

			<!-- Back to Dashboard -->
			<div class="mt-4 text-center">
				<button
					onclick={() => goto('/')}
					class="text-sm text-gray-600 hover:text-gray-900 transition-colors"
				>
					← Back to Dashboard
				</button>
			</div>
		{/if}
	</div>
</div>
```

**Step 2: Commit**

```bash
git add src/routes/scan/start/+page.svelte
git commit -m "feat(scan): add scan start page

- Profile selection (dropdown if multiple profiles)
- Info box explaining scan process
- Start scan button with loading state
- Error handling and display
- Redirect to profile setup if no profiles exist"
```

---

## Task 3: Create Progress Page with Polling

**Files:**
- Create: `src/routes/scan/progress/[id]/+page.svelte`

**Goal:** Real-time scan progress tracking with broker-by-broker status

**Step 1: Create progress page**

Create `src/routes/scan/progress/[id]/+page.svelte`:

```svelte
<script lang="ts">
	import { scanStore } from '$lib/stores';
	import { goto } from '$app/navigation';
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';

	const scanJobId = $derived($page.params.id);

	onMount(() => {
		if (!scanJobId) {
			goto('/scan/start');
			return;
		}

		// Start polling
		scanStore.startPolling(scanJobId);
	});

	onDestroy(() => {
		// Stop polling when leaving page
		scanStore.stopPolling();
	});

	// Auto-navigate when complete
	$effect(() => {
		if (scanStore.scanStatus?.status === 'Completed') {
			// Wait 2 seconds then navigate
			setTimeout(() => {
				goto(`/scan/review/${scanJobId}`);
			}, 2000);
		}
	});

	const status = $derived(scanStore.scanStatus);
	const isComplete = $derived(status?.status === 'Completed');
	const isFailed = $derived(status?.status === 'Failed');
	const isInProgress = $derived(status?.status === 'InProgress');

	// Calculate progress percentage
	const progressPercent = $derived(
		status ? Math.round((status.completed_brokers / status.total_brokers) * 100) : 0
	);

	function getStatusColor(brokerStatus: string): string {
		switch (brokerStatus) {
			case 'Pending':
				return 'bg-gray-200 text-gray-700';
			case 'InProgress':
				return 'bg-blue-100 text-blue-700';
			case 'Success':
				return 'bg-green-100 text-green-700';
			case 'Failed':
				return 'bg-red-100 text-red-700';
			case 'Skipped':
				return 'bg-yellow-100 text-yellow-700';
			default:
				return 'bg-gray-100 text-gray-700';
		}
	}

	function getStatusIcon(brokerStatus: string): string {
		switch (brokerStatus) {
			case 'Success':
				return '✓';
			case 'Failed':
				return '✗';
			case 'Skipped':
				return '⊘';
			case 'InProgress':
				return '⟳';
			default:
				return '·';
		}
	}
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 p-4">
	<div class="max-w-4xl mx-auto">
		<div class="bg-white rounded-lg shadow-xl p-8">
			<!-- Header -->
			<div class="mb-8">
				<h1 class="text-3xl font-bold text-gray-900 mb-2">
					{#if isComplete}
						Scan Complete!
					{:else if isFailed}
						Scan Failed
					{:else}
						Scanning Data Brokers...
					{/if}
				</h1>
				{#if status}
					<p class="text-gray-600">
						{#if isComplete}
							Found results from {status.completed_brokers} of {status.total_brokers} brokers
						{:else}
							Checking {status.completed_brokers} of {status.total_brokers} brokers
						{/if}
					</p>
				{/if}
			</div>

			<!-- Progress Bar -->
			{#if status}
				<div class="mb-8">
					<div class="flex items-center justify-between mb-2">
						<span class="text-sm font-medium text-gray-700">Progress</span>
						<span class="text-sm font-medium text-gray-700">{progressPercent}%</span>
					</div>
					<div class="w-full bg-gray-200 rounded-full h-3">
						<div
							class="bg-primary-600 h-3 rounded-full transition-all duration-500"
							style="width: {progressPercent}%; background-color: #0284c7;"
						></div>
					</div>
				</div>
			{/if}

			<!-- Status Messages -->
			{#if isComplete}
				<div class="mb-6 p-4 bg-green-50 border border-green-200 rounded-lg">
					<p class="text-sm text-green-700">
						✓ Scan complete! Redirecting to review findings...
					</p>
				</div>
			{:else if isFailed}
				<div class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg">
					<p class="text-sm text-red-700">
						✗ Scan failed: {status?.error_message || 'Unknown error'}
					</p>
				</div>
				<button
					onclick={() => goto('/scan/start')}
					class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors"
					style="background-color: #0284c7; color: white;"
				>
					Try Again
				</button>
			{/if}

			<!-- Broker Status List (Placeholder - Task 7 will add BrokerStatusList component) -->
			{#if isInProgress}
				<div class="text-center py-8">
					<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto"></div>
					<p class="text-sm text-gray-600 mt-4">Scanning brokers...</p>
				</div>
			{/if}

			<!-- Manual Navigate (for testing) -->
			{#if isComplete}
				<div class="mt-6 text-center">
					<button
						onclick={() => goto(`/scan/review/${scanJobId}`)}
						class="text-sm text-primary-600 hover:text-primary-700 transition-colors"
					>
						View Results Now →
					</button>
				</div>
			{/if}
		</div>
	</div>
</div>
```

**Step 2: Commit**

```bash
git add src/routes/scan/progress/[id]/+page.svelte
git commit -m "feat(scan): add progress page with real-time polling

- Poll scan status every 2 seconds
- Progress bar showing broker completion
- Auto-navigate to review when complete
- Status messages for complete/failed states
- Cleanup polling on unmount"
```

---

## Task 4: Create Review Page with Grouped Findings

**Files:**
- Create: `src/routes/scan/review/[id]/+page.svelte`

**Goal:** Display findings grouped by broker with verify/reject actions

**Step 1: Create review page**

Create `src/routes/scan/review/[id]/+page.svelte`:

```svelte
<script lang="ts">
	import { scanStore } from '$lib/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import type { Finding } from '$lib/api/scan';

	const scanJobId = $derived($page.params.id);
	let expandedFindings = $state<Set<string>>(new Set());

	onMount(async () => {
		if (!scanJobId) {
			goto('/scan/start');
			return;
		}

		// Load pending findings
		await scanStore.loadFindings(scanJobId, 'PendingVerification');
	});

	// Group findings by broker
	const groupedFindings = $derived.by(() => {
		const groups = new Map<string, Finding[]>();

		for (const finding of scanStore.findings) {
			const existing = groups.get(finding.broker_id) || [];
			groups.set(finding.broker_id, [...existing, finding]);
		}

		return groups;
	});

	const totalFindings = $derived(scanStore.findings.length);
	const confirmedCount = $derived(
		scanStore.findings.filter(f => f.verification_status === 'Confirmed').length
	);

	async function handleVerify(findingId: string, isMatch: boolean) {
		try {
			await scanStore.verifyFinding(findingId, isMatch);
		} catch (err) {
			console.error('Verification failed:', err);
		}
	}

	async function handleSubmit() {
		if (confirmedCount === 0) {
			return;
		}

		try {
			const count = await scanStore.submitRemovals(scanJobId);
			goto(`/removals?count=${count}`);
		} catch (err) {
			console.error('Submission failed:', err);
		}
	}

	function toggleExpanded(findingId: string) {
		const newSet = new Set(expandedFindings);
		if (newSet.has(findingId)) {
			newSet.delete(findingId);
		} else {
			newSet.add(findingId);
		}
		expandedFindings = newSet;
	}

	function formatPhoneNumbers(phones: string[]): string {
		if (phones.length === 0) return 'None';
		if (phones.length <= 2) return phones.join(', ');
		return `${phones[0]}, ${phones[1]} (+${phones.length - 2} more)`;
	}
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 p-4">
	<div class="max-w-6xl mx-auto">
		<div class="bg-white rounded-lg shadow-xl p-8">
			<!-- Header -->
			<div class="mb-8">
				<h1 class="text-3xl font-bold text-gray-900 mb-2">Review Findings</h1>
				<p class="text-gray-600">
					{totalFindings} result{totalFindings !== 1 ? 's' : ''} found •
					{confirmedCount} confirmed for removal
				</p>
			</div>

			{#if scanStore.loading}
				<div class="flex items-center justify-center py-12">
					<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
				</div>
			{:else if totalFindings === 0}
				<!-- No Findings -->
				<div class="text-center py-12">
					<p class="text-gray-600 mb-6">
						No results found. Your information may not be on these data brokers.
					</p>
					<button
						onclick={() => goto('/')}
						class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors"
						style="background-color: #0284c7; color: white;"
					>
						Return to Dashboard
					</button>
				</div>
			{:else}
				<!-- Instructions -->
				<div class="mb-6 p-4 bg-blue-50 border border-blue-200 rounded-lg">
					<p class="text-sm text-gray-700">
						Review each finding and confirm which ones are accurate matches.
						Only confirmed findings will be submitted for removal.
					</p>
				</div>

				<!-- Grouped Findings -->
				<div class="space-y-6 mb-8">
					{#each [...groupedFindings.entries()] as [brokerId, findings]}
						<div class="border border-gray-200 rounded-lg">
							<!-- Broker Header -->
							<div class="bg-gray-50 px-6 py-4 border-b border-gray-200">
								<h3 class="text-lg font-semibold text-gray-900">
									{brokerId}
									<span class="text-sm font-normal text-gray-600 ml-2">
										({findings.length} result{findings.length !== 1 ? 's' : ''})
									</span>
								</h3>
							</div>

							<!-- Findings List -->
							<div class="divide-y divide-gray-200">
								{#each findings as finding}
									{@const isExpanded = expandedFindings.has(finding.id)}
									{@const isConfirmed = finding.verification_status === 'Confirmed'}
									{@const isRejected = finding.verification_status === 'Rejected'}

									<div class="px-6 py-4">
										<!-- Compact View -->
										<div class="flex items-start justify-between gap-4">
											<div class="flex-1">
												<p class="font-medium text-gray-900">
													{finding.extracted_data.name || 'Unknown Name'}
													{#if finding.extracted_data.age}
														<span class="text-gray-600 text-sm ml-2">
															Age {finding.extracted_data.age}
														</span>
													{/if}
												</p>
												{#if finding.extracted_data.addresses.length > 0}
													<p class="text-sm text-gray-600 mt-1">
														{finding.extracted_data.addresses[0]}
													</p>
												{/if}
												{#if finding.extracted_data.phone_numbers.length > 0}
													<p class="text-sm text-gray-600">
														{formatPhoneNumbers(finding.extracted_data.phone_numbers)}
													</p>
												{/if}
											</div>

											<!-- Actions -->
											<div class="flex items-center gap-2">
												{#if !isConfirmed && !isRejected}
													<button
														onclick={() => handleVerify(finding.id, true)}
														class="px-4 py-2 bg-green-100 text-green-700 rounded-md hover:bg-green-200 transition-colors text-sm font-medium"
													>
														Confirm
													</button>
													<button
														onclick={() => handleVerify(finding.id, false)}
														class="px-4 py-2 bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 transition-colors text-sm font-medium"
													>
														Reject
													</button>
												{:else if isConfirmed}
													<span class="px-4 py-2 bg-green-100 text-green-700 rounded-md text-sm font-medium">
														✓ Confirmed
													</span>
												{:else}
													<span class="px-4 py-2 bg-gray-100 text-gray-500 rounded-md text-sm font-medium">
														✗ Rejected
													</span>
												{/if}

												<button
													onclick={() => toggleExpanded(finding.id)}
													class="px-3 py-2 text-primary-600 hover:bg-primary-50 rounded-md transition-colors text-sm font-medium"
												>
													{isExpanded ? 'Hide' : 'Details'}
												</button>
											</div>
										</div>

										<!-- Expanded View -->
										{#if isExpanded}
											<div class="mt-4 p-4 bg-gray-50 rounded-md">
												<dl class="grid grid-cols-2 gap-4 text-sm">
													{#if finding.extracted_data.name}
														<div>
															<dt class="font-medium text-gray-700">Name</dt>
															<dd class="text-gray-900">{finding.extracted_data.name}</dd>
														</div>
													{/if}
													{#if finding.extracted_data.age}
														<div>
															<dt class="font-medium text-gray-700">Age</dt>
															<dd class="text-gray-900">{finding.extracted_data.age}</dd>
														</div>
													{/if}
													{#if finding.extracted_data.addresses.length > 0}
														<div class="col-span-2">
															<dt class="font-medium text-gray-700">Addresses</dt>
															<dd class="text-gray-900">
																{#each finding.extracted_data.addresses as address}
																	<div>{address}</div>
																{/each}
															</dd>
														</div>
													{/if}
													{#if finding.extracted_data.phone_numbers.length > 0}
														<div class="col-span-2">
															<dt class="font-medium text-gray-700">Phone Numbers</dt>
															<dd class="text-gray-900">
																{finding.extracted_data.phone_numbers.join(', ')}
															</dd>
														</div>
													{/if}
													{#if finding.extracted_data.emails.length > 0}
														<div class="col-span-2">
															<dt class="font-medium text-gray-700">Emails</dt>
															<dd class="text-gray-900">
																{finding.extracted_data.emails.join(', ')}
															</dd>
														</div>
													{/if}
													{#if finding.extracted_data.relatives.length > 0}
														<div class="col-span-2">
															<dt class="font-medium text-gray-700">Relatives</dt>
															<dd class="text-gray-900">
																{finding.extracted_data.relatives.join(', ')}
															</dd>
														</div>
													{/if}
													<div class="col-span-2">
														<dt class="font-medium text-gray-700">Listing URL</dt>
														<dd class="text-gray-900">
															<a
																href={finding.listing_url}
																target="_blank"
																rel="noopener noreferrer"
																class="text-primary-600 hover:text-primary-700 underline"
															>
																View on {brokerId} ↗
															</a>
														</dd>
													</div>
												</dl>
											</div>
										{/if}
									</div>
								{/each}
							</div>
						</div>
					{/each}
				</div>

				<!-- Footer Actions -->
				<div class="flex items-center justify-between pt-6 border-t border-gray-200">
					<button
						onclick={() => goto('/')}
						class="px-6 py-3 border border-gray-300 text-gray-700 rounded-lg font-medium hover:bg-gray-50 transition-colors"
					>
						Cancel
					</button>

					<div class="flex items-center gap-4">
						<p class="text-sm text-gray-600">
							{confirmedCount} of {totalFindings} confirmed
						</p>
						<button
							onclick={handleSubmit}
							disabled={confirmedCount === 0 || scanStore.loading}
							class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
							style="background-color: #0284c7; color: white;"
						>
							{scanStore.loading ? 'Submitting...' : 'Submit Removals'}
						</button>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>
```

**Step 2: Commit**

```bash
git add src/routes/scan/review/[id]/+page.svelte
git commit -m "feat(scan): add review page with grouped findings

- Group findings by broker
- Compact view with key info (name, age, location, phone)
- Expandable details view with all extracted data
- Confirm/reject buttons per finding
- Submit removals when at least 1 confirmed
- Count display and validation"
```

---

## Task 5: Create Removals Placeholder Page

**Files:**
- Create: `src/routes/removals/+page.svelte`

**Goal:** Acknowledge removal submission (Phase 4 will expand this)

**Step 1: Create removals page**

Create `src/routes/removals/+page.svelte`:

```svelte
<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	const removalCount = $derived($page.url.searchParams.get('count') || '0');
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 flex items-center justify-center p-4">
	<div class="bg-white rounded-lg shadow-xl p-8 max-w-2xl w-full text-center">
		<!-- Success Icon -->
		<div class="mb-6">
			<div class="inline-flex items-center justify-center w-16 h-16 bg-green-100 rounded-full">
				<span class="text-4xl text-green-600">✓</span>
			</div>
		</div>

		<!-- Header -->
		<h1 class="text-3xl font-bold text-gray-900 mb-4">
			Removal Requests Submitted
		</h1>

		<p class="text-lg text-gray-600 mb-8">
			We've queued {removalCount} removal request{removalCount !== '1' ? 's' : ''} for processing.
		</p>

		<!-- Info Box -->
		<div class="mb-8 p-6 bg-blue-50 border border-blue-200 rounded-lg text-left">
			<h2 class="text-sm font-semibold text-gray-900 mb-3">What happens next:</h2>
			<ul class="text-sm text-gray-700 space-y-2">
				<li>• Removal requests are being prepared for each data broker</li>
				<li>• You'll be notified when requests are sent</li>
				<li>• Removal tracking coming soon in Phase 4</li>
			</ul>
		</div>

		<!-- Early Access Notice -->
		<div class="mb-8 p-4 bg-yellow-50 border border-yellow-200 rounded-lg">
			<p class="text-sm text-gray-700">
				<strong>Early Access:</strong> Full removal tracking and status updates
				will be available in the next release.
			</p>
		</div>

		<!-- Return to Dashboard -->
		<button
			onclick={() => goto('/')}
			class="px-8 py-4 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors text-lg"
			style="background-color: #0284c7; color: white;"
		>
			Return to Dashboard
		</button>
	</div>
</div>
```

**Step 2: Commit**

```bash
git add src/routes/removals/+page.svelte
git commit -m "feat(removals): add placeholder removals page

- Success confirmation with count
- Info box explaining next steps
- Early access notice
- Return to dashboard button
- Phase 4 will expand with tracking"
```

---

## Task 6: Add Dashboard Integration

**Files:**
- Modify: `src/routes/+page.svelte`

**Goal:** Add "Scan for Your Data" button to dashboard

**Step 1: Update dashboard page**

Modify `src/routes/+page.svelte` - replace the "Coming Soon" section (around line 62-68):

Find this section:
```svelte
<!-- Coming Soon -->
<div class="mt-6 p-4 bg-gray-50 border border-gray-200 rounded-md">
	<p class="text-gray-700 text-sm">
		<strong>Coming Soon:</strong> Data broker scanning and automated removal requests will appear
		here.
	</p>
</div>
```

Replace with:
```svelte
<!-- Scan for Data -->
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
```

**Step 2: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat(dashboard): add scan trigger button

- Replace 'Coming Soon' with 'Scan for Your Data' button
- Link to /scan/start
- Inline styles matching design system"
```

---

## Task 7: Create Reusable Components

**Files:**
- Create: `src/lib/components/scan/ScanProgressBar.svelte`
- Create: `src/lib/components/scan/BrokerStatusBadge.svelte`

**Goal:** Reusable components for scan UI

**Step 1: Create ScanProgressBar component**

Create `src/lib/components/scan/ScanProgressBar.svelte`:

```svelte
<script lang="ts">
	interface Props {
		current: number;
		total: number;
	}

	let { current, total }: Props = $props();

	const percentage = $derived(total > 0 ? Math.round((current / total) * 100) : 0);
</script>

<div class="w-full">
	<div class="flex items-center justify-between mb-2">
		<span class="text-sm font-medium text-gray-700">Progress</span>
		<span class="text-sm font-medium text-gray-700">{percentage}%</span>
	</div>
	<div class="w-full bg-gray-200 rounded-full h-3">
		<div
			class="bg-primary-600 h-3 rounded-full transition-all duration-500"
			style="width: {percentage}%; background-color: #0284c7;"
		></div>
	</div>
	<div class="flex items-center justify-between mt-2">
		<span class="text-xs text-gray-600">
			{current} of {total} complete
		</span>
	</div>
</div>
```

**Step 2: Create BrokerStatusBadge component**

Create `src/lib/components/scan/BrokerStatusBadge.svelte`:

```svelte
<script lang="ts">
	interface Props {
		status: 'Pending' | 'InProgress' | 'Success' | 'Failed' | 'Skipped';
	}

	let { status }: Props = $props();

	const config = $derived.by(() => {
		switch (status) {
			case 'Pending':
				return { color: 'bg-gray-200 text-gray-700', icon: '·' };
			case 'InProgress':
				return { color: 'bg-blue-100 text-blue-700', icon: '⟳' };
			case 'Success':
				return { color: 'bg-green-100 text-green-700', icon: '✓' };
			case 'Failed':
				return { color: 'bg-red-100 text-red-700', icon: '✗' };
			case 'Skipped':
				return { color: 'bg-yellow-100 text-yellow-700', icon: '⊘' };
			default:
				return { color: 'bg-gray-100 text-gray-700', icon: '·' };
		}
	});
</script>

<span class="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium {config.color}">
	<span class="mr-1">{config.icon}</span>
	{status}
</span>
```

**Step 3: Create components barrel export**

Create or modify `src/lib/components/scan/index.ts`:

```typescript
export { default as ScanProgressBar } from './ScanProgressBar.svelte';
export { default as BrokerStatusBadge } from './BrokerStatusBadge.svelte';
```

**Step 4: Commit**

```bash
git add src/lib/components/scan/
git commit -m "feat(components): add reusable scan components

- ScanProgressBar: animated progress with percentage
- BrokerStatusBadge: colored status badges with icons
- Barrel export for easy imports"
```

---

## Testing

**Manual Testing Checklist:**

1. **Scan Start Page:**
   - [ ] No profile → redirects to profile setup
   - [ ] Single profile → shows profile info, no dropdown
   - [ ] Multiple profiles → shows dropdown
   - [ ] Start scan → navigates to progress page

2. **Progress Page:**
   - [ ] Progress bar updates every 2 seconds
   - [ ] Status badges show correctly
   - [ ] Auto-navigates to review when complete
   - [ ] Failed scan shows error + retry button

3. **Review Page:**
   - [ ] Findings grouped by broker
   - [ ] Confirm button marks as confirmed
   - [ ] Reject button marks as rejected
   - [ ] Details expand/collapse
   - [ ] Submit disabled with 0 confirmed
   - [ ] Submit redirects to removals page

4. **Removals Page:**
   - [ ] Shows correct count from query param
   - [ ] Return to dashboard works

5. **Dashboard:**
   - [ ] "Scan for Your Data" button links to /scan/start

**Run Type Check:**
```bash
npm run check
```

**Run Lint:**
```bash
npm run lint
```

---

## Completion Checklist

- [ ] All 7 tasks completed
- [ ] Manual testing passed
- [ ] Type check passes
- [ ] Lint passes
- [ ] All commits pushed to branch

**Ready for review and merge!**
