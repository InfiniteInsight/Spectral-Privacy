<script lang="ts">
	interface Props {
		fieldName: string;
		phase?: string;
	}

	let { fieldName, phase = 'Phase 2' }: Props = $props();
</script>

<div class="disabled-field-notice">
	<div class="flex items-start gap-2 p-3 bg-gray-50 border border-gray-200 rounded-md">
		<svg class="w-5 h-5 text-gray-400 flex-shrink-0 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
			<path
				fill-rule="evenodd"
				d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z"
				clip-rule="evenodd"
			/>
		</svg>
		<div class="flex-1">
			<p class="text-sm font-medium text-gray-700">{fieldName}</p>
			<p class="text-xs text-gray-500 mt-0.5">Coming in {phase}</p>
		</div>
	</div>
</div>

<style>
	.disabled-field-notice {
		margin-bottom: 1rem;
	}
</style>
