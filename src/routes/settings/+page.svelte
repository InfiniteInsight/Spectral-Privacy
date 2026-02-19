<script lang="ts">
	import { page } from '$app/stores';
	import { vaultStore } from '$lib/stores/vault.svelte';
	import { renameVault, deleteVault } from '$lib/api/vault';
	import {
		testSmtpConnection,
		testImapConnection,
		getScheduledJobs,
		updateScheduledJob,
		runJobNow,
		type ScheduledJob
	} from '$lib/api/settings';

	// Tab from query param: ?tab=vaults (default), privacy, email, scheduling, audit
	let activeTab = $derived($page.url.searchParams.get('tab') ?? 'vaults');

	// Vault management state
	let renameTarget = $state<string | null>(null);
	let renameValue = $state('');
	let deleteTarget = $state<string | null>(null);
	let deletePassword = $state('');
	let actionError = $state<string | null>(null);
	let actionLoading = $state(false);

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

	async function handleRename(vaultId: string) {
		actionError = null;
		actionLoading = true;
		try {
			await renameVault(vaultId, renameValue);
			await vaultStore.loadVaults();
			renameTarget = null;
			renameValue = '';
		} catch (err) {
			actionError = err instanceof Error ? err.message : String(err);
		} finally {
			actionLoading = false;
		}
	}

	async function handleDelete(vaultId: string) {
		actionError = null;
		actionLoading = true;
		try {
			await deleteVault(vaultId, deletePassword);
			await vaultStore.loadVaults();
			deleteTarget = null;
			deletePassword = '';
		} catch (err) {
			actionError = err instanceof Error ? err.message : String(err);
		} finally {
			actionLoading = false;
		}
	}

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
		{#each [['vaults', 'Vaults'], ['privacy', 'Privacy Level'], ['email', 'Email'], ['scheduling', 'Scheduling'], ['audit', 'Audit Log']] as [id, label] (id)}
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

	<!-- Vaults tab -->
	{#if activeTab === 'vaults'}
		<section>
			<h2 class="mb-4 text-lg font-semibold text-gray-800">Your vaults</h2>
			<div class="space-y-3">
				{#each vaultStore.availableVaults as vault (vault.vault_id)}
					<div class="rounded-lg border border-gray-200 bg-white p-4">
						{#if renameTarget === vault.vault_id}
							<div class="flex items-center gap-3">
								<input
									bind:value={renameValue}
									class="flex-1 rounded-md border border-gray-300 px-3 py-1.5 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
									onkeydown={(e) => e.key === 'Enter' && handleRename(vault.vault_id)}
								/>
								<button
									onclick={() => handleRename(vault.vault_id)}
									disabled={actionLoading}
									class="rounded-md bg-primary-600 px-3 py-1.5 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
									>Save</button
								>
								<button
									onclick={() => {
										renameTarget = null;
										renameValue = '';
										actionError = null;
									}}
									class="rounded-md border border-gray-300 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
									>Cancel</button
								>
							</div>
						{:else}
							<div class="flex items-center justify-between">
								<div>
									<p class="font-medium text-gray-900">{vault.display_name}</p>
									<p class="text-xs text-gray-400">
										Last accessed: {new Date(vault.last_accessed).toLocaleDateString()}
									</p>
								</div>
								<div class="flex gap-2">
									<button
										onclick={() => {
											renameTarget = vault.vault_id;
											renameValue = vault.display_name;
										}}
										class="rounded-md border border-gray-200 px-3 py-1.5 text-xs text-gray-600 hover:bg-gray-50"
										>Rename</button
									>
									<button
										onclick={() => {
											deleteTarget = vault.vault_id;
											actionError = null;
										}}
										class="rounded-md border border-red-200 px-3 py-1.5 text-xs text-red-600 hover:bg-red-50"
										>Delete</button
									>
								</div>
							</div>
						{/if}
					</div>
				{/each}
			</div>
			{#if actionError && (renameTarget !== null || deleteTarget !== null)}
				<p class="mt-2 text-sm text-red-600">{actionError}</p>
			{/if}
			<div class="mt-4">
				<a
					href="/profile/setup"
					class="inline-block rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50"
					>+ Add new vault</a
				>
			</div>
		</section>
	{:else if activeTab === 'privacy'}
		<section>
			<h2 class="mb-2 text-lg font-semibold text-gray-800">Privacy Level</h2>
			<p class="mb-6 text-sm text-gray-500">
				Choose how Spectral handles your data. This affects which features are available.
			</p>
			<div class="grid grid-cols-2 gap-4">
				{#each [{ id: 'paranoid', label: 'Paranoid', desc: 'No LLM, no network scanning, manual everything. Full control.' }, { id: 'local', label: 'Local Privacy', desc: 'Local LLM only, filesystem/email scanning, no cloud APIs.', recommended: true }, { id: 'balanced', label: 'Balanced', desc: 'Full features with cloud LLMs, PII filtering enforced.' }, { id: 'custom', label: 'Custom', desc: 'Everything disabled — enable as needed.' }] as preset (preset.id)}
					<button
						class="relative rounded-lg border-2 border-gray-200 p-4 text-left hover:border-primary-300 focus:outline-none focus:ring-2 focus:ring-primary-500"
					>
						{#if preset.recommended}
							<span
								class="absolute right-3 top-3 rounded-full bg-primary-100 px-2 py-0.5 text-xs font-medium text-primary-700"
								>Recommended</span
							>
						{/if}
						<p class="font-medium text-gray-900">{preset.label}</p>
						<p class="mt-1 text-xs text-gray-500">{preset.desc}</p>
					</button>
				{/each}
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

	<!-- Delete vault confirmation modal -->
	{#if deleteTarget}
		<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
			<div
				class="w-full max-w-sm rounded-lg bg-white p-6 shadow-xl"
				role="dialog"
				aria-modal="true"
				aria-labelledby="delete-vault-title"
			>
				<h2 id="delete-vault-title" class="mb-2 text-lg font-semibold text-gray-900">
					Delete vault?
				</h2>
				<p class="mb-4 text-sm text-gray-500">
					This permanently deletes all data in this vault. Enter your master password to confirm.
				</p>
				<input
					type="password"
					bind:value={deletePassword}
					placeholder="Master password"
					autocomplete="current-password"
					class="mb-3 w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-red-400 focus:outline-none focus:ring-1 focus:ring-red-400"
				/>
				{#if actionError}
					<p class="mb-3 text-sm text-red-600">{actionError}</p>
				{/if}
				<div class="flex gap-3">
					<button
						onclick={() => handleDelete(deleteTarget!)}
						disabled={actionLoading}
						class="flex-1 rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:opacity-50"
						>Delete permanently</button
					>
					<button
						onclick={() => {
							deleteTarget = null;
							deletePassword = '';
							actionError = null;
						}}
						class="flex-1 rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50"
						>Cancel</button
					>
				</div>
			</div>
		</div>
	{/if}
</div>
