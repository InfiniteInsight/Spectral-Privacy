<script lang="ts">
	import { vaultStore } from '$lib/stores/vault.svelte';
	import { renameVault, deleteVault } from '$lib/api/vault';

	// Vault management state
	let renameTarget = $state<string | null>(null);
	let renameValue = $state('');
	let deleteTarget = $state<string | null>(null);
	let deletePassword = $state('');
	let unlockTarget = $state<string | null>(null);
	let unlockPassword = $state('');
	let actionError = $state<string | null>(null);
	let actionLoading = $state(false);
	let showCreateForm = $state(false);
	let newVaultId = $state('');
	let newVaultName = $state('');
	let newVaultPassword = $state('');

	async function handleRename(vaultId: string) {
		actionError = null;
		actionLoading = true;
		try {
			await renameVault(vaultId, renameValue);
			await vaultStore.loadVaults();
			renameTarget = null;
			renameValue = '';
		} catch (err) {
			actionError = err instanceof Error ? err.message : String(err);
		} finally {
			actionLoading = false;
		}
	}

	async function handleDelete(vaultId: string) {
		actionError = null;
		actionLoading = true;
		try {
			await deleteVault(vaultId, deletePassword);
			await vaultStore.loadVaults();
			deleteTarget = null;
			deletePassword = '';
		} catch (err) {
			actionError = err instanceof Error ? err.message : String(err);
		} finally {
			actionLoading = false;
		}
	}

	async function handleLock(vaultId: string) {
		actionError = null;
		actionLoading = true;
		try {
			await vaultStore.lock(vaultId);
		} catch (err) {
			actionError = err instanceof Error ? err.message : String(err);
		} finally {
			actionLoading = false;
		}
	}

	async function handleUnlock(vaultId: string) {
		actionError = null;
		actionLoading = true;
		try {
			await vaultStore.unlock(vaultId, unlockPassword);
			unlockTarget = null;
			unlockPassword = '';
		} catch (err) {
			actionError = err instanceof Error ? err.message : String(err);
		} finally {
			actionLoading = false;
		}
	}

	async function handleCreateVault() {
		if (!newVaultId || !newVaultName || !newVaultPassword) return;
		actionError = null;
		actionLoading = true;
		try {
			await vaultStore.createVault(newVaultId, newVaultName, newVaultPassword);
			newVaultId = '';
			newVaultName = '';
			newVaultPassword = '';
			showCreateForm = false;
			await vaultStore.loadVaults();
		} catch (err) {
			actionError = err instanceof Error ? err.message : String(err);
		} finally {
			actionLoading = false;
		}
	}
</script>

