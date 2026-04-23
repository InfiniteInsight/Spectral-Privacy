<script lang="ts">
	/**
	 * Reusable spinner component with consistent styling across the app.
	 * Gray track with a colored indicator set via inline style to avoid
	 * Tailwind v4 border-color shorthand vs border-top-color ordering issues.
	 *
	 * @param size - Size of the spinner: 'sm' (5x5), 'md' (8x8), 'lg' (12x12)
	 * @param color - Color of the spinning indicator: 'primary', 'purple', 'indigo', 'orange', 'white'
	 * @param inline - If true, displays inline without margin. If false, centers with mx-auto.
	 */

	const COLOR_MAP: Record<string, string> = {
		primary: '#0284c7',
		purple: '#9333ea',
		indigo: '#4f46e5',
		orange: '#ea580c',
		white: 'white'
	};

	const SIZE_MAP = {
		sm: 'h-5 w-5 border-2',
		md: 'h-8 w-8 border-4',
		lg: 'h-12 w-12 border-4'
	};

	let {
		size = 'md',
		color = 'primary',
		inline = false
	}: {
		size?: 'sm' | 'md' | 'lg';
		color?: 'primary' | 'purple' | 'indigo' | 'orange' | 'white';
		inline?: boolean;
	} = $props();

	const trackClass = $derived(color === 'white' ? 'border-transparent' : 'border-gray-200');
	const indicatorColor = $derived(COLOR_MAP[color] ?? '#0284c7');
	const sizeClass = $derived(SIZE_MAP[size]);
	const marginClass = $derived(inline ? '' : 'mx-auto');
</script>

<div
	class="animate-spin rounded-full {sizeClass} {trackClass} {marginClass}"
	style="border-top-color: {indicatorColor}"
></div>
