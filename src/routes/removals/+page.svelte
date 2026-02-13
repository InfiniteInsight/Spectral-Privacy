<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	const removalCount = $derived($page.url.searchParams.get('count') || '0');
</script>

<div
	class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 flex items-center justify-center p-4"
>
	<div class="bg-white rounded-lg shadow-xl p-8 max-w-2xl w-full text-center">
		<!-- Success Icon -->
		<div class="mb-6">
			<div class="inline-flex items-center justify-center w-16 h-16 bg-green-100 rounded-full">
				<span class="text-4xl text-green-600">✓</span>
			</div>
		</div>

		<!-- Header -->
		<h1 class="text-3xl font-bold text-gray-900 mb-4">Removal Requests Submitted</h1>

		<p class="text-lg text-gray-600 mb-8">
			We've queued {removalCount} removal request{removalCount !== '1' ? 's' : ''} for processing.
		</p>

		<!-- Info Box -->
		<div class="mb-8 p-6 bg-blue-50 border border-blue-200 rounded-lg text-left">
			<h2 class="text-sm font-semibold text-gray-900 mb-3">What happens next:</h2>
			<ul class="text-sm text-gray-700 space-y-2">
				<li>• Removal requests are being prepared for each data broker</li>
				<li>• You'll be notified when requests are sent</li>
				<li>• Removal tracking coming soon in Phase 4</li>
			</ul>
		</div>

		<!-- Early Access Notice -->
		<div class="mb-8 p-4 bg-yellow-50 border border-yellow-200 rounded-lg">
			<p class="text-sm text-gray-700">
				<strong>Early Access:</strong> Full removal tracking and status updates will be available in the
				next release.
			</p>
		</div>

		<!-- Return to Dashboard -->
		<button
			onclick={() => goto('/')}
			class="px-8 py-4 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors text-lg"
			style="background-color: #0284c7; color: white;"
		>
			Return to Dashboard
		</button>
	</div>
</div>
