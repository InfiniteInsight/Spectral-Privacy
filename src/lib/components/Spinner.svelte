<script lang="ts">
	/**
	 * Reusable spinner component with consistent styling across the app.
	 * Uses the cookie scanner's preferred style with gray track and colored top border.
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

<!-- Use explicit class combinations with gray track and colored top border -->
{#if size === 'sm' && color === 'primary'}
	<div
		class="animate-spin rounded-full h-5 w-5 border-2 border-gray-200 border-t-primary-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'sm' && color === 'purple'}
	<div
		class="animate-spin rounded-full h-5 w-5 border-2 border-gray-200 border-t-purple-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'sm' && color === 'indigo'}
	<div
		class="animate-spin rounded-full h-5 w-5 border-2 border-gray-200 border-t-indigo-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'sm' && color === 'orange'}
	<div
		class="animate-spin rounded-full h-5 w-5 border-2 border-gray-200 border-t-orange-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'sm' && color === 'white'}
	<div
		class="animate-spin rounded-full h-5 w-5 border-2 border-transparent border-t-white {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'md' && color === 'primary'}
	<div
		class="animate-spin rounded-full h-8 w-8 border-4 border-gray-200 border-t-primary-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'md' && color === 'purple'}
	<div
		class="animate-spin rounded-full h-8 w-8 border-4 border-gray-200 border-t-purple-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'md' && color === 'indigo'}
	<div
		class="animate-spin rounded-full h-8 w-8 border-4 border-gray-200 border-t-indigo-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'md' && color === 'orange'}
	<div
		class="animate-spin rounded-full h-8 w-8 border-4 border-gray-200 border-t-orange-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'md' && color === 'white'}
	<div
		class="animate-spin rounded-full h-8 w-8 border-4 border-transparent border-t-white {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'lg' && color === 'primary'}
	<div
		class="animate-spin rounded-full h-12 w-12 border-4 border-gray-200 border-t-primary-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'lg' && color === 'purple'}
	<div
		class="animate-spin rounded-full h-12 w-12 border-4 border-gray-200 border-t-purple-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'lg' && color === 'indigo'}
	<div
		class="animate-spin rounded-full h-12 w-12 border-4 border-gray-200 border-t-indigo-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'lg' && color === 'orange'}
	<div
		class="animate-spin rounded-full h-12 w-12 border-4 border-gray-200 border-t-orange-600 {inline
			? ''
			: 'mx-auto'}"
	></div>
{:else if size === 'lg' && color === 'white'}
	<div
		class="animate-spin rounded-full h-12 w-12 border-4 border-transparent border-t-white {inline
			? ''
			: 'mx-auto'}"
	></div>
{/if}
