<script lang="ts">
	/**
	 * Reusable spinner component with consistent styling across the app.
	 *
	 * @param size - Size of the spinner: 'sm' (5x5), 'md' (8x8), 'lg' (12x12)
	 * @param color - Color of the spinning part: 'primary', 'purple', 'indigo', 'orange', 'white'
	 * @param inline - If true, displays inline without margin. If false, centers with mx-auto.
	 */

	let {
		size = 'md',
		color = 'primary',
		inline = false
	}: {
		size?: 'sm' | 'md' | 'lg';
		color?: 'primary' | 'purple' | 'indigo' | 'orange' | 'white';
		inline?: boolean;
	} = $props();
</script>

<!-- Use explicit class combinations for each size/color to ensure Tailwind processes them -->
{#if size === 'sm' && color === 'primary'}
	<div
		class="animate-spin rounded-full h-5 w-5 border-b-2 border-primary-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'sm' && color === 'purple'}
	<div
		class="animate-spin rounded-full h-5 w-5 border-b-2 border-purple-600 {inline ? '' : 'mx-auto'}"
	></div>
{:else if size === 'sm' && color === 'indigo'}
	<div
		class="animate-spin rounded-full h-5 w-5 border-b-2 border-indigo-600 {inline ? '' : 'mx-auto'}"
	></div>
{:else if size === 'sm' && color === 'orange'}
	<div
		class="animate-spin rounded-full h-5 w-5 border-b-2 border-orange-600 {inline ? '' : 'mx-auto'}"
	></div>
{:else if size === 'sm' && color === 'white'}
	<div
		class="animate-spin rounded-full h-5 w-5 border-b-2 border-white {inline ? '' : 'mx-auto'}"
	></div>
{:else if size === 'md' && color === 'primary'}
	<div
		class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'md' && color === 'purple'}
	<div
		class="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-600 {inline ? '' : 'mx-auto'}"
	></div>
{:else if size === 'md' && color === 'indigo'}
	<div
		class="animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600 {inline ? '' : 'mx-auto'}"
	></div>
{:else if size === 'md' && color === 'orange'}
	<div
		class="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-600 {inline ? '' : 'mx-auto'}"
	></div>
{:else if size === 'md' && color === 'white'}
	<div
		class="animate-spin rounded-full h-8 w-8 border-b-2 border-white {inline ? '' : 'mx-auto'}"
	></div>
{:else if size === 'lg' && color === 'primary'}
	<div
		class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'lg' && color === 'purple'}
	<div
		class="animate-spin rounded-full h-12 w-12 border-b-2 border-purple-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'lg' && color === 'indigo'}
	<div
		class="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'lg' && color === 'orange'}
	<div
		class="animate-spin rounded-full h-12 w-12 border-b-2 border-orange-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'lg' && color === 'white'}
	<div
		class="animate-spin rounded-full h-12 w-12 border-b-2 border-white {inline ? '' : 'mx-auto'}"
	></div>
{/if}
