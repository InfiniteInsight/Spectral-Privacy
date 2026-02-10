<script lang="ts">
	import { vaultStore } from '$lib/stores';
	import { onMount } from 'svelte';

	let password = $state('');
	let showPassword = $state(false);

	// More robust Tauri detection
	let isTauri = $state(false);

	onMount(() => {
		// Check multiple ways Tauri might be available
		isTauri = !!(
			(typeof window !== 'undefined' && '__TAURI__' in window) ||
			(typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) ||
			(typeof window !== 'undefined' && (window as any).__TAURI__)
		);
		console.log('Tauri detected:', isTauri);
		console.log('window.__TAURI__:', (window as any).__TAURI__);
	});
	let showCreateForm = $state(false);
	let newVaultId = $state('');
	let newVaultName = $state('');
	let newVaultPassword = $state('');

	const selectedVault = $derived(
		vaultStore.availableVaults.find((v) => v.vault_id === vaultStore.currentVaultId)
	);

	onMount(async () => {
		await vaultStore.loadVaults();
	});

	async function handleUnlock() {
		if (!vaultStore.currentVaultId || !password) return;

		try {
			await vaultStore.unlock(vaultStore.currentVaultId, password);
			password = ''; // Clear password on success
		} catch (err) {
			// Error already set in store, but log for debugging
			console.error('Unlock failed:', err);
		}
	}

	async function handleCreateVault() {
		if (!newVaultId || !newVaultName || !newVaultPassword) return;

		try {
			await vaultStore.createVault(newVaultId, newVaultName, newVaultPassword);
			// Clear form
			newVaultId = '';
			newVaultName = '';
			newVaultPassword = '';
			showCreateForm = false;
			// Reload vaults
			await vaultStore.loadVaults();
		} catch (err) {
			console.error('Create vault failed:', err);
		}
	}
</script>

<div
	class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 flex items-center justify-center p-4"
>
	<div class="bg-white rounded-lg shadow-xl p-8 w-full max-w-md">
		<h1 class="text-2xl font-bold text-gray-900 mb-6 text-center">Unlock Vault</h1>

		<!-- Debug Info (remove after fixing) -->
		<details class="mb-4 text-xs">
			<summary class="cursor-pointer text-gray-500">Debug Info</summary>
			<pre class="mt-2 p-2 bg-gray-100 rounded text-xs overflow-auto">
