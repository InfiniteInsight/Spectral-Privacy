<script lang="ts">
	import type { ProfileCompleteness } from '$lib/api/profile';

	interface Props {
		completeness: ProfileCompleteness;
	}

	let { completeness }: Props = $props();

	const tierConfig = {
		Minimal: {
			color: 'bg-red-100 text-red-800 border-red-200',
			barColor: 'bg-red-500',
			icon: '\u26A0\uFE0F'
		},
		Basic: {
			color: 'bg-yellow-100 text-yellow-800 border-yellow-200',
			barColor: 'bg-yellow-500',
			icon: '\uD83D\uDCDD'
		},
		Good: {
			color: 'bg-blue-100 text-blue-800 border-blue-200',
			barColor: 'bg-blue-500',
			icon: '\uD83D\uDC4D'
		},
		Excellent: {
			color: 'bg-green-100 text-green-800 border-green-200',
			barColor: 'bg-green-500',
			icon: '\u2728'
		}
	};

	const config = $derived(tierConfig[completeness.tier]);
</script>

<div class="border rounded-lg p-4 {config.color}">
	<div class="flex items-start gap-3">
		<span class="text-2xl" aria-hidden="true">{config.icon}</span>
		<div class="flex-1">
			<div class="flex items-center justify-between mb-2">
				<h3 class="font-semibold">Profile Completeness</h3>
				<span class="text-sm font-medium">{completeness.percentage}%</span>
			</div>
			<div
				class="w-full bg-white/50 rounded-full h-2 mb-2"
				role="progressbar"
				aria-valuenow={completeness.percentage}
				aria-valuemin={0}
				aria-valuemax={100}
			>
				<div
					class="{config.barColor} h-2 rounded-full transition-all duration-300"
					style:width="{completeness.percentage}%"
				/>
			</div>
			<p class="text-sm">{completeness.message}</p>
		</div>
	</div>
</div>
