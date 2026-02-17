<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	onMount(() => {
		// For MVP, redirect to dashboard
		// In future, could show removal history/job list
		goto('/');
	});
</script>

<div
	class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 flex items-center justify-center"
>
	<div class="text-center">
		<div
			class="inline-block w-8 h-8 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"
		></div>
		<p class="text-gray-600 mt-4">Redirecting...</p>
	</div>
</div>
