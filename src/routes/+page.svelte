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
				style="background-color: #e0f2fe; color: #0369a1; padding: 0.5rem 1rem; border-radius: 9999px; font-size: 0.875rem; font-weight: 500; display: inline-flex; align-items: center;"
			>
				✓ Vault Unlocked
			</div>
			<button
				onclick={() => vaultStore.currentVaultId && vaultStore.lock(vaultStore.currentVaultId)}
				class="mt-4 px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 transition-colors"
				style="background-color: #0284c7; color: white; padding: 0.75rem 1.5rem; border-radius: 0.5rem; font-weight: 500; margin-top: 1rem;"
			>
				Lock Vault
			</button>
		</div>
	</div>
{:else}
	<UnlockScreen />
{/if}