<div class="mx-auto max-w-3xl px-4 py-8">
	<div class="mb-6">
		<h1 class="text-2xl font-bold text-gray-900">People</h1>
		<p class="mt-1 text-sm text-gray-500">
			Manage vaults for different identities. Each vault stores separate scans, findings, and
			removal history.
		</p>
	</div>

	<!-- Vault List -->
	<div class="space-y-3">
		{#each vaultStore.availableVaults as vault (vault.vault_id)}
			<div class="rounded-lg border border-gray-200 bg-white p-4">
				{#if renameTarget === vault.vault_id}
					<div class="flex items-center gap-3">
						<input
							bind:value={renameValue}
							class="flex-1 rounded-md border border-gray-300 px-3 py-1.5 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							onkeydown={(e) => e.key === 'Enter' && handleRename(vault.vault_id)}
						/>
						<button
							onclick={() => handleRename(vault.vault_id)}
							disabled={actionLoading}
							class="rounded-md bg-primary-600 px-3 py-1.5 text-sm text-white hover:bg-primary-700 disabled:opacity-50"
							style="background-color: #0284c7; color: white;">Save</button
						>
						<button
							onclick={() => {
								renameTarget = null;
								renameValue = '';
								actionError = null;
							}}
							class="rounded-md border border-gray-300 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
							>Cancel</button
						>
					</div>
				{:else}
					<div class="flex items-center justify-between">
						<div>
							<p class="font-medium text-gray-900">{vault.display_name}</p>
							<p class="text-xs text-gray-400">
								Last accessed: {new Date(vault.last_accessed).toLocaleDateString()}
							</p>
						</div>
						<div class="flex gap-2">
							{#if vaultStore.unlockedVaultIds.has(vault.vault_id)}
								<button
									onclick={() => handleLock(vault.vault_id)}
									disabled={actionLoading}
									class="rounded-md border border-gray-200 px-3 py-1.5 text-xs text-gray-600 hover:bg-gray-50 disabled:opacity-50"
									>Lock</button
								>
							{:else}
								<button
									onclick={() => {
										unlockTarget = vault.vault_id;
										actionError = null;
									}}
									class="rounded-md border border-primary-200 px-3 py-1.5 text-xs text-primary-600 hover:bg-primary-50"
									>Unlock</button
								>
							{/if}
							<button
								onclick={() => {
									renameTarget = vault.vault_id;
									renameValue = vault.display_name;
								}}
								class="rounded-md border border-gray-200 px-3 py-1.5 text-xs text-gray-600 hover:bg-gray-50"
								>Rename</button
							>
							<button
								onclick={() => {
									deleteTarget = vault.vault_id;
									actionError = null;
								}}
								class="rounded-md border border-red-200 px-3 py-1.5 text-xs text-red-600 hover:bg-red-50"
								>Delete</button
							>
						</div>
					</div>
				{/if}
			</div>
		{/each}
	</div>

	{#if actionError && (renameTarget !== null || deleteTarget !== null || unlockTarget !== null)}
		<p class="mt-2 text-sm text-red-600">{actionError}</p>
	{/if}

	<!-- Create New Vault -->
	<div class="mt-6">
		{#if !showCreateForm}
			<button
				onclick={() => (showCreateForm = true)}
				class="inline-flex items-center gap-2 rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50"
			>
				<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M12 4v16m8-8H4"
					/>
				</svg>
				Add new vault
			</button>
		{:else}
			<div class="rounded-lg border border-gray-200 bg-white p-4">
				<h3 class="mb-4 font-medium text-gray-900">Create New Vault</h3>
				<form
					onsubmit={(e) => {
						e.preventDefault();
						handleCreateVault();
					}}
					class="space-y-4"
				>
					<div>
						<label for="vault-id" class="block text-sm font-medium text-gray-700 mb-1">
							Vault ID
						</label>
						<input
							id="vault-id"
							type="text"
							bind:value={newVaultId}
							class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							placeholder="my-vault"
							disabled={actionLoading}
						/>
						<p class="text-xs text-gray-500 mt-1">Lowercase letters, numbers, and hyphens only</p>
					</div>

					<div>
						<label for="vault-name" class="block text-sm font-medium text-gray-700 mb-1">
							Display Name
						</label>
						<input
							id="vault-name"
							type="text"
							bind:value={newVaultName}
							class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							placeholder="My Vault"
							disabled={actionLoading}
						/>
					</div>

					<div>
						<label for="new-password" class="block text-sm font-medium text-gray-700 mb-1">
							Password
						</label>
						<input
							id="new-password"
							type="password"
							bind:value={newVaultPassword}
							class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							placeholder="Choose a strong password"
							disabled={actionLoading}
						/>
					</div>

					{#if actionError && showCreateForm}
						<div class="bg-red-50 border border-red-200 rounded-md p-3">
							<p class="text-sm text-red-800">{actionError}</p>
						</div>
					{/if}

					<div class="flex gap-2">
						<button
							type="button"
							onclick={() => {
								showCreateForm = false;
								newVaultId = '';
								newVaultName = '';
								newVaultPassword = '';
								actionError = null;
							}}
							disabled={actionLoading}
							class="flex-1 rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50"
							>Cancel</button
						>
						<button
							type="submit"
							disabled={actionLoading || !newVaultId || !newVaultName || !newVaultPassword}
							class="flex-1 rounded-md px-4 py-2 text-sm font-medium text-white disabled:opacity-50 disabled:cursor-not-allowed"
							style="background-color: {actionLoading ||
							!newVaultId ||
							!newVaultName ||
							!newVaultPassword
								? '#d1d5db'
								: '#0284c7'}; color: white;"
							>{actionLoading ? 'Creating...' : 'Create Vault'}</button
						>
					</div>
				</form>
			</div>
		{/if}
	</div>

	<!-- Delete vault confirmation modal -->
	{#if deleteTarget}
		<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
			<div
				class="w-full max-w-sm rounded-lg bg-white p-6 shadow-xl"
				role="dialog"
				aria-modal="true"
				aria-labelledby="delete-vault-title"
			>
				<h2 id="delete-vault-title" class="mb-2 text-lg font-semibold text-gray-900">
					Delete vault?
				</h2>
				<p class="mb-4 text-sm text-gray-500">
					This permanently deletes all data in this vault. Enter your master password to confirm.
				</p>
				<input
					type="password"
					bind:value={deletePassword}
					placeholder="Master password"
					autocomplete="current-password"
					class="mb-3 w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-red-400 focus:outline-none focus:ring-1 focus:ring-red-400"
				/>
				{#if actionError}
					<p class="mb-3 text-sm text-red-600">{actionError}</p>
				{/if}
				<div class="flex gap-3">
					<button
						onclick={() => handleDelete(deleteTarget!)}
						disabled={actionLoading}
						class="flex-1 rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:opacity-50"
						>Delete permanently</button
					>
					<button
						onclick={() => {
							deleteTarget = null;
							deletePassword = '';
							actionError = null;
						}}
						class="flex-1 rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50"
						>Cancel</button
					>
				</div>
			</div>
		</div>
	{/if}

	<!-- Unlock vault modal -->
	{#if unlockTarget}
		<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
			<div
				class="w-full max-w-sm rounded-lg bg-white p-6 shadow-xl"
				role="dialog"
				aria-modal="true"
				aria-labelledby="unlock-vault-title"
			>
				<h2 id="unlock-vault-title" class="mb-4 text-lg font-semibold text-gray-900">
					Unlock vault
				</h2>
				<input
					type="password"
					bind:value={unlockPassword}
					placeholder="Master password"
					autocomplete="current-password"
					class="mb-3 w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
					onkeydown={(e) => e.key === 'Enter' && handleUnlock(unlockTarget!)}
				/>
				{#if actionError}
					<p class="mb-3 text-sm text-red-600">{actionError}</p>
				{/if}
				<div class="flex gap-3">
					<button
						onclick={() => handleUnlock(unlockTarget!)}
						disabled={actionLoading || !unlockPassword}
						class="flex-1 rounded-md px-4 py-2 text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed"
						style="background-color: {actionLoading || !unlockPassword
							? '#d1d5db'
							: '#0284c7'}; color: white;">{actionLoading ? 'Unlocking...' : 'Unlock'}</button
					>
					<button
						onclick={() => {
							unlockTarget = null;
							unlockPassword = '';
							actionError = null;
						}}
						class="flex-1 rounded-md border px-4 py-2 text-sm hover:bg-gray-50"
						style="border-color: #d1d5db; color: #374151;">Cancel</button
					>
				</div>
			</div>
		</div>
	{/if}
</div>
