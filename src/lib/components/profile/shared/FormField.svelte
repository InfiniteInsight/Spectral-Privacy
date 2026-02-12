<script lang="ts">
	/* eslint-disable no-unused-vars */
	interface Props {
		label: string;
		id: string;
		type?: 'text' | 'email' | 'date';
		value: string;
		error?: string;
		required?: boolean;
		placeholder?: string;
		onchange: (value: string) => void;
	}

	let {
		label,
		id,
		type = 'text',
		value = $bindable(''),
		error,
		required = false,
		placeholder,
		onchange
	}: Props = $props();
	/* eslint-enable no-unused-vars */

	function handleInput(e: Event) {
		const target = e.target as HTMLInputElement;
		value = target.value;
		onchange(target.value);
	}

	function handleKeyDown(e: KeyboardEvent) {
		// For date inputs, Enter should close the picker and move to next field
		if (e.key === 'Enter' && type === 'date') {
			e.preventDefault();
			const target = e.target as HTMLInputElement;

			// Blur to close the date picker
			target.blur();

			// Find and focus the next focusable element
			const form = target.form || target.closest('form');
			if (form) {
				const focusableElements = Array.from(
					form.querySelectorAll<HTMLElement>(
						'input:not([disabled]), select:not([disabled]), textarea:not([disabled]), button:not([disabled])'
					)
				);
				const currentIndex = focusableElements.indexOf(target);
				const nextElement = focusableElements[currentIndex + 1];
				if (nextElement) {
					nextElement.focus();
				}
			}
		}
	}
</script>

<div class="form-field">
	<label for={id} class="block text-sm font-medium text-gray-700 mb-2">
		{label}
		{#if required}
			<span class="text-red-500">*</span>
		{/if}
	</label>
	<input
		{id}
		{type}
		{value}
		{placeholder}
		{required}
		oninput={handleInput}
		onkeydown={handleKeyDown}
		class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 transition-colors"
		class:border-gray-300={!error}
		class:border-red-500={error}
		class:focus:ring-primary-500={!error}
		class:focus:ring-red-500={error}
	/>
	{#if error}
		<p class="mt-1 text-sm text-red-600">{error}</p>
	{/if}
</div>

<style>
	.form-field {
		margin-bottom: 1rem;
	}
</style>