isTauri: {isTauri}
window.__TAURI__: {typeof window !== 'undefined' ? '__TAURI__' in window : 'N/A'}
Computed style test: <span class="bg-red-500 text-white px-2">Should be red</span>
			</pre>
		</details>

		{#if !isTauri}
			<div class="bg-yellow-50 border-2 border-yellow-400 rounded-lg p-4 mb-6">
				<p class="text-yellow-800 text-sm font-medium mb-2">⚠️ Browser Mode</p>
				<p class="text-yellow-700 text-xs">
					Vault functionality requires the Tauri desktop app. Please run:
					<code class="block mt-2 bg-yellow-100 px-2 py-1 rounded font-mono text-xs"
						>cargo tauri dev</code
					>
				</p>
			</div>
		{/if}

		{#if vaultStore.loading}
			<div class="flex items-center justify-center py-8">
				<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
			</div>
		{:else if vaultStore.availableVaults.length === 0}
			{#if !showCreateForm}
				<div class="text-center py-8">
					<p class="text-gray-600 mb-4">No vaults found</p>
					<p class="text-sm text-gray-500 mb-6">Create a vault to get started</p>
					<button
						onclick={() => (showCreateForm = true)}
						class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 transition-colors"
						style="background-color: #0284c7; color: white; padding: 0.75rem 1.5rem; border-radius: 0.5rem; font-weight: 500;"
					>
						Create Vault
					</button>
				</div>
			{:else}
				<form
					onsubmit={(e) => {
						e.preventDefault();
						handleCreateVault();
					}}
					class="space-y-4"
				>
					<div>
						<label for="vault-id" class="block text-sm font-medium text-gray-700 mb-2">
							Vault ID
						</label>
						<input
							id="vault-id"
							type="text"
							bind:value={newVaultId}
							class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
							placeholder="my-vault"
							disabled={vaultStore.loading}
						/>
						<p class="text-xs text-gray-500 mt-1">Lowercase letters, numbers, and hyphens only</p>
					</div>

					<div>
						<label for="vault-name" class="block text-sm font-medium text-gray-700 mb-2">
							Display Name
						</label>
						<input
							id="vault-name"
							type="text"
							bind:value={newVaultName}
							class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
							placeholder="My Vault"
							disabled={vaultStore.loading}
						/>
					</div>

					<div>
						<label for="new-password" class="block text-sm font-medium text-gray-700 mb-2">
							Password
						</label>
						<input
							id="new-password"
							type="password"
							bind:value={newVaultPassword}
							class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
							placeholder="Choose a strong password"
							disabled={vaultStore.loading}
						/>
					</div>

					{#if vaultStore.error}
						<div class="bg-red-50 border border-red-200 rounded-md p-3">
							<p class="text-sm text-red-800">{vaultStore.error}</p>
						</div>
					{/if}

					<div class="flex gap-2">
						<button
							type="button"
							onclick={() => (showCreateForm = false)}
							disabled={vaultStore.loading}
							class="flex-1 px-4 py-2 border border-gray-300 text-gray-700 rounded-md hover:bg-gray-50 disabled:bg-gray-100 disabled:cursor-not-allowed transition-colors"
						>
							Cancel
						</button>
						<button
							type="submit"
							disabled={vaultStore.loading || !newVaultId || !newVaultName || !newVaultPassword}
							class="flex-1 bg-primary-600 text-white py-2 px-4 rounded-md hover:bg-primary-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
						>
							{vaultStore.loading ? 'Creating...' : 'Create Vault'}
						</button>
					</div>
				</form>
			{/if}
		{:else}
			<form
				onsubmit={(e) => {
					e.preventDefault();
					handleUnlock();
				}}
				class="space-y-4"
			>
				<!-- Vault Selection -->
				{#if vaultStore.availableVaults.length > 1}
					<div>
						<label for="vault-select" class="block text-sm font-medium text-gray-700 mb-2">
							Select Vault
						</label>
						<select
							id="vault-select"
							value={vaultStore.currentVaultId}
							onchange={(e) => vaultStore.setCurrentVault(e.currentTarget.value)}
							disabled={vaultStore.loading}
							class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
						>
							{#each vaultStore.availableVaults as vault}
								<option value={vault.vault_id}>
									{vault.display_name}
									{vault.unlocked ? ' (Unlocked)' : ''}
								</option>
							{/each}
						</select>
					</div>
				{:else}
					<div class="text-center py-2">
						<p class="text-lg font-medium text-gray-900">
							{vaultStore.availableVaults[0].display_name}
						</p>
					</div>
				{/if}

				<!-- Password Input -->
				<div>
					<label for="password" class="block text-sm font-medium text-gray-700 mb-2">
						Password
					</label>
					<div class="relative">
						<input
							id="password"
							type={showPassword ? 'text' : 'password'}
							bind:value={password}
							autocomplete="current-password"
							class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
							placeholder="Enter vault password"
							disabled={vaultStore.loading}
						/>
						<button
							type="button"
							onclick={() => (showPassword = !showPassword)}
							class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 hover:text-gray-700"
							aria-label={showPassword ? 'Hide password' : 'Show password'}
						>
							{showPassword ? 'Hide' : 'Show'}
						</button>
					</div>
				</div>

				<!-- Error Display -->
				{#if vaultStore.error}
					<div class="bg-red-50 border border-red-200 rounded-md p-3">
						<p class="text-sm text-red-800">{vaultStore.error}</p>
					</div>
				{/if}

				<!-- Unlock Button -->
				<button
					type="submit"
					disabled={vaultStore.loading || !password}
					class="w-full bg-primary-600 text-white py-2 px-4 rounded-md hover:bg-primary-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
				>
					{vaultStore.loading ? 'Unlocking...' : 'Unlock'}
				</button>

				<!-- Last Accessed Info -->
				{#if selectedVault}
					<div class="pt-4 border-t border-gray-200">
						<p class="text-xs text-gray-500 text-center">
							Last accessed: {selectedVault.last_accessed
								? new Date(selectedVault.last_accessed).toLocaleString()
								: 'Never'}
						</p>
					</div>
				{/if}
			</form>
		{/if}
	</div>
</div>
