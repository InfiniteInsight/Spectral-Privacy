<script lang="ts">
	import { page } from '$app/stores';
	import { vaultStore } from '$lib/stores/vault.svelte';
	import { profileStore } from '$lib/stores';
	import {
		testSmtpConnection,
		testImapConnection,
		getScheduledJobs,
		updateScheduledJob,
		runJobNow,
		type ScheduledJob
	} from '$lib/api/settings';
	import {
		getPrivacySettings,
		setPrivacyLevel,
		setCustomFeatureFlags,
		getLlmProviderSettings,
		setLlmPrimaryProvider,
		setLlmTaskProvider,
		setLlmApiKey,
		testLlmProvider,
		type PrivacyLevel,
		type PrivacySettings,
		type FeatureFlags,
		type LlmProvider,
		type LlmProviderSettings,
		type TaskType
	} from '$lib/api/privacy';
	import { auditAPI, type AuditLogEntry } from '$lib/api/audit';
	import EmailProviderAccordion from '$lib/components/EmailProviderAccordion.svelte';
	import EmailSettings from '$lib/components/EmailSettings.svelte';
	import ProfileWizard from '$lib/components/profile/ProfileWizard.svelte';
	import Spinner from '$lib/components/Spinner.svelte';

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

	// Privacy settings state
	let privacySettings = $state<PrivacySettings | null>(null);
	let loadingPrivacy = $state(false);
	let privacyError = $state<string | null>(null);

	// LLM provider settings state
	let llmSettings = $state<LlmProviderSettings | null>(null);
	let loadingLlm = $state(false);
	let llmError = $state<string | null>(null);
	let llmTestResults = $state<Record<LlmProvider, 'idle' | 'testing' | 'success' | 'error'>>({
		'open-ai': 'idle',
		gemini: 'idle',
		claude: 'idle',
		ollama: 'idle',
		'lm-studio': 'idle'
	});
	let llmTestMessages = $state<Record<LlmProvider, string>>({
		'open-ai': '',
		gemini: '',
		claude: '',
		ollama: '',
		'lm-studio': ''
	});

	// API key inputs (local state, not persisted until save)
	let apiKeys = $state<Record<string, string>>({
		'open-ai': '',
		gemini: '',
		claude: ''
	});

	// Audit log state
	let auditEntries = $state<AuditLogEntry[]>([]);
	let loadingAudit = $state(false);
	let auditError = $state<string | null>(null);
	let auditFilter = $state<string>('all'); // 'all' or specific event type

	// Profile state
	let loadingProfile = $state(false);
	let profileError = $state<string | null>(null);

	// Email setup help state
	let showEmailHelp = $state(false);
	let expandedProvider = $state<string | null>(null);

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

	// Load privacy settings when privacy tab becomes active
	$effect(() => {
		if (activeTab === 'privacy' && vaultStore.currentVaultId) {
			loadPrivacySettings();
		}
	});

	// Load LLM settings when llm tab becomes active
	$effect(() => {
		if (activeTab === 'llm' && vaultStore.currentVaultId) {
			loadLlmSettings();
		}
	});

	// Load audit log when audit tab becomes active
	$effect(() => {
		if (activeTab === 'audit' && vaultStore.currentVaultId) {
			loadAuditLog();
		}
	});

	// Load profile when profile tab becomes active
	$effect(() => {
		if (activeTab === 'profile' && vaultStore.currentVaultId) {
			loadProfileData();
		}
	});

	async function loadProfileData() {
		if (!vaultStore.currentVaultId) return;

		loadingProfile = true;
		profileError = null;

		try {
			await profileStore.loadProfiles(vaultStore.currentVaultId);

			// Load full profile data if exists
			if (profileStore.profiles.length > 0) {
				const profileId = profileStore.profiles[0].id;
				await profileStore.loadProfile(vaultStore.currentVaultId, profileId);
				console.log('Settings - loaded profile:', profileStore.currentProfile);
				console.log(
					'Settings - profile.email_addresses:',
					profileStore.currentProfile?.email_addresses
				);
				console.log(
					'Settings - profile.phone_numbers:',
					profileStore.currentProfile?.phone_numbers
				);
			}
		} catch (err) {
			profileError = err instanceof Error ? err.message : String(err);
			console.error('Failed to load profile:', err);
		} finally {
			loadingProfile = false;
		}
	}

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

	async function loadPrivacySettings() {
		if (!vaultStore.currentVaultId) return;
		loadingPrivacy = true;
		privacyError = null;
		try {
			privacySettings = await getPrivacySettings(vaultStore.currentVaultId);
		} catch (err) {
			privacyError = err instanceof Error ? err.message : String(err);
			console.error('Failed to load privacy settings:', err);
		} finally {
			loadingPrivacy = false;
		}
	}

	async function handleSetPrivacyLevel(level: PrivacyLevel) {
		if (!vaultStore.currentVaultId) return;
		privacyError = null;
		try {
			await setPrivacyLevel(vaultStore.currentVaultId, level);
			await loadPrivacySettings(); // Reload to get updated feature flags
		} catch (err) {
			privacyError = err instanceof Error ? err.message : String(err);
			console.error('Failed to set privacy level:', err);
		}
	}

	async function handleUpdateFeatureFlag(flag: keyof FeatureFlags, value: boolean) {
		if (!vaultStore.currentVaultId || !privacySettings) return;
		privacyError = null;
		try {
			const updatedFlags = { ...privacySettings.feature_flags, [flag]: value };
			await setCustomFeatureFlags(vaultStore.currentVaultId, updatedFlags);
			await loadPrivacySettings();
		} catch (err) {
			privacyError = err instanceof Error ? err.message : String(err);
			console.error('Failed to update feature flag:', err);
		}
	}

	async function loadLlmSettings() {
		if (!vaultStore.currentVaultId) return;
		loadingLlm = true;
		llmError = null;
		try {
			llmSettings = await getLlmProviderSettings(vaultStore.currentVaultId);
		} catch (err) {
			llmError = err instanceof Error ? err.message : String(err);
			console.error('Failed to load LLM settings:', err);
		} finally {
			loadingLlm = false;
		}
	}

	async function handleSetPrimaryProvider(provider: LlmProvider) {
		if (!vaultStore.currentVaultId) return;
		llmError = null;
		try {
			await setLlmPrimaryProvider(vaultStore.currentVaultId, provider);
			await loadLlmSettings();
		} catch (err) {
			llmError = err instanceof Error ? err.message : String(err);
			console.error('Failed to set primary provider:', err);
		}
	}

	async function handleSetTaskProvider(taskType: TaskType, provider: LlmProvider) {
		if (!vaultStore.currentVaultId) return;
		llmError = null;
		try {
			await setLlmTaskProvider(vaultStore.currentVaultId, taskType, provider);
			await loadLlmSettings();
		} catch (err) {
			llmError = err instanceof Error ? err.message : String(err);
			console.error('Failed to set task provider:', err);
		}
	}

	async function handleSaveApiKey(provider: LlmProvider) {
		if (!vaultStore.currentVaultId) return;
		const key = apiKeys[provider];
		if (!key) return;
		llmError = null;
		try {
			await setLlmApiKey(vaultStore.currentVaultId, provider, key);
			await loadLlmSettings(); // Reload to update has_*_key flags
			apiKeys[provider] = ''; // Clear input after save
		} catch (err) {
			llmError = err instanceof Error ? err.message : String(err);
			console.error('Failed to save API key:', err);
		}
	}

	async function handleTestProvider(provider: LlmProvider) {
		if (!vaultStore.currentVaultId) return;
		llmTestResults[provider] = 'testing';
		llmTestMessages[provider] = '';
		try {
			const message = await testLlmProvider(vaultStore.currentVaultId, provider);
			llmTestResults[provider] = 'success';
			llmTestMessages[provider] = message;
		} catch (err) {
			llmTestResults[provider] = 'error';
			llmTestMessages[provider] = err instanceof Error ? err.message : String(err);
		}
	}

	async function loadAuditLog() {
		if (!vaultStore.currentVaultId) return;
		loadingAudit = true;
		auditError = null;
		try {
			if (auditFilter === 'all') {
				auditEntries = await auditAPI.getAuditLog(vaultStore.currentVaultId, 100);
			} else {
				auditEntries = await auditAPI.getAuditLogByType(
					vaultStore.currentVaultId,
					auditFilter,
					100
				);
			}
		} catch (err) {
			auditError = err instanceof Error ? err.message : String(err);
			console.error('Failed to load audit log:', err);
		} finally {
			loadingAudit = false;
		}
	}

	async function handleClearAuditLog() {
		if (!vaultStore.currentVaultId) return;
		if (!confirm('Are you sure you want to clear all audit log entries? This cannot be undone.')) {
			return;
		}
		auditError = null;
		try {
			await auditAPI.clearAuditLog(vaultStore.currentVaultId);
			await loadAuditLog(); // Reload to show empty state
		} catch (err) {
			auditError = err instanceof Error ? err.message : String(err);
			console.error('Failed to clear audit log:', err);
		}
	}

	function formatTimestamp(timestamp: string): string {
		const date = new Date(timestamp);
		return date.toLocaleString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		});
	}

	function getEventTypeColor(eventType: string): string {
		if (eventType.includes('Error') || eventType.includes('Failed'))
			return 'bg-red-100 text-red-700';
		if (eventType.includes('Started') || eventType.includes('Created'))
			return 'bg-blue-100 text-blue-700';
		if (eventType.includes('Completed') || eventType.includes('Success'))
			return 'bg-green-100 text-green-700';
		if (eventType.includes('Denied')) return 'bg-orange-100 text-orange-700';
		return 'bg-gray-100 text-gray-700';
	}
