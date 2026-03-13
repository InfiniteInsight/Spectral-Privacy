<script lang="ts">
	import type { EmailFallback } from '$lib/api/brokers';

	interface Props {
		emailFallback: EmailFallback;
	}

	let { emailFallback }: Props = $props();
</script>

<div class="mb-8 p-6 border-2 border-orange-300 rounded-lg bg-orange-50">
	<h2 class="text-xl font-bold text-orange-900 mb-2 flex items-center gap-2">
		<span class="text-2xl">📧</span>
		Email Fallback Available
	</h2>
	<p class="text-sm text-orange-800 mb-4">
		If the web form doesn't work or you don't receive a response within {emailFallback.processing_days}
		{emailFallback.processing_days === 1 ? 'day' : 'days'}, you can send an email removal request
		to:
	</p>

	<!-- Email Address -->
	<div class="mb-4">
		<label class="text-sm font-medium text-gray-700">Email:</label>
		<div
			class="mt-1 p-3 bg-white border border-gray-300 rounded font-mono text-sm flex items-center justify-between"
		>
			<span>{emailFallback.email}</span>
			<button
				onclick={() => navigator.clipboard.writeText(emailFallback.email)}
				class="px-3 py-1 text-xs bg-orange-600 text-white rounded hover:bg-orange-700"
			>
				Copy
			</button>
		</div>
	</div>

	<!-- Phone Numbers -->
	{#if emailFallback.phone || emailFallback.ccpa_phone}
		<div class="mb-4 space-y-2">
			{#if emailFallback.phone}
				<div>
					<label class="text-sm font-medium text-gray-700">Phone:</label>
					<div class="mt-1 p-3 bg-white border border-gray-300 rounded font-mono text-sm">
						{emailFallback.phone}
					</div>
				</div>
			{/if}
			{#if emailFallback.ccpa_phone}
				<div>
					<label class="text-sm font-medium text-gray-700">CCPA Phone (CA residents):</label>
					<div class="mt-1 p-3 bg-white border border-gray-300 rounded font-mono text-sm">
						{emailFallback.ccpa_phone}
					</div>
				</div>
			{/if}
		</div>
	{/if}

	<!-- Subject Line -->
	{#if emailFallback.subject}
		<div class="mb-4">
			<label class="text-sm font-medium text-gray-700">
				Subject:
				{#if emailFallback.subject_required}
					<span class="text-red-600">*</span>
					<span class="text-xs text-red-600">(Must match exactly)</span>
				{/if}
			</label>
			<div
				class="mt-1 p-3 bg-white border border-gray-300 rounded font-mono text-sm flex items-center justify-between"
			>
				<span>{emailFallback.subject}</span>
				<button
					onclick={() => navigator.clipboard.writeText(emailFallback.subject || '')}
					class="px-3 py-1 text-xs bg-orange-600 text-white rounded hover:bg-orange-700"
				>
					Copy
				</button>
			</div>
		</div>
	{/if}

	<!-- Required Fields -->
	<div class="mb-4">
		<label class="text-sm font-medium text-gray-700">Include in Email Body:</label>
		<div class="mt-1 p-3 bg-white border border-gray-300 rounded text-sm">
			<ul class="list-disc list-inside space-y-1">
				{#each emailFallback.required_fields as field}
					<li class="text-gray-700">
						{field.replace(/_/g, ' ').replace(/\b\w/g, (l) => l.toUpperCase())}
					</li>
				{/each}
			</ul>
		</div>
	</div>

	<!-- Notes -->
	{#if emailFallback.notes}
		<div class="mb-4 p-4 bg-blue-50 border border-blue-200 rounded">
			<h4 class="font-medium text-blue-900 mb-2">Important Notes:</h4>
			<p class="text-sm text-blue-800 whitespace-pre-wrap">
				{emailFallback.notes}
			</p>
		</div>
	{/if}

	<!-- Network Note -->
	{#if emailFallback.network_note}
		<div class="p-4 bg-green-50 border border-green-200 rounded">
			<h4 class="font-medium text-green-900 mb-2">✨ Bonus:</h4>
			<p class="text-sm text-green-800 whitespace-pre-wrap">
				{emailFallback.network_note}
			</p>
		</div>
	{/if}

	<!-- CCPA/GDPR Compliance Badges -->
	<div class="mt-4 flex gap-2">
		{#if emailFallback.ccpa_compliant}
			<span class="px-2 py-1 bg-blue-100 text-blue-700 text-xs rounded font-medium">
				CCPA Compliant
			</span>
		{/if}
		{#if emailFallback.gdpr_compliant}
			<span class="px-2 py-1 bg-blue-100 text-blue-700 text-xs rounded font-medium">
				GDPR Compliant
			</span>
		{/if}
	</div>
</div>
