<script lang="ts">
	/**
	 * Reusable spinner component with consistent styling across the app.
	 *
	 * @param size - Size of the spinner: 'sm' (4x4), 'md' (8x8), 'lg' (12x12)
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

	// Size classes
	const sizeClasses = {
		sm: 'h-5 w-5 border-2',
		md: 'h-8 w-8 border-4',
		lg: 'h-12 w-12 border-4'
	};

	// Color classes for the spinning part
	const colorClasses = {
		primary: 'border-t-primary-600',
		purple: 'border-t-purple-600',
		indigo: 'border-t-indigo-600',
		orange: 'border-t-orange-600',
		white: 'border-t-white'
	};

	// Track color (background circle)
	const trackColor = color === 'white' ? 'border-transparent' : 'border-gray-200';

	// Margin class
	const marginClass = inline ? '' : 'mx-auto';

	const classes = `${sizeClasses[size]} ${trackColor} ${colorClasses[color]} ${marginClass} animate-spin rounded-full`;
</script>

<div class={classes}></div>
