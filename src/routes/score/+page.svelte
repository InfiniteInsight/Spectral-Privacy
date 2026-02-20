<script lang="ts">
	import { vaultStore } from '$lib/stores/vault.svelte';
	import { getPrivacyScore, type PrivacyScoreResult } from '$lib/api/score';

	let result = $state<PrivacyScoreResult | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);

	$effect(() => {
		const vid = vaultStore.currentVaultId;
		if (!vid) {
			loading = false;
			return;
		}
		loading = true;
		getPrivacyScore(vid)
			.then((d) => {
				result = d;
			})
			.catch((e) => {
				error = e instanceof Error ? e.message : String(e);
			})
			.finally(() => {
				loading = false;
			});
	});

	// SVG gauge helpers
	const SIZE = 200;
	const RADIUS = 80;
	const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

	function gaugeColor(score: number): string {
		if (score < 40) return '#ef4444'; // red
		if (score < 70) return '#f59e0b'; // amber
		if (score < 90) return '#22c55e'; // green
		return '#10b981'; // emerald
	}

	function strokeDashoffset(score: number): number {
		return CIRCUMFERENCE * (1 - score / 100);
	}
</script>

<div class="mx-auto max-w-2xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-bold text-gray-900">Privacy Score</h1>

	{#if loading}
		<div class="flex justify-center py-12">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600 mx-auto"></div>
		</div>
	{:else if error}
		<div class="rounded-md bg-red-50 p-4 text-sm text-red-700">{error}</div>
	{:else if result}
		<!-- Gauge -->
		<div class="mb-8 flex flex-col items-center">
			<svg
				width={SIZE}
				height={SIZE}
				viewBox="0 0 {SIZE} {SIZE}"
				role="img"
				aria-label="Privacy score gauge: {result.score} out of 100"
			>
				<!-- Background track -->
				<circle
					cx={SIZE / 2}
					cy={SIZE / 2}
					r={RADIUS}
					fill="none"
					stroke="#e5e7eb"
					stroke-width="16"
					stroke-dasharray={CIRCUMFERENCE}
					stroke-dashoffset="0"
					transform="rotate(-90 {SIZE / 2} {SIZE / 2})"
				/>
				<!-- Score arc -->
				<circle
					cx={SIZE / 2}
					cy={SIZE / 2}
					r={RADIUS}
					fill="none"
					stroke={gaugeColor(result.score)}
					stroke-width="16"
					stroke-linecap="round"
					stroke-dasharray={CIRCUMFERENCE}
					stroke-dashoffset={strokeDashoffset(result.score)}
					transform="rotate(-90 {SIZE / 2} {SIZE / 2})"
					style="transition: stroke-dashoffset 0.6s ease"
				/>
				<!-- Score number -->
				<text
					x={SIZE / 2}
					y={SIZE / 2 - 8}
					text-anchor="middle"
					dominant-baseline="middle"
					font-size="36"
					font-weight="bold"
					fill={gaugeColor(result.score)}>{result.score}</text
				>
				<text x={SIZE / 2} y={SIZE / 2 + 22} text-anchor="middle" font-size="13" fill="#6b7280"
					>{result.descriptor}</text
				>
			</svg>
		</div>

		<!-- Breakdown -->
		<div class="mb-6 rounded-lg border border-gray-200 bg-white overflow-hidden">
			<table class="w-full text-sm" aria-label="Privacy score breakdown">
				<thead class="bg-gray-50 text-xs uppercase text-gray-500">
					<tr>
						<th class="px-4 py-3 text-left">Status</th>
						<th class="px-4 py-3 text-right">Count</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-gray-100">
					<tr>
						<td class="px-4 py-3 text-gray-700">Unresolved findings</td>
						<td class="px-4 py-3 text-right text-gray-900">{result.unresolved_count}</td>
					</tr>
					<tr>
						<td class="px-4 py-3 text-gray-700">Confirmed removals</td>
						<td class="px-4 py-3 text-right text-gray-900">{result.confirmed_count}</td>
					</tr>
					<tr>
						<td class="px-4 py-3 text-gray-700">Failed removals</td>
						<td class="px-4 py-3 text-right text-gray-900">{result.failed_count}</td>
					</tr>
				</tbody>
			</table>
		</div>

		<div class="text-center">
			<a href="/removals" class="text-sm text-primary-600 hover:underline"
				>View removal history <span aria-hidden="true">→</span></a
			>
		</div>
	{:else}
		<p class="py-12 text-center text-sm text-gray-500">
			Please unlock a vault to view your privacy score.
		</p>
	{/if}
</div>
