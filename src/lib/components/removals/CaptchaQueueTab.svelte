<script lang="ts">
	import type { RemovalAttempt } from '$lib/api/removal';
	import { open } from '@tauri-apps/plugin-shell';

	interface Props {
		captchaQueue: RemovalAttempt[];
	}

	let { captchaQueue }: Props = $props();

	function extractCaptchaUrl(errorMessage: string): string {
		// Format: "CAPTCHA_REQUIRED:https://..."
		const parts = errorMessage.split('CAPTCHA_REQUIRED:');
		return parts.length > 1 ? parts[1] : '';
	}

	async function openInBrowser(url: string) {
		try {
			await open(url);
		} catch (err) {
			console.error('Failed to open browser:', err);
			alert('Failed to open URL in browser');
		}
	}

	function formatTime(isoString: string) {
		const date = new Date(isoString);
		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffMins = Math.floor(diffMs / 60000);

		if (diffMins < 1) return 'Just now';
		if (diffMins < 60) return `${diffMins}m ago`;
		const diffHours = Math.floor(diffMins / 60);
		if (diffHours < 24) return `${diffHours}h ago`;
		const diffDays = Math.floor(diffHours / 24);
		return `${diffDays}d ago`;
	}
</script>

<div class="space-y-4">
	{#if captchaQueue.length === 0}
		<!-- Empty State -->
		<div class="bg-white rounded-lg border border-gray-200 p-12 text-center">
			<div class="inline-flex items-center justify-center w-16 h-16 bg-green-100 rounded-full mb-4">
				<span class="text-3xl text-green-600">✓</span>
			</div>
			<h3 class="text-lg font-semibold text-gray-900 mb-2">No CAPTCHAs to solve</h3>
			<p class="text-sm text-gray-600">All removals processed without CAPTCHA blocks</p>
		</div>
	{:else}
		<!-- CAPTCHA Queue List -->
		<div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
			<div class="px-6 py-4 border-b border-gray-200 bg-gray-50">
				<h2 class="text-lg font-semibold text-gray-900">
					CAPTCHA Queue ({captchaQueue.length})
				</h2>
				<p class="text-sm text-gray-600 mt-1">These removals require CAPTCHA solving to continue</p>
			</div>

			<div class="divide-y divide-gray-200">
				{#each captchaQueue as attempt}
					{@const captchaUrl = extractCaptchaUrl(attempt.error_message || '')}
					<div class="p-6 hover:bg-gray-50">
						<div class="flex items-start justify-between">
							<div class="flex-1">
								<div class="flex items-center gap-2 mb-2">
									<span class="text-sm font-semibold text-gray-900">{attempt.broker_id}</span>
									<span
										class="px-2 py-1 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800"
									>
										CAPTCHA Required
									</span>
								</div>

								<div class="text-sm text-gray-600 mb-2">
									<span class="font-medium">Listing URL:</span>
									<span class="ml-2 break-all">{captchaUrl || 'Unknown'}</span>
								</div>

								<div class="text-xs text-gray-500">
									Blocked {formatTime(attempt.created_at)}
								</div>
							</div>

							<button
								onclick={() => openInBrowser(captchaUrl)}
								disabled={!captchaUrl}
								class="ml-4 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm font-medium"
							>
								Open in Browser
							</button>
						</div>

						<div class="mt-3 p-3 bg-blue-50 rounded-lg">
							<p class="text-xs text-blue-900">
								<strong>Instructions:</strong> Click "Open in Browser" to solve the CAPTCHA manually.
								After solving, the removal may be retried from the Failed queue if needed.
							</p>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
