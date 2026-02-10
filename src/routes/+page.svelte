<script lang="ts">
	import { UnlockScreen } from '$lib/components';
	import { vaultStore } from '$lib/stores';
</script>

{#if vaultStore.isCurrentVaultUnlocked}
	<!-- Dashboard Placeholder -->
	<div
		class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 flex items-center justify-center p-4"
	>
		<div class="bg-white rounded-lg shadow-xl p-8 text-center">
			<h1 class="text-3xl font-bold text-gray-900 mb-4">Spectral</h1>
			<p class="text-gray-600 mb-6">Automated data broker removal</p>
			<div
				class="inline-flex items-center px-4 py-2 bg-primary-100 text-primary-700 rounded-full text-sm font-medium"
			>
				✓ Vault Unlocked
			</div>
		</div>
	</div>
{:else}
	<UnlockScreen />
{/if}
