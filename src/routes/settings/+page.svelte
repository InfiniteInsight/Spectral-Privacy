<script lang="ts">
	import { page } from '$app/stores';
	import { vaultStore } from '$lib/stores/vault.svelte';
	import {
		testSmtpConnection,
		testImapConnection,
		getScheduledJobs,
		updateScheduledJob,
		runJobNow,
		type ScheduledJob
	} from '$lib/api/settings';

	// Tab from query param: ?tab=privacy (default), email, scheduling, audit
	let activeTab = $derived($page.url.searchParams.get('tab') ?? 'privacy');

	// Email settings state
	let smtpEnabled = $state(false);
	let smtpHost = $state('');
	let smtpPort = $state(587);
	let smtpUsername = $state('');
	let smtpPassword = $state('');
	let imapEnabled = $state(false);
	let imapHost = $state('');
	let imapPort = $state(993);
	let imapUsername = $state('');
	let imapPassword = $state('');
	let smtpTestResult = $state<'idle' | 'testing' | 'success' | 'error'>('idle');
	let imapTestResult = $state<'idle' | 'testing' | 'success' | 'error'>('idle');
	let smtpError = $state('');
	let imapError = $state('');

	// Scheduling state
	let scheduledJobs = $state<ScheduledJob[]>([]);
	let loadingJobs = $state(false);
	let schedulingError = $state<string | null>(null);

	async function handleTestSmtp() {
		smtpTestResult = 'testing';
		try {
			await testSmtpConnection(smtpHost, smtpPort, smtpUsername, smtpPassword);
			smtpTestResult = 'success';
		} catch (err) {
			smtpTestResult = 'error';
			smtpError = err instanceof Error ? err.message : String(err);
		}
	}

	async function handleTestImap() {
		imapTestResult = 'testing';
		try {
			await testImapConnection(imapHost, imapPort, imapUsername, imapPassword);
			imapTestResult = 'success';
		} catch (err) {
			imapTestResult = 'error';
			imapError = err instanceof Error ? err.message : String(err);
		}
	}

	// Load scheduled jobs when scheduling tab becomes active
	$effect(() => {
		if (activeTab === 'scheduling' && vaultStore.currentVaultId) {
			loadScheduledJobs();
		}
	});

	async function loadScheduledJobs() {
		if (!vaultStore.currentVaultId) return;
		loadingJobs = true;
		schedulingError = null;
		try {
			scheduledJobs = await getScheduledJobs(vaultStore.currentVaultId);
		} catch (err) {
			schedulingError = err instanceof Error ? err.message : String(err);
			console.error('Failed to load scheduled jobs:', err);
		} finally {
			loadingJobs = false;
		}
	}

	async function handleUpdateJob(jobId: string, intervalDays: number, enabled: boolean) {
		if (!vaultStore.currentVaultId) return;
		schedulingError = null;
		try {
			await updateScheduledJob(vaultStore.currentVaultId, jobId, intervalDays, enabled);
			await loadScheduledJobs(); // Reload to get updated next_run_at
		} catch (err) {
			schedulingError = err instanceof Error ? err.message : String(err);
			console.error('Failed to update job:', err);
		}
	}

	async function handleRunNow(jobType: string) {
		if (!vaultStore.currentVaultId) return;
		schedulingError = null;
		try {
			await runJobNow(vaultStore.currentVaultId, jobType);
		} catch (err) {
			schedulingError = err instanceof Error ? err.message : String(err);
			console.error('Failed to run job:', err);
		}
	}
</script>

