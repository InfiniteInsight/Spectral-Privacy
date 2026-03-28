<script lang="ts">
	import type { BrokerDetail } from '$lib/api/brokers';
	import { removalAPI } from '$lib/api/removal';

	interface Props {
		broker: BrokerDetail;
		vaultId: string;
		profileId: string;
		onScanClick: () => void;
		disabled?: boolean;
		class?: string;
	}

	let {
		broker,
		vaultId,
		profileId,
		onScanClick,
		disabled = false,
		class: extraClass = ''
	}: Props = $props();

	let submitting = $state(false);
	let submitted = $state(false);
	let submitError = $state<string | null>(null);
	let showInstructions = $state(false);

	const isEmailMethod = $derived(broker.removal_method.startsWith('Email'));
	const isWebFormMethod = $derived(
		broker.removal_method.startsWith('WebForm') || broker.removal_method.startsWith('BrowserForm')
	);
	const isManualMethod = $derived(
		broker.removal_method.startsWith('Manual') || broker.removal_method.startsWith('Phone')
	);

	function buildMailtoUrl(): string {
		if (!broker.email_template) return `mailto:?subject=Data%20Removal%20Request`;
		const { email, subject, body } = broker.email_template;
		return `mailto:${encodeURIComponent(email)}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
	}

	async function handleFormSubmit() {
		submitting = true;
		submitError = null;
		try {
			await removalAPI.initiateDirectRemoval(vaultId, broker.id, profileId);
			submitted = true;
		} catch (err) {
			submitError = err instanceof Error ? err.message : String(err);
		} finally {
			submitting = false;
		}
	}
</script>

{#if broker.search_method_type === 'scannable'}
	<!-- Scannable broker: delegate to parent scan handler -->
	<button
		onclick={onScanClick}
		{disabled}
		class="flex-1 px-6 py-3 border-2 border-orange-600 text-orange-700 rounded-lg font-medium hover:bg-orange-50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed {extraClass}"
	>
		Scan This Company
	</button>
{:else if isEmailMethod}
	<!-- Email-method broker: open mailto: in user's email client -->
	{#if submitted}
		<p class="flex-1 px-6 py-3 text-green-700 font-medium text-center">
			✓ Email draft opened — send it from your email client
		</p>
	{:else}
		<a
			href={buildMailtoUrl()}
			class="flex-1 px-6 py-3 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 transition-colors text-center {extraClass}"
			onclick={() => (submitted = true)}
		>
			Send Removal Email
		</a>
	{/if}
{:else if isWebFormMethod}
	<!-- Web-form broker: trigger automated form submission -->
	{#if submitted}
		<p class="flex-1 px-6 py-3 text-green-700 font-medium text-center">
			✓ Form submission queued — check Removal History for progress
		</p>
	{:else if submitError}
		<div class="flex-1 flex flex-col gap-2">
			<p class="text-sm text-red-700">✗ {submitError}</p>
			<button
				onclick={handleFormSubmit}
				class="px-6 py-3 border-2 border-orange-600 text-orange-700 rounded-lg font-medium hover:bg-orange-50 transition-colors"
			>
				Retry
			</button>
		</div>
	{:else}
		<button
			onclick={handleFormSubmit}
			disabled={submitting || disabled}
			class="flex-1 px-6 py-3 bg-orange-600 text-white rounded-lg font-medium hover:bg-orange-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed {extraClass}"
		>
			{submitting ? 'Submitting...' : 'Submit Opt-Out Form'}
		</button>
	{/if}
{:else if isManualMethod}
	<!-- Manual/Phone: show instructions and link to privacy page -->
	<button
		onclick={() => (showInstructions = !showInstructions)}
		{disabled}
		class="flex-1 px-6 py-3 border-2 border-gray-400 text-gray-700 rounded-lg font-medium hover:bg-gray-50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed {extraClass}"
	>
		{showInstructions ? 'Hide Instructions' : 'View Removal Instructions'}
	</button>

	{#if showInstructions && broker.removal_action_url}
		<div class="w-full mt-3 p-4 bg-gray-50 border border-gray-200 rounded-lg">
			<p class="text-sm text-gray-700 mb-3">
				This company requires manual opt-out. Visit their privacy page:
			</p>
			<a
				href={broker.removal_action_url}
				target="_blank"
				rel="noopener noreferrer"
				class="inline-block px-4 py-2 bg-gray-700 text-white text-sm rounded hover:bg-gray-800 transition-colors"
			>
				Open Privacy / Opt-Out Page ↗
			</a>
		</div>
	{/if}
{:else}
	<!-- Fallback: link to broker site -->
	<a
		href={broker.url}
		target="_blank"
		rel="noopener noreferrer"
		class="flex-1 px-6 py-3 border-2 border-gray-400 text-gray-700 rounded-lg font-medium hover:bg-gray-50 transition-colors text-center {extraClass}"
	>
		Visit Site to Opt Out ↗
	</a>
{/if}
