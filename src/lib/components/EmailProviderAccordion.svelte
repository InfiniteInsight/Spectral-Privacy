<script lang="ts">
	interface Props {
		name: string;
		isExpanded: boolean;
		onToggle: () => void;
		children: any;
	}

	let { name, isExpanded, onToggle, children }: Props = $props();
</script>

<div class="rounded-lg border border-blue-300 bg-white">
	<button onclick={onToggle} class="flex w-full items-center justify-between p-3 text-left">
		<span class="font-medium text-gray-900">{name}</span>
		<svg
			class="h-4 w-4 text-gray-600 transition-transform {isExpanded ? 'rotate-180' : ''}"
			fill="none"
			viewBox="0 0 24 24"
			stroke="currentColor"
		>
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
		</svg>
	</button>
	{#if isExpanded}
		<div class="border-t border-blue-200 p-3">
			<div class="space-y-3 text-sm text-gray-700">
				{@render children()}
			</div>
		</div>
	{/if}
</div>
