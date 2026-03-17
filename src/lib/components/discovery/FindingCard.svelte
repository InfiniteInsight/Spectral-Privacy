<script lang="ts">
	import type { DiscoveryFinding } from '$lib/api/discovery';

	interface Props {
		finding: DiscoveryFinding;
		// eslint-disable-next-line no-unused-vars
		onMarkFixed: (id: string) => void;
		// eslint-disable-next-line no-unused-vars
		onIgnore: (id: string) => void;
		// eslint-disable-next-line no-unused-vars
		onDelete: (id: string, path: string) => void;
		// eslint-disable-next-line no-unused-vars
		onOpenLocation: (path: string) => void;
	}

	let { finding, onMarkFixed, onIgnore, onDelete, onOpenLocation }: Props = $props();
	let showDeleteConfirm = $state(false);

	function riskClass(level: string): string {
		const classes: Record<string, string> = {
			critical: 'bg-red-100 text-red-800',
			high: 'bg-orange-100 text-orange-800',
			medium: 'bg-yellow-100 text-yellow-800',
			low: 'bg-blue-100 text-blue-800'
		};
		return classes[level] || 'bg-gray-100 text-gray-800';
	}

	function formatDate(iso: string): string {
		try {
			return new Date(iso).toLocaleDateString();
		} catch {
			return iso;
		}
	}
</script>

<div class="rounded-lg border border-gray-200 bg-white p-4">
	<div class="flex items-center gap-2 mb-2 flex-wrap">
		<span class="rounded-full px-2 py-0.5 text-xs font-medium {riskClass(finding.risk_level)}"
			>{finding.risk_level}</span
		>
		<span class="rounded-full px-2 py-0.5 text-xs font-medium bg-purple-100 text-purple-800"
			>{finding.pii_type}</span
		>
		{#if finding.remediated}<span
				class="rounded-full px-2 py-0.5 text-xs font-medium bg-green-100 text-green-800">Fixed</span
			>{/if}
		{#if finding.ignored}<span
				class="rounded-full px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-800">Ignored</span
			>{/if}
		{#if finding.still_present_after_remediation}<span
				class="rounded-full px-2 py-0.5 text-xs font-medium bg-orange-100 text-orange-800"
				>Still Present</span
			>{/if}
	</div>

	<p class="text-sm font-medium text-gray-900 truncate" title={finding.source_detail}>
		{finding.source_detail}
	</p>

	{#if finding.matched_value || finding.line_number}
		<div class="mt-2 p-2 rounded bg-gray-50 font-mono text-xs">
			{#if finding.line_number}<span class="text-gray-500">Line {finding.line_number}:</span>{/if}
			{#if finding.matched_value}<span class="text-gray-900 ml-1">{finding.matched_value}</span
				>{/if}
		</div>
	{/if}

	<p class="mt-2 text-xs text-gray-400">Found {formatDate(finding.found_at)}</p>

	{#if !finding.remediated || finding.still_present_after_remediation}
		<div class="mt-3 pt-3 border-t border-gray-100 flex flex-wrap gap-2">
			<button
				onclick={() => onOpenLocation(finding.source_detail)}
				class="px-3 py-1.5 text-xs font-medium rounded-md bg-blue-50 text-blue-700 hover:bg-blue-100"
				>Open Location</button
			>
			<button
				onclick={() => onMarkFixed(finding.id)}
				class="px-3 py-1.5 text-xs font-medium rounded-md bg-green-50 text-green-700 hover:bg-green-100"
				>Mark Fixed</button
			>
			<button
				onclick={() => onIgnore(finding.id)}
				class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-50 text-gray-700 hover:bg-gray-100"
				>Ignore</button
			>
			{#if !showDeleteConfirm}
				<button
					onclick={() => (showDeleteConfirm = true)}
					class="px-3 py-1.5 text-xs font-medium rounded-md bg-red-50 text-red-700 hover:bg-red-100"
					>Delete File</button
				>
			{:else}
				<div class="flex items-center gap-2 p-2 rounded bg-red-50">
					<span class="text-xs text-red-700">Delete?</span>
					<button
						onclick={() => {
							onDelete(finding.id, finding.source_detail);
							showDeleteConfirm = false;
						}}
						class="px-2 py-1 text-xs rounded bg-red-600 text-white">Yes</button
					>
					<button
						onclick={() => (showDeleteConfirm = false)}
						class="px-2 py-1 text-xs rounded bg-gray-200 text-gray-700">No</button
					>
				</div>
			{/if}
		</div>
	{/if}
</div>
