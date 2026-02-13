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
