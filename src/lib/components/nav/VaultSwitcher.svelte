<script lang="ts">
	import { vaultStore } from '$lib/stores/vault.svelte';

	let open = $state(false);
	let unlockModalVaultId = $state<string | null>(null);
	let unlockPassword = $state('');
	let unlockError = $state<string | null>(null);

	const currentVault = $derived(
		vaultStore.availableVaults.find((v) => v.vault_id === vaultStore.currentVaultId)
	);

	function switchTo(vaultId: string) {
		const isUnlocked = vaultStore.unlockedVaultIds.has(vaultId);
		if (isUnlocked) {
			vaultStore.setCurrentVault(vaultId);
			open = false;
		} else {
			unlockModalVaultId = vaultId;
			open = false;
		}
	}

	async function handleUnlock() {
		if (!unlockModalVaultId) return;
		unlockError = null;
		try {
			await vaultStore.unlock(unlockModalVaultId, unlockPassword);
			vaultStore.setCurrentVault(unlockModalVaultId);
			unlockModalVaultId = null;
			unlockPassword = '';
		} catch (err) {
			unlockError = err instanceof Error ? err.message : String(err);
		}
	}

	$effect(() => {
		if (!open) return;
		function handleOutsideClick(e: PointerEvent) {
			const target = e.target as Element | null;
			if (!target?.closest('[data-vault-switcher]')) {
				open = false;
			}
		}
		document.addEventListener('pointerdown', handleOutsideClick);
		return () => document.removeEventListener('pointerdown', handleOutsideClick);
	});
</script>

<div class="relative" data-vault-switcher="">
	<button
		onclick={() => (open = !open)}
		aria-haspopup="listbox"
		aria-expanded={open}
		class="flex items-center gap-2 rounded-md px-3 py-1.5 text-sm font-medium text-gray-700 hover:bg-gray-100 focus:outline-none"
	>
		<span class="max-w-32 truncate">{currentVault?.display_name ?? 'No vault'}</span>
		<svg class="h-4 w-4 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
		</svg>
	</button>

	{#if open}
		<div
			class="absolute right-0 z-50 mt-1 w-64 rounded-md border border-gray-200 bg-white py-1 shadow-lg"
		>
			{#each vaultStore.availableVaults as vault (vault.vault_id)}
				<button
					onclick={() => switchTo(vault.vault_id)}
					class="flex w-full items-center justify-between px-4 py-2 text-sm hover:bg-gray-50 {vault.vault_id ===
					vaultStore.currentVaultId
						? 'bg-primary-50 font-medium text-primary-700'
						: 'text-gray-700'}"
				>
					<span>{vault.display_name}</span>
					{#if vaultStore.unlockedVaultIds.has(vault.vault_id)}
						<span class="text-xs text-green-600">Unlocked</span>
					{:else}
						<span class="text-xs text-gray-400">Locked</span>
					{/if}
				</button>
			{/each}
			<div class="border-t border-gray-100 pt-1">
				<a
					href="/settings?tab=vaults"
					class="block px-4 py-2 text-sm text-gray-500 hover:bg-gray-50">Manage vaults…</a
				>
			</div>
		</div>
	{/if}
</div>

{#if unlockModalVaultId}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
		<div
			class="w-full max-w-sm rounded-lg bg-white p-6 shadow-xl"
			role="dialog"
			aria-modal="true"
			aria-labelledby="vault-unlock-title"
		>
			<h2 id="vault-unlock-title" class="mb-4 text-lg font-semibold text-gray-900">Unlock vault</h2>
			<input
				type="password"
				bind:value={unlockPassword}
				placeholder="Master password"
				autocomplete="current-password"
				class="mb-3 w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
				onkeydown={(e) => e.key === 'Enter' && handleUnlock()}
			/>
			{#if unlockError}
				<p class="mb-3 text-sm text-red-600">{unlockError}</p>
			{/if}
			<div class="flex gap-3">
				<button
					onclick={handleUnlock}
					class="flex-1 rounded-md bg-primary-600 px-4 py-2 text-sm font-medium text-white hover:bg-primary-700"
					>Unlock</button
				>
				<button
					onclick={() => {
						unlockModalVaultId = null;
						unlockPassword = '';
						unlockError = null;
					}}
					class="flex-1 rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50"
					>Cancel</button
				>
			</div>
		</div>
	</div>
{/if}
