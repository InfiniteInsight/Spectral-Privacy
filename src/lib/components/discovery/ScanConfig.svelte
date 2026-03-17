<script lang="ts">
	interface Props {
		config: {
			scan_emails: boolean;
			scan_phones: boolean;
			scan_ssn: boolean;
			scan_addresses: boolean;
			scan_names: boolean;
			scan_dob: boolean;
		};
		// eslint-disable-next-line no-unused-vars
		onConfigChange: (config: Props['config']) => void;
		disabled?: boolean;
	}

	let { config, onConfigChange, disabled = false }: Props = $props();

	function toggle(key: keyof Props['config']) {
		onConfigChange({ ...config, [key]: !config[key] });
	}

	const options = [
		{ key: 'scan_emails' as const, label: 'Emails', icon: '✉️' },
		{ key: 'scan_phones' as const, label: 'Phones', icon: '📞' },
		{ key: 'scan_ssn' as const, label: 'SSN', icon: '🔐' },
		{ key: 'scan_addresses' as const, label: 'Addresses', icon: '🏠' },
		{ key: 'scan_names' as const, label: 'Names', icon: '👤' },
		{ key: 'scan_dob' as const, label: 'Date of Birth', icon: '🎂' }
	];

	const hasAny = $derived(Object.values(config).some(Boolean));
</script>

<div class="rounded-lg border border-gray-200 bg-white p-4">
	<h3 class="text-sm font-semibold text-gray-900 mb-3">What PII to scan for?</h3>
	<div class="grid grid-cols-2 md:grid-cols-3 gap-2">
		{#each options as opt}
			<label
				class="flex items-center gap-2 p-2 rounded-lg border cursor-pointer transition-colors
                {config[opt.key]
					? 'border-indigo-300 bg-indigo-50'
					: 'border-gray-200 hover:bg-gray-50'}
                {disabled ? 'opacity-50 cursor-not-allowed' : ''}"
			>
				<input
					type="checkbox"
					checked={config[opt.key]}
					onchange={() => toggle(opt.key)}
					{disabled}
					class="h-4 w-4 rounded border-gray-300 text-indigo-600"
				/>
				<span class="text-sm">{opt.icon}</span>
				<span class="text-sm text-gray-700">{opt.label}</span>
			</label>
		{/each}
	</div>
	{#if !hasAny}
		<p class="mt-2 text-xs text-red-600">Select at least one PII type.</p>
	{/if}
</div>
