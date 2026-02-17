<script lang="ts">
	import { goto } from '$app/navigation';

	$effect(() => {
		// For MVP, redirect to dashboard
		// In future, could show removal history/job list
		goto('/');
	});
</script>

<div
	class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 flex items-center justify-center p-4"
>
	<div class="text-center">
		<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600 mx-auto"></div>
		<p class="text-gray-600 mt-4">Redirecting...</p>
	</div>
</div>