</script>

<div class="mx-auto max-w-3xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-bold text-gray-900">Settings</h1>

	<!-- Tab bar -->
	<div class="mb-8 flex gap-1 border-b border-gray-200" role="tablist">
		{#each [['profile', 'Profile'], ['privacy', 'Privacy Level'], ['llm', 'LLM Providers'], ['email', 'Email'], ['scheduling', 'Scheduling'], ['audit', 'Audit Log']] as [id, label] (id)}
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

	<!-- Profile tab -->
	{#if activeTab === 'profile'}
		<section>
			<h2 class="mb-4 text-lg font-semibold text-gray-800">Your Profile</h2>

			{#if profileError}
				<div class="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-900">
					{profileError}
				</div>
			{/if}

			{#if loadingProfile}
				<div class="flex items-center justify-center py-8">
					<Spinner />
				</div>
			{:else if profileStore.currentProfile}
				<ProfileWizard
					mode="edit"
					profileId={profileStore.currentProfile.id}
					initialData={profileStore.currentProfile}
				/>
			{:else}
				<div class="rounded-lg border border-gray-200 bg-gray-50 p-8 text-center">
					<p class="mb-4 text-gray-600">No profile found.</p>
					<a
						href="/profile/setup"
						class="inline-block px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700"
						style="background-color: #0284c7; color: white;"
					>
						Create Profile
					</a>
				</div>
			{/if}
		</section>
	{:else if activeTab === 'privacy'}
		<section>
			<h2 class="mb-4 text-lg font-semibold text-gray-800">Privacy Level</h2>
			{#if privacyError}
				<div class="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-900">
					{privacyError}
				</div>
			{/if}
			{#if loadingPrivacy}
				<p class="text-gray-500">Loading...</p>
			{:else if privacySettings}
				<p class="mb-4 text-sm text-gray-600">
					Choose a privacy preset that balances security and convenience for your needs.
				</p>
				<div class="grid gap-4 md:grid-cols-2">
					<!-- Paranoid preset -->
					<button
						onclick={() => handleSetPrivacyLevel('paranoid')}
						class="rounded-lg border p-4 text-left transition-all {privacySettings.privacy_level ===
						'paranoid'
							? 'border-primary-600 bg-primary-50 ring-2 ring-primary-600'
							: 'border-gray-200 bg-white hover:border-gray-300'}"
					>
						<div class="mb-2 flex items-center gap-2">
							<svg
								class="h-5 w-5 {privacySettings.privacy_level === 'paranoid'
									? 'text-primary-600'
									: 'text-gray-600'}"
								fill="none"
								viewBox="0 0 24 24"
								stroke="currentColor"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
								/>
							</svg>
							<h3
								class="font-semibold {privacySettings.privacy_level === 'paranoid'
									? 'text-primary-900'
									: 'text-gray-900'}"
							>
								Paranoid
							</h3>
						</div>
						<p class="text-sm text-gray-600">
							Maximum privacy. No automation, no cloud services, manual-only operations.
						</p>
					</button>

					<!-- Local Privacy preset -->
					<button
						onclick={() => handleSetPrivacyLevel('local_privacy')}
						class="rounded-lg border p-4 text-left transition-all {privacySettings.privacy_level ===
						'local_privacy'
							? 'border-primary-600 bg-primary-50 ring-2 ring-primary-600'
							: 'border-gray-200 bg-white hover:border-gray-300'}"
					>
						<div class="mb-2 flex items-center gap-2">
							<svg
								class="h-5 w-5 {privacySettings.privacy_level === 'local_privacy'
									? 'text-primary-600'
									: 'text-gray-600'}"
								fill="none"
								viewBox="0 0 24 24"
								stroke="currentColor"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
								/>
							</svg>
							<h3
								class="font-semibold {privacySettings.privacy_level === 'local_privacy'
									? 'text-primary-900'
									: 'text-gray-900'}"
							>
								Local Privacy
							</h3>
						</div>
						<p class="text-sm text-gray-600">
							Local LLM assistance only (Ollama/LM Studio). Email automation enabled.
						</p>
					</button>

					<!-- Balanced preset -->
					<button
						onclick={() => handleSetPrivacyLevel('balanced')}
						class="rounded-lg border p-4 text-left transition-all {privacySettings.privacy_level ===
						'balanced'
							? 'border-primary-600 bg-primary-50 ring-2 ring-primary-600'
							: 'border-gray-200 bg-white hover:border-gray-300'}"
					>
						<div class="mb-2 flex items-center gap-2">
							<svg
								class="h-5 w-5 {privacySettings.privacy_level === 'balanced'
									? 'text-primary-600'
									: 'text-gray-600'}"
								fill="none"
								viewBox="0 0 24 24"
								stroke="currentColor"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M3 6l3 1m0 0l-3 9a5.002 5.002 0 006.001 0M6 7l3 9M6 7l6-2m6 2l3-1m-3 1l-3 9a5.002 5.002 0 006.001 0M18 7l3 9m-3-9l-6-2m0-2v2m0 16V5m0 16H9m3 0h3"
								/>
							</svg>
							<h3
								class="font-semibold {privacySettings.privacy_level === 'balanced'
									? 'text-primary-900'
									: 'text-gray-900'}"
							>
								Balanced
							</h3>
						</div>
						<p class="text-sm text-gray-600">
							Cloud LLM with PII filtering. Full automation for convenience.
						</p>
					</button>

					<!-- Custom preset -->
					<button
						onclick={() => handleSetPrivacyLevel('custom')}
						class="rounded-lg border p-4 text-left transition-all {privacySettings.privacy_level ===
						'custom'
							? 'border-primary-600 bg-primary-50 ring-2 ring-primary-600'
							: 'border-gray-200 bg-white hover:border-gray-300'}"
					>
						<div class="mb-2 flex items-center gap-2">
							<svg
								class="h-5 w-5 {privacySettings.privacy_level === 'custom'
									? 'text-primary-600'
									: 'text-gray-600'}"
								fill="none"
								viewBox="0 0 24 24"
								stroke="currentColor"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
								/>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
								/>
							</svg>
							<h3
								class="font-semibold {privacySettings.privacy_level === 'custom'
									? 'text-primary-900'
									: 'text-gray-900'}"
							>
								Custom
							</h3>
						</div>
						<p class="text-sm text-gray-600">
							Fine-grained control over each feature. Configure below.
						</p>
					</button>
				</div>

				<!-- Custom Feature Flags Editor (only visible when Custom level is active) -->
				{#if privacySettings.privacy_level === 'custom'}
					<div class="mt-6 rounded-lg border border-gray-200 bg-white p-4">
						<h3 class="mb-4 font-medium text-gray-900">Custom Feature Configuration</h3>
						<div class="space-y-3">
							<label class="flex cursor-pointer items-center justify-between">
								<div>
									<span class="text-sm font-medium text-gray-700">Local LLM Assistance</span>
									<p class="text-xs text-gray-500">
										Enable Ollama/LM Studio for form filling and email drafting
									</p>
								</div>
								<input
									type="checkbox"
									checked={privacySettings.feature_flags.allow_local_llm}
									onchange={(e) =>
										handleUpdateFeatureFlag('allow_local_llm', e.currentTarget.checked)}
									class="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
								/>
							</label>

							<label class="flex cursor-pointer items-center justify-between">
								<div>
									<span class="text-sm font-medium text-gray-700">Cloud LLM Assistance</span>
									<p class="text-xs text-gray-500">
										Enable OpenAI/Claude/Gemini with PII filtering
									</p>
								</div>
								<input
									type="checkbox"
									checked={privacySettings.feature_flags.allow_cloud_llm}
									onchange={(e) =>
										handleUpdateFeatureFlag('allow_cloud_llm', e.currentTarget.checked)}
									class="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
								/>
							</label>

							<label class="flex cursor-pointer items-center justify-between">
								<div>
									<span class="text-sm font-medium text-gray-700">Browser Automation</span>
									<p class="text-xs text-gray-500">
										Auto-fill opt-out forms using browser automation
									</p>
								</div>
								<input
									type="checkbox"
									checked={privacySettings.feature_flags.allow_browser_automation}
									onchange={(e) =>
										handleUpdateFeatureFlag('allow_browser_automation', e.currentTarget.checked)}
									class="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
								/>
							</label>

							<label class="flex cursor-pointer items-center justify-between">
								<div>
									<span class="text-sm font-medium text-gray-700">Email Sending</span>
									<p class="text-xs text-gray-500">Send opt-out emails via SMTP</p>
								</div>
								<input
									type="checkbox"
									checked={privacySettings.feature_flags.allow_email_sending}
									onchange={(e) =>
										handleUpdateFeatureFlag('allow_email_sending', e.currentTarget.checked)}
									class="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
								/>
							</label>

							<label class="flex cursor-pointer items-center justify-between">
								<div>
									<span class="text-sm font-medium text-gray-700">IMAP Monitoring</span>
									<p class="text-xs text-gray-500">
										Scan inbox for marketing emails, broker confirmations, and account discovery
									</p>
								</div>
								<input
									type="checkbox"
									checked={privacySettings.feature_flags.allow_imap_monitoring}
									onchange={(e) =>
										handleUpdateFeatureFlag('allow_imap_monitoring', e.currentTarget.checked)}
									class="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
								/>
							</label>

							<label class="flex cursor-pointer items-center justify-between">
								<div>
									<span class="text-sm font-medium text-gray-700">PII Scanning</span>
									<p class="text-xs text-gray-500">
										Scan for and filter sensitive data before cloud upload
									</p>
								</div>
								<input
									type="checkbox"
									checked={privacySettings.feature_flags.allow_pii_scanning}
									onchange={(e) =>
										handleUpdateFeatureFlag('allow_pii_scanning', e.currentTarget.checked)}
									class="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
								/>
							</label>
						</div>
					</div>
				{/if}
			{/if}
		</section>
	{:else if activeTab === 'llm'}
		<section>
			<h2 class="mb-4 text-lg font-semibold text-gray-800">LLM Providers</h2>
			{#if llmError}
				<div class="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-900">
					{llmError}
				</div>
			{/if}
			{#if loadingLlm}
				<p class="text-gray-500">Loading...</p>
			{:else if llmSettings}
				<div class="space-y-6">
					<!-- Provider Selection -->
					<div class="rounded-lg border border-gray-200 bg-white p-4">
						<h3 class="mb-4 font-medium text-gray-900">Provider Selection</h3>
						<div class="grid gap-4 md:grid-cols-3">
							<div>
								<label for="primary-provider" class="mb-1 block text-sm font-medium text-gray-700"
									>Primary Provider</label
								>
								<select
									id="primary-provider"
									value={llmSettings.primary_provider ?? ''}
									onchange={(e) => {
										const val = e.currentTarget.value;
										if (val) handleSetPrimaryProvider(val as LlmProvider);
									}}
									class="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
								>
									<option value="">None</option>
									<option value="ollama">Ollama (Local)</option>
									<option value="lm-studio">LM Studio (Local)</option>
									<option value="open-ai">OpenAI</option>
									<option value="claude">Claude</option>
									<option value="gemini">Gemini</option>
								</select>
							</div>
							<div>
								<label
									for="email-draft-provider"
									class="mb-1 block text-sm font-medium text-gray-700">Email Drafting</label
								>
								<select
									id="email-draft-provider"
									value={llmSettings.email_draft_provider ?? ''}
									onchange={(e) => {
										const val = e.currentTarget.value;
										if (val) handleSetTaskProvider('email-draft', val as LlmProvider);
									}}
									class="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
								>
									<option value="">Use Primary</option>
									<option value="ollama">Ollama (Local)</option>
									<option value="lm-studio">LM Studio (Local)</option>
									<option value="open-ai">OpenAI</option>
									<option value="claude">Claude</option>
									<option value="gemini">Gemini</option>
								</select>
							</div>
							<div>
								<label for="form-fill-provider" class="mb-1 block text-sm font-medium text-gray-700"
									>Form Filling</label
								>
								<select
									id="form-fill-provider"
									value={llmSettings.form_fill_provider ?? ''}
									onchange={(e) => {
										const val = e.currentTarget.value;
										if (val) handleSetTaskProvider('form-fill', val as LlmProvider);
									}}
									class="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
								>
									<option value="">Use Primary</option>
									<option value="ollama">Ollama (Local)</option>
									<option value="lm-studio">LM Studio (Local)</option>
									<option value="open-ai">OpenAI</option>
									<option value="claude">Claude</option>
									<option value="gemini">Gemini</option>
								</select>
							</div>
						</div>
					</div>

					<!-- Ollama Configuration -->
					<div class="rounded-lg border border-gray-200 bg-white p-4">
						<div class="mb-3 flex items-center gap-2">
							<h3 class="font-medium text-gray-900">Ollama (Local)</h3>
							{#if llmTestResults.ollama === 'success'}
								<span class="text-xs text-green-600">Connected</span>
							{:else if llmTestResults.ollama === 'error'}
								<span class="text-xs text-red-600">Error</span>
							{/if}
						</div>
						<p class="mb-3 text-sm text-gray-600">
							Local LLM via Ollama. Requires Ollama running on localhost:11434.
						</p>
						<button
							onclick={() => handleTestProvider('ollama')}
							disabled={llmTestResults.ollama === 'testing'}
							class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
						>
							{llmTestResults.ollama === 'testing' ? 'Testing...' : 'Test Connection'}
						</button>
						{#if llmTestMessages.ollama}
							<p
								class="mt-2 text-sm {llmTestResults.ollama === 'success'
									? 'text-green-600'
									: 'text-red-600'}"
							>
								{llmTestMessages.ollama}
							</p>
						{/if}
					</div>

					<!-- LM Studio Configuration -->
					<div class="rounded-lg border border-gray-200 bg-white p-4">
						<div class="mb-3 flex items-center gap-2">
							<h3 class="font-medium text-gray-900">LM Studio (Local)</h3>
							{#if llmTestResults['lm-studio'] === 'success'}
								<span class="text-xs text-green-600">Connected</span>
							{:else if llmTestResults['lm-studio'] === 'error'}
								<span class="text-xs text-red-600">Error</span>
							{/if}
						</div>
						<p class="mb-3 text-sm text-gray-600">
							Local LLM via LM Studio. Requires LM Studio server running on localhost:1234.
						</p>
						<button
							onclick={() => handleTestProvider('lm-studio')}
							disabled={llmTestResults['lm-studio'] === 'testing'}
							class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
						>
							{llmTestResults['lm-studio'] === 'testing' ? 'Testing...' : 'Test Connection'}
						</button>
						{#if llmTestMessages['lm-studio']}
							<p
								class="mt-2 text-sm {llmTestResults['lm-studio'] === 'success'
									? 'text-green-600'
									: 'text-red-600'}"
							>
								{llmTestMessages['lm-studio']}
							</p>
						{/if}
					</div>

					<!-- OpenAI Configuration -->
					<div class="rounded-lg border border-gray-200 bg-white p-4">
						<div class="mb-3 flex items-center gap-2">
							<h3 class="font-medium text-gray-900">OpenAI</h3>
							{#if llmSettings.has_openai_key}
								<span class="text-xs text-green-600">API Key Saved</span>
							{/if}
							{#if llmTestResults['open-ai'] === 'success'}
								<span class="text-xs text-green-600">Connected</span>
							{:else if llmTestResults['open-ai'] === 'error'}
								<span class="text-xs text-red-600">Error</span>
							{/if}
						</div>
						<p class="mb-3 text-sm text-gray-600">
							Cloud LLM via OpenAI API. Requires API key from platform.openai.com.
						</p>
						<div class="flex gap-2">
							<input
								type="password"
								bind:value={apiKeys['open-ai']}
								autocomplete="off"
								placeholder={llmSettings.has_openai_key ? 'API key is saved' : 'Enter API key'}
								class="flex-1 rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							/>
							<button
								onclick={() => handleSaveApiKey('open-ai')}
								disabled={!apiKeys['open-ai']}
								class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
							>
								Save
							</button>
							<button
								onclick={() => handleTestProvider('open-ai')}
								disabled={llmTestResults['open-ai'] === 'testing' || !llmSettings.has_openai_key}
								class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
							>
								{llmTestResults['open-ai'] === 'testing' ? 'Testing...' : 'Test'}
							</button>
						</div>
						{#if llmTestMessages['open-ai']}
							<p
								class="mt-2 text-sm {llmTestResults['open-ai'] === 'success'
									? 'text-green-600'
									: 'text-red-600'}"
							>
								{llmTestMessages['open-ai']}
							</p>
						{/if}
					</div>

					<!-- Claude Configuration -->
					<div class="rounded-lg border border-gray-200 bg-white p-4">
						<div class="mb-3 flex items-center gap-2">
							<h3 class="font-medium text-gray-900">Claude (Anthropic)</h3>
							{#if llmSettings.has_claude_key}
								<span class="text-xs text-green-600">API Key Saved</span>
							{/if}
							{#if llmTestResults.claude === 'success'}
								<span class="text-xs text-green-600">Connected</span>
							{:else if llmTestResults.claude === 'error'}
								<span class="text-xs text-red-600">Error</span>
							{/if}
						</div>
						<p class="mb-3 text-sm text-gray-600">
							Cloud LLM via Anthropic API. Requires API key from console.anthropic.com.
						</p>
						<div class="flex gap-2">
							<input
								type="password"
								bind:value={apiKeys.claude}
								autocomplete="off"
								placeholder={llmSettings.has_claude_key ? 'API key is saved' : 'Enter API key'}
								class="flex-1 rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							/>
							<button
								onclick={() => handleSaveApiKey('claude')}
								disabled={!apiKeys.claude}
								class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
							>
								Save
							</button>
							<button
								onclick={() => handleTestProvider('claude')}
								disabled={llmTestResults.claude === 'testing' || !llmSettings.has_claude_key}
								class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
							>
								{llmTestResults.claude === 'testing' ? 'Testing...' : 'Test'}
							</button>
						</div>
						{#if llmTestMessages.claude}
							<p
								class="mt-2 text-sm {llmTestResults.claude === 'success'
									? 'text-green-600'
									: 'text-red-600'}"
							>
								{llmTestMessages.claude}
							</p>
						{/if}
					</div>

					<!-- Gemini Configuration -->
					<div class="rounded-lg border border-gray-200 bg-white p-4">
						<div class="mb-3 flex items-center gap-2">
							<h3 class="font-medium text-gray-900">Gemini (Google)</h3>
							{#if llmSettings.has_gemini_key}
								<span class="text-xs text-green-600">API Key Saved</span>
							{/if}
							{#if llmTestResults.gemini === 'success'}
								<span class="text-xs text-green-600">Connected</span>
							{:else if llmTestResults.gemini === 'error'}
								<span class="text-xs text-red-600">Error</span>
							{/if}
						</div>
						<p class="mb-3 text-sm text-gray-600">
							Cloud LLM via Google AI API. Requires API key from aistudio.google.com.
						</p>
						<div class="flex gap-2">
							<input
								type="password"
								bind:value={apiKeys.gemini}
								autocomplete="off"
								placeholder={llmSettings.has_gemini_key ? 'API key is saved' : 'Enter API key'}
								class="flex-1 rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							/>
							<button
								onclick={() => handleSaveApiKey('gemini')}
								disabled={!apiKeys.gemini}
								class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
							>
								Save
							</button>
							<button
								onclick={() => handleTestProvider('gemini')}
								disabled={llmTestResults.gemini === 'testing' || !llmSettings.has_gemini_key}
								class="rounded-lg bg-primary-600 px-4 py-2 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
							>
								{llmTestResults.gemini === 'testing' ? 'Testing...' : 'Test'}
							</button>
						</div>
						{#if llmTestMessages.gemini}
							<p
								class="mt-2 text-sm {llmTestResults.gemini === 'success'
									? 'text-green-600'
									: 'text-red-600'}"
							>
								{llmTestMessages.gemini}
							</p>
						{/if}
					</div>
				</div>
			{/if}
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
								autocomplete="off"
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
						<h3 class="font-medium text-gray-900">IMAP Email Monitoring</h3>
						<p class="text-sm text-gray-500">
							Scan inbox for marketing emails, broker confirmations, and data exposure
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
								autocomplete="off"
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

			<!-- Email Setup Help Section -->
			<div class="rounded-lg border border-blue-200 bg-blue-50 p-4">
				<button
					onclick={() => (showEmailHelp = !showEmailHelp)}
					class="flex w-full items-center justify-between text-left"
				>
					<div class="flex items-center gap-2">
						<svg
							class="h-5 w-5 text-blue-600"
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
						<h3 class="font-medium text-blue-900">Email Setup Guide & What IMAP Does</h3>
					</div>
					<svg
						class="h-5 w-5 text-blue-600 transition-transform {showEmailHelp ? 'rotate-180' : ''}"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M19 9l-7 7-7-7"
						/>
					</svg>
				</button>

				{#if showEmailHelp}
					<div class="mt-4 space-y-4">
						<!-- What IMAP Does -->
						<div class="rounded-lg border border-blue-300 bg-white p-3">
							<h4 class="mb-2 font-medium text-gray-900">What IMAP Monitoring Does</h4>
							<div class="space-y-2 text-sm text-gray-700">
								<p>
									<strong>IMAP</strong> allows Spectral to scan your inbox and identify companies that
									have your data:
								</p>
								<ul class="ml-4 list-disc space-y-1">
									<li>
										<strong>Marketing & Promotional Emails:</strong> Identifies companies sending you
										marketing emails and automatically submits opt-out/deletion requests
									</li>
									<li>
										<strong>Broker Confirmations:</strong> Detects confirmation emails when data brokers
										respond to removal requests
									</li>
									<li>
										<strong>Account Discovery:</strong> Finds services and websites where you have accounts,
										even if you forgot about them
									</li>
									<li>
										<strong>Data Exposure Alerts:</strong> Identifies potential data leaks or unexpected
										companies that have your information
									</li>
								</ul>
								<div class="mt-2 rounded-lg bg-green-50 p-2">
									<p class="text-xs text-green-800">
										<strong>Privacy Note:</strong> Email scanning happens locally on your device. Only
										sender domains and metadata are analyzed - message content stays private and encrypted.
									</p>
								</div>
							</div>
						</div>

						<p class="text-sm text-blue-800">
							Most email providers require special setup for third-party applications. Click on your
							provider below for detailed instructions.
						</p>

						<!-- Gmail Instructions -->
						<EmailProviderAccordion
							name="Gmail"
							isExpanded={expandedProvider === 'gmail'}
							onToggle={() => (expandedProvider = expandedProvider === 'gmail' ? null : 'gmail')}
						>
							{#snippet children()}
								<EmailSettings
									settings={[
										{ label: 'SMTP Host', value: 'smtp.gmail.com' },
										{ label: 'SMTP Port', value: '587 (TLS)' },
										{ label: 'IMAP Host', value: 'imap.gmail.com' },
										{ label: 'IMAP Port', value: '993 (SSL)' }
									]}
								/>
								<div>
									<p class="font-medium text-gray-900">Setup Steps:</p>
									<ol class="ml-4 mt-1 list-decimal space-y-2">
										<li>
											<strong>Enable 2-Factor Authentication:</strong>
											<ul class="ml-4 mt-1 list-disc">
												<li>
													Go to <a
														href="https://myaccount.google.com/security"
														target="_blank"
														class="text-blue-600 hover:underline">Google Account Security</a
													>
												</li>
												<li>Click "2-Step Verification" and follow the setup</li>
											</ul>
										</li>
										<li>
											<strong>Generate App Password:</strong>
											<ul class="ml-4 mt-1 list-disc">
												<li>
													Go to <a
														href="https://myaccount.google.com/apppasswords"
														target="_blank"
														class="text-blue-600 hover:underline">App Passwords</a
													>
												</li>
												<li>Select "Mail" and "Other (Custom name)"</li>
												<li>Enter "Spectral Privacy" as the name</li>
												<li>Copy the 16-character password (no spaces)</li>
											</ul>
										</li>
										<li>
											<strong>Enable IMAP:</strong>
											<ul class="ml-4 mt-1 list-disc">
												<li>
													Go to <a
														href="https://mail.google.com/mail/u/0/#settings/fwdandpop"
														target="_blank"
														class="text-blue-600 hover:underline"
														>Gmail Settings → Forwarding and POP/IMAP</a
													>
												</li>
												<li>Click "Enable IMAP" and save changes</li>
											</ul>
										</li>
										<li>
											<strong>Configure Spectral:</strong>
											<ul class="ml-4 mt-1 list-disc">
												<li>Use your Gmail address as username</li>
												<li>Use the app password (NOT your regular Gmail password)</li>
											</ul>
										</li>
									</ol>
								</div>
							{/snippet}
						</EmailProviderAccordion>

						<!-- Outlook/Hotmail Instructions -->
						<EmailProviderAccordion
							name="Outlook / Hotmail"
							isExpanded={expandedProvider === 'outlook'}
							onToggle={() =>
								(expandedProvider = expandedProvider === 'outlook' ? null : 'outlook')}
						>
							{#snippet children()}
								<EmailSettings
									settings={[
										{ label: 'SMTP Host', value: ['smtp-mail.outlook.com', 'smtp.office365.com'] },
										{ label: 'SMTP Port', value: '587 (TLS)' },
										{ label: 'IMAP Host', value: 'outlook.office365.com' },
										{ label: 'IMAP Port', value: '993 (SSL)' }
									]}
								/>
								<div>
									<p class="font-medium text-gray-900">Setup Steps:</p>
									<ol class="ml-4 mt-1 list-decimal space-y-2">
										<li>
											<strong>Enable 2FA (Recommended):</strong>
											<ul class="ml-4 mt-1 list-disc">
												<li>
													Go to <a
														href="https://account.microsoft.com/security"
														target="_blank"
														class="text-blue-600 hover:underline">Microsoft Account Security</a
													>
												</li>
												<li>Set up two-step verification</li>
											</ul>
										</li>
										<li>
											<strong>Generate App Password:</strong>
											<ul class="ml-4 mt-1 list-disc">
												<li>Go to Security settings → Advanced security options</li>
												<li>Under "App passwords", create a new password</li>
												<li>Use this app password in Spectral</li>
											</ul>
										</li>
										<li>
											<strong>Configure Spectral:</strong>
											<ul class="ml-4 mt-1 list-disc">
												<li>Username: your full Outlook/Hotmail email address</li>
												<li>Password: the app password (or regular password if 2FA not enabled)</li>
											</ul>
										</li>
									</ol>
								</div>
							{/snippet}
						</EmailProviderAccordion>

						<!-- Yahoo Instructions -->
						<EmailProviderAccordion
							name="Yahoo Mail"
							isExpanded={expandedProvider === 'yahoo'}
							onToggle={() => (expandedProvider = expandedProvider === 'yahoo' ? null : 'yahoo')}
						>
							{#snippet children()}
								<EmailSettings
									settings={[
										{ label: 'SMTP Host', value: 'smtp.mail.yahoo.com' },
										{ label: 'SMTP Port', value: '587 (TLS)' },
										{ label: 'IMAP Host', value: 'imap.mail.yahoo.com' },
										{ label: 'IMAP Port', value: '993 (SSL)' }
									]}
								/>
								<div>
									<p class="font-medium text-gray-900">Setup Steps:</p>
									<ol class="ml-4 mt-1 list-decimal space-y-2">
										<li>
											<strong>Generate App Password:</strong>
											<ul class="ml-4 mt-1 list-disc">
												<li>
													Go to <a
														href="https://login.yahoo.com/account/security"
														target="_blank"
														class="text-blue-600 hover:underline">Yahoo Account Security</a
													>
												</li>
												<li>Click "Generate app password"</li>
												<li>Select "Other App" and name it "Spectral Privacy"</li>
												<li>Copy the generated password</li>
											</ul>
										</li>
										<li>
											<strong>Configure Spectral:</strong>
											<ul class="ml-4 mt-1 list-disc">
												<li>Username: your Yahoo email address</li>
												<li>Password: the app password (NOT your regular Yahoo password)</li>
											</ul>
										</li>
									</ol>
								</div>
								<div class="rounded-lg bg-yellow-50 p-2">
									<p class="text-xs text-yellow-800">
										<strong>Note:</strong> Yahoo requires app passwords for all third-party applications.
										You cannot use your regular password.
									</p>
								</div>
							{/snippet}
						</EmailProviderAccordion>

						<!-- General/Other Providers -->
						<EmailProviderAccordion
							name="Other Email Providers"
							isExpanded={expandedProvider === 'other'}
							onToggle={() => (expandedProvider = expandedProvider === 'other' ? null : 'other')}
						>
							{#snippet children()}
								<p>
									For other email providers, you'll need to find their SMTP and IMAP settings.
									Common steps:
								</p>
								<ol class="ml-4 list-decimal space-y-2">
									<li>Search for "[Your Provider] SMTP IMAP settings" online</li>
									<li>Look for SSL/TLS enabled ports (usually 587 for SMTP, 993 for IMAP)</li>
									<li>Check if your provider requires app-specific passwords</li>
									<li>
										Some providers may need "Allow less secure apps" enabled (not recommended)
									</li>
								</ol>
								<div class="rounded-lg bg-blue-50 p-2">
									<p class="text-xs text-blue-800">
										<strong>Common IMAP Ports:</strong> 993 (SSL), 143 (STARTTLS)<br />
										<strong>Common SMTP Ports:</strong> 587 (TLS), 465 (SSL), 25 (unencrypted - not recommended)
									</p>
								</div>
							{/snippet}
						</EmailProviderAccordion>
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
			<div class="mb-4 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-gray-800">Privacy Audit Log</h2>
				<button
					onclick={handleClearAuditLog}
					class="rounded-md border border-red-300 bg-white px-3 py-1 text-sm text-red-600 hover:bg-red-50"
				>
					Clear Log
				</button>
			</div>

			<p class="mb-4 text-sm text-gray-600">
				Track all privacy-related actions, including scans, removals, and data access events.
			</p>

			{#if auditError}
				<div class="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-900">
					{auditError}
				</div>
			{/if}

			<!-- Filter controls -->
			<div class="mb-4 flex items-center gap-2">
				<label for="audit-filter" class="text-sm font-medium text-gray-700">Filter:</label>
				<select
					id="audit-filter"
					bind:value={auditFilter}
					onchange={() => loadAuditLog()}
					class="rounded-md border border-gray-300 px-3 py-1 text-sm"
				>
					<option value="all">All Events</option>
					<option value="ScanStarted">Scans Started</option>
					<option value="FindingCreated">Findings Created</option>
					<option value="RemovalSubmitted">Removals Submitted</option>
					<option value="DataAccessed">Data Accessed</option>
					<option value="VaultUnlocked">Vault Unlocked</option>
				</select>
			</div>

			{#if loadingAudit}
				<div class="rounded-lg border border-gray-200 bg-white p-8 text-center">
					<div
						class="mx-auto h-8 w-8 animate-spin rounded-full border-4 border-gray-200 border-t-primary-600"
					></div>
					<p class="mt-2 text-sm text-gray-500">Loading audit log...</p>
				</div>
			{:else if auditEntries.length === 0}
				<div class="rounded-lg border border-gray-200 bg-gray-50 p-8 text-center">
					<svg
						class="mx-auto mb-4 h-12 w-12 text-gray-400"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
						/>
					</svg>
					<h3 class="mb-2 text-lg font-medium text-gray-900">No Audit Entries</h3>
					<p class="text-sm text-gray-600">
						Audit log entries will appear here as you use Spectral.
					</p>
				</div>
			{:else}
				<div class="space-y-2">
					{#each auditEntries as entry (entry.id)}
						<div class="rounded-lg border border-gray-200 bg-white p-4 hover:bg-gray-50">
							<div class="flex items-start justify-between gap-4">
								<div class="flex-1">
									<div class="mb-1 flex items-center gap-2">
										<span
											class="rounded-full px-2 py-1 text-xs font-medium {getEventTypeColor(
												entry.event_type
											)}"
										>
											{entry.event_type}
										</span>
										<span class="text-xs text-gray-500">{formatTimestamp(entry.timestamp)}</span>
									</div>
									<p class="text-sm text-gray-900">{entry.subject}</p>
									<div class="mt-2 flex flex-wrap items-center gap-4 text-xs text-gray-600">
										<div class="flex items-center gap-1">
											<span class="font-medium">Destination:</span>
											<span>{entry.data_destination}</span>
										</div>
										<div class="flex items-center gap-1">
											<span class="font-medium">Outcome:</span>
											<span
												class="rounded px-1 {entry.outcome === 'Allowed'
													? 'bg-green-100 text-green-700'
													: 'bg-red-100 text-red-700'}">{entry.outcome}</span
											>
										</div>
										{#if entry.pii_fields && entry.pii_fields.length > 0}
											<div class="flex items-center gap-1">
												<span class="font-medium">PII Fields:</span>
												<span>{entry.pii_fields.join(', ')}</span>
											</div>
										{/if}
									</div>
								</div>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</section>
	{/if}
</div>
