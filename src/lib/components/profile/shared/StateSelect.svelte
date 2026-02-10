<script lang="ts">
	/* eslint-disable no-unused-vars */
	interface Props {
		label: string;
		id: string;
		value: string;
		error?: string;
		required?: boolean;
		onchange: (value: string) => void;
	}

	let { label, id, value = $bindable(''), error, required = false, onchange }: Props = $props();
	/* eslint-enable no-unused-vars */

	const US_STATES = [
		{ code: 'AL', name: 'Alabama' },
		{ code: 'AK', name: 'Alaska' },
		{ code: 'AZ', name: 'Arizona' },
		{ code: 'AR', name: 'Arkansas' },
		{ code: 'CA', name: 'California' },
		{ code: 'CO', name: 'Colorado' },
		{ code: 'CT', name: 'Connecticut' },
		{ code: 'DE', name: 'Delaware' },
		{ code: 'FL', name: 'Florida' },
		{ code: 'GA', name: 'Georgia' },
		{ code: 'HI', name: 'Hawaii' },
		{ code: 'ID', name: 'Idaho' },
		{ code: 'IL', name: 'Illinois' },
		{ code: 'IN', name: 'Indiana' },
		{ code: 'IA', name: 'Iowa' },
		{ code: 'KS', name: 'Kansas' },
		{ code: 'KY', name: 'Kentucky' },
		{ code: 'LA', name: 'Louisiana' },
		{ code: 'ME', name: 'Maine' },
		{ code: 'MD', name: 'Maryland' },
		{ code: 'MA', name: 'Massachusetts' },
		{ code: 'MI', name: 'Michigan' },
		{ code: 'MN', name: 'Minnesota' },
		{ code: 'MS', name: 'Mississippi' },
		{ code: 'MO', name: 'Missouri' },
		{ code: 'MT', name: 'Montana' },
		{ code: 'NE', name: 'Nebraska' },
		{ code: 'NV', name: 'Nevada' },
		{ code: 'NH', name: 'New Hampshire' },
		{ code: 'NJ', name: 'New Jersey' },
		{ code: 'NM', name: 'New Mexico' },
		{ code: 'NY', name: 'New York' },
		{ code: 'NC', name: 'North Carolina' },
		{ code: 'ND', name: 'North Dakota' },
		{ code: 'OH', name: 'Ohio' },
		{ code: 'OK', name: 'Oklahoma' },
		{ code: 'OR', name: 'Oregon' },
		{ code: 'PA', name: 'Pennsylvania' },
		{ code: 'RI', name: 'Rhode Island' },
		{ code: 'SC', name: 'South Carolina' },
		{ code: 'SD', name: 'South Dakota' },
		{ code: 'TN', name: 'Tennessee' },
		{ code: 'TX', name: 'Texas' },
		{ code: 'UT', name: 'Utah' },
		{ code: 'VT', name: 'Vermont' },
		{ code: 'VA', name: 'Virginia' },
		{ code: 'WA', name: 'Washington' },
		{ code: 'WV', name: 'West Virginia' },
		{ code: 'WI', name: 'Wisconsin' },
		{ code: 'WY', name: 'Wyoming' }
	];

	function handleChange(e: Event) {
		const target = e.target as HTMLSelectElement;
		value = target.value;
		onchange(target.value);
	}
</script>

<div class="form-field">
	<label for={id} class="block text-sm font-medium text-gray-700 mb-2">
		{label}
		{#if required}
			<span class="text-red-500">*</span>
		{/if}
	</label>
	<select
		{id}
		{value}
		{required}
		onchange={handleChange}
		class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 transition-colors"
		class:border-gray-300={!error}
		class:border-red-500={error}
		class:focus:ring-primary-500={!error}
		class:focus:ring-red-500={error}
	>
		<option value="">Select a state...</option>
		{#each US_STATES as state}
			<option value={state.code}>{state.name}</option>
		{/each}
	</select>
	{#if error}
		<p class="mt-1 text-sm text-red-600">{error}</p>
	{/if}
</div>

<style>
	.form-field {
		margin-bottom: 1rem;
	}
</style>
