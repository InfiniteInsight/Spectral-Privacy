<script lang="ts">
	interface Props {
		piiTypeFilter: Set<string>;
		riskLevelFilter: Set<string>;
		showIgnored: boolean;
		// eslint-disable-next-line no-unused-vars
		onPiiTypeToggle: (type: string) => void;
		// eslint-disable-next-line no-unused-vars
		onRiskLevelToggle: (level: string) => void;
		// eslint-disable-next-line no-unused-vars
		onShowIgnoredChange: (show: boolean) => void;
		totalCount: number;
		filteredCount: number;
	}

	let {
		piiTypeFilter,
		riskLevelFilter,
		showIgnored,
		onPiiTypeToggle,
		onRiskLevelToggle,
		onShowIgnoredChange,
		totalCount,
		filteredCount
	}: Props = $props();

	function chipClass(selected: boolean): string {
		return selected
			? 'px-3 py-1 rounded-full text-sm font-medium bg-indigo-600 text-white cursor-pointer'
			: 'px-3 py-1 rounded-full text-sm font-medium bg-gray-200 text-gray-700 cursor-pointer hover:bg-gray-300';
	}

	const piiTypes = ['email', 'phone', 'ssn', 'address', 'name', 'dob'];
	const riskLevels = ['critical', 'high', 'medium', 'low'];
</script>

<div class="space-y-3 mb-6">
	<div class="flex items-center gap-2 flex-wrap">
		<span class="text-sm font-medium text-gray-700">PII Type:</span>
		{#each piiTypes as type}
			<button onclick={() => onPiiTypeToggle(type)} class={chipClass(piiTypeFilter.has(type))}
				>{type}</button
			>
		{/each}
	</div>

	<div class="flex items-center gap-2 flex-wrap">
		<span class="text-sm font-medium text-gray-700">Risk:</span>
		{#each riskLevels as level}
			<button onclick={() => onRiskLevelToggle(level)} class={chipClass(riskLevelFilter.has(level))}
				>{level}</button
			>
		{/each}
	</div>

	<div class="flex items-center justify-between">
		<label class="flex items-center gap-2 cursor-pointer">
			<input
				type="checkbox"
				checked={showIgnored}
				onchange={(e) => onShowIgnoredChange(e.currentTarget.checked)}
				class="h-4 w-4 rounded"
			/>
			<span class="text-sm text-gray-700">Show ignored</span>
		</label>
		<span class="text-sm text-gray-600">Showing {filteredCount} of {totalCount}</span>
	</div>
</div>