<div class="mx-auto max-w-3xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-bold text-gray-900">Settings</h1>

	<!-- Tab bar -->
	<div class="mb-8 flex gap-1 border-b border-gray-200" role="tablist">
		{#each [['privacy', 'Privacy Level'], ['email', 'Email'], ['scheduling', 'Scheduling'], ['audit', 'Audit Log']] as [id, label] (id)}
			<a
				href="/settings?tab={id}"
				role="tab"
				aria-selected={activeTab === id}
				class="px-4 py-2 text-sm font-medium {activeTab === id
					? 'border-b-2 border-primary-600 text-primary-700'
					: 'text-gray-500 hover:text-gray-700'}">{label}</a
			>
		{/each}
	</div>

	<!-- Privacy Level tab -->
	{#if activeTab === 'privacy'}
		<section>
			<h2 class="mb-2 text-lg font-semibold text-gray-800">Privacy Level</h2>
			<div class="rounded-lg border border-blue-200 bg-blue-50 p-6">
				<div class="flex items-start gap-3">
					<svg
						class="h-6 w-6 flex-shrink-0 text-blue-600"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
						/>
					</svg>
					<div>
						<h3 class="mb-2 font-medium text-blue-900">Coming Soon</h3>
						<p class="mb-3 text-sm text-blue-800">
							This tab will allow you to configure privacy presets and LLM integration preferences
							in a future release. Options will include:
						</p>
						<ul class="mb-3 space-y-1 text-sm text-blue-800">
							<li class="flex items-center gap-2">
								<span class="text-blue-600">•</span>
								<span>Local-only processing with no cloud services</span>
							</li>
							<li class="flex items-center gap-2">
								<span class="text-blue-600">•</span>
								<span>Optional LLM assistance for form filling and email drafting</span>
							</li>
							<li class="flex items-center gap-2">
								<span class="text-blue-600">•</span>
								<span>Configurable privacy/convenience trade-offs</span>
							</li>
						</ul>
						<p class="text-sm text-blue-700">
							For now, view your current <a
								href="/score"
								class="font-medium underline hover:text-blue-900">Privacy Score</a
							> to track your data exposure reduction progress.
						</p>
					</div>
				</div>
			</div>
		</section>
	{:else if activeTab === 'email'}
		<section class="space-y-6">
			<!-- SMTP Configuration Card -->
			<div class="rounded-lg border border-gray-200 bg-white p-4">
				<div class="mb-4 flex items-center justify-between">
					<div>
						<h3 class="font-medium text-gray-900">SMTP Email Sending</h3>
						<p class="text-sm text-gray-500">Send opt-out emails via your mail server</p>
					</div>
					<label class="flex cursor-pointer items-center gap-2">
						<span class="text-sm text-gray-700">Enable SMTP</span>
						<input type="checkbox" bind:checked={smtpEnabled} class="peer sr-only" />
						<div
							class="h-6 w-11 rounded-full bg-gray-200 transition-colors peer-checked:bg-primary-600 peer-focus:outline-none"
						></div>
					</label>
				</div>
				{#if smtpEnabled}
					<div class="grid grid-cols-2 gap-4">
						<div>
							<label for="smtp-host" class="mb-1 block text-sm font-medium text-gray-700"
								>SMTP Host</label
							>
							<input
								id="smtp-host"
								type="text"
								bind:value={smtpHost}
								placeholder="smtp.gmail.com"
								class="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							/>
						</div>
						<div>
							<label for="smtp-port" class="mb-1 block text-sm font-medium text-gray-700"
								>SMTP Port</label
							>
							<input
								id="smtp-port"
								type="number"
								bind:value={smtpPort}
								class="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							/>
						</div>
						<div>
							<label for="smtp-user" class="mb-1 block text-sm font-medium text-gray-700"
								>Username</label
							>
							<input
								id="smtp-user"
								type="text"
								bind:value={smtpUsername}
								class="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							/>
						</div>
						<div>
							<label for="smtp-pass" class="mb-1 block text-sm font-medium text-gray-700"
								>Password</label
							>
							<input
								id="smtp-pass"
								type="password"
								bind:value={smtpPassword}
								class="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							/>
						</div>
					</div>
					<div class="mt-3 flex items-center gap-3">
						<button
							onclick={handleTestSmtp}
							disabled={smtpTestResult === 'testing'}
							class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
						>
							{smtpTestResult === 'testing' ? 'Testing...' : 'Test Connection'}
						</button>
						{#if smtpTestResult === 'success'}
							<span class="text-sm text-green-600">Connected successfully</span>
						{:else if smtpTestResult === 'error'}
							<span class="text-sm text-red-600">{smtpError}</span>
						{/if}
					</div>
				{/if}
			</div>

			<!-- IMAP Configuration Card -->
			<div class="rounded-lg border border-gray-200 bg-white p-4">
				<div class="mb-4 flex items-center justify-between">
					<div>
						<h3 class="font-medium text-gray-900">IMAP Verification Monitoring</h3>
						<p class="text-sm text-gray-500">
							Automatically detect confirmation emails from brokers
						</p>
					</div>
					<label class="flex cursor-pointer items-center gap-2">
						<span class="text-sm text-gray-700">Enable IMAP</span>
						<input type="checkbox" bind:checked={imapEnabled} class="peer sr-only" />
						<div
							class="h-6 w-11 rounded-full bg-gray-200 transition-colors peer-checked:bg-primary-600 peer-focus:outline-none"
						></div>
					</label>
				</div>
				{#if imapEnabled}
					<div class="grid grid-cols-2 gap-4">
						<div>
							<label for="imap-host" class="mb-1 block text-sm font-medium text-gray-700"
								>IMAP Host</label
							>
							<input
								id="imap-host"
								type="text"
								bind:value={imapHost}
								placeholder="imap.gmail.com"
								class="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							/>
						</div>
						<div>
							<label for="imap-port" class="mb-1 block text-sm font-medium text-gray-700"
								>IMAP Port</label
							>
							<input
								id="imap-port"
								type="number"
								bind:value={imapPort}
								class="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							/>
						</div>
						<div>
							<label for="imap-user" class="mb-1 block text-sm font-medium text-gray-700"
								>Username</label
							>
							<input
								id="imap-user"
								type="text"
								bind:value={imapUsername}
								class="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							/>
						</div>
						<div>
							<label for="imap-pass" class="mb-1 block text-sm font-medium text-gray-700"
								>Password</label
							>
							<input
								id="imap-pass"
								type="password"
								bind:value={imapPassword}
								class="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							/>
						</div>
					</div>
					<div class="mt-3 flex items-center gap-3">
						<button
							onclick={handleTestImap}
							disabled={imapTestResult === 'testing'}
							class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
						>
							{imapTestResult === 'testing' ? 'Testing...' : 'Test Connection'}
						</button>
						{#if imapTestResult === 'success'}
							<span class="text-sm text-green-600">Connected successfully</span>
						{:else if imapTestResult === 'error'}
							<span class="text-sm text-red-600">{imapError}</span>
						{/if}
					</div>
				{/if}
			</div>
		</section>
	{:else if activeTab === 'scheduling'}
		<section>
			<h2 class="mb-4 text-lg font-semibold text-gray-800">Scheduled Jobs</h2>
			{#if schedulingError}
				<div class="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-900">
					{schedulingError}
				</div>
			{/if}
			{#if loadingJobs}
				<p class="text-gray-500">Loading...</p>
			{:else}
				<div class="space-y-4">
					{#each scheduledJobs as job}
						{@const jobName =
							job.job_type === 'ScanAll'
								? 'Weekly Scan'
								: job.job_type === 'VerifyRemovals'
									? 'Removal Verification'
									: job.job_type}
						<div class="rounded-lg border border-gray-200 p-4">
							<div class="mb-3 flex items-center justify-between">
								<div>
									<h3 class="font-medium text-gray-900">{jobName}</h3>
									<p class="text-sm text-gray-500">
										Next run: {new Date(job.next_run_at).toLocaleString()}
									</p>
								</div>
								<label class="flex items-center gap-2">
									<input
										type="checkbox"
										checked={job.enabled}
										onchange={(e) =>
											handleUpdateJob(job.id, job.interval_days, e.currentTarget.checked)}
										class="rounded"
									/>
									<span class="text-sm">Enabled</span>
								</label>
							</div>
							<div class="flex items-center gap-4">
								<select
									value={job.interval_days}
									onchange={(e) =>
										handleUpdateJob(job.id, parseInt(e.currentTarget.value), job.enabled)}
									class="rounded-lg border border-gray-300 px-3 py-2 text-sm"
								>
									<option value="1">Daily</option>
									<option value="3">Every 3 days</option>
									<option value="7">Weekly</option>
									<option value="14">Bi-weekly</option>
									<option value="30">Monthly</option>
								</select>
								<button
									onclick={() => handleRunNow(job.job_type)}
									class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700"
								>
									Run Now
								</button>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</section>
	{:else if activeTab === 'audit'}
		<section>
			<h2 class="mb-2 text-lg font-semibold text-gray-800">Privacy Audit Log</h2>
			<p class="mb-4 text-sm text-gray-500">Audit log will appear here (Phase 6 Task 5)</p>
		</section>
	{/if}
</div>
