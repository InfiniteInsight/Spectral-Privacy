<script lang="ts">
	import type { EmailTemplate } from '$lib/api/brokers';
	import { substituteEmailVariables } from '$lib/utils/email-template';

	interface Props {
		template: EmailTemplate;
	}

	let { template }: Props = $props();

	const substituted = $derived(substituteEmailVariables(template));
</script>

<div class="mb-8 p-6 border-2 border-gray-200 rounded-lg bg-gray-50">
	<h2 class="text-xl font-bold text-gray-900 mb-4">Email Removal Template</h2>

	<div class="space-y-4">
		<!-- To -->
		<div>
			<label class="text-sm font-medium text-gray-700">To:</label>
			<div class="mt-1 p-3 bg-white border border-gray-300 rounded font-mono text-sm">
				{substituted.email}
			</div>
		</div>

		<!-- Subject -->
		<div>
			<label class="text-sm font-medium text-gray-700">Subject:</label>
			<div class="mt-1 p-3 bg-white border border-gray-300 rounded font-mono text-sm">
				{substituted.subject}
			</div>
		</div>

		<!-- Body -->
		<div>
			<label class="text-sm font-medium text-gray-700">Body:</label>
			<div
				class="mt-1 p-4 bg-white border border-gray-300 rounded font-mono text-sm whitespace-pre-wrap max-h-96 overflow-y-auto"
			>
				{substituted.body}
			</div>
		</div>

		<!-- Expected Response Time -->
		<div class="flex items-center gap-2 text-sm text-gray-600">
			<span class="font-medium">Expected Response:</span>
			<span>{template.response_days} {template.response_days === 1 ? 'day' : 'days'}</span>
		</div>

		<!-- Notes -->
		{#if template.notes}
			<div class="mt-4 p-4 bg-blue-50 border border-blue-200 rounded">
				<h4 class="font-medium text-blue-900 mb-2">Notes:</h4>
				<p class="text-sm text-blue-800 whitespace-pre-wrap">
					{template.notes}
				</p>
			</div>
		{/if}
	</div>
</div>
