<script lang="ts">
	import { page } from '$app/stores';
	import { vaultStore } from '$lib/stores/vault.svelte';
	import { renameVault, deleteVault } from '$lib/api/vault';

	// Tab from query param: ?tab=vaults (default), privacy, email, scheduling, audit
	let activeTab = $derived($page.url.searchParams.get('tab') ?? 'vaults');

	// Vault management state
	let renameTarget = $state<string | null>(null);
	let renameValue = $state('');
	let deleteTarget = $state<string | null>(null);
	let deletePassword = $state('');
	let actionError = $state<string | null>(null);
	let actionLoading = $state(false);

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
</script>

<div class="mx-auto max-w-3xl px-4 py-8">
	<h1 class="mb-6 text-2xl font-bold text-gray-900">Settings</h1>

	<!-- Tab bar -->
	<div class="mb-8 flex gap-1 border-b border-gray-200">
		{#each [['vaults', 'Vaults'], ['privacy', 'Privacy Level'], ['email', 'Email'], ['scheduling', 'Scheduling'], ['audit', 'Audit Log']] as [id, label] (id)}
			<a
				href="/settings?tab={id}"
				class="px-4 py-2 text-sm font-medium {activeTab === id
					? 'border-b-2 border-primary-600 text-primary-700'
					: 'text-gray-500 hover:text-gray-700'}">{label}</a
			>
		{/each}
	</div>

	<!-- Vaults tab -->
	{#if activeTab === 'vaults'}
		<section>
			<h2 class="mb-4 text-lg font-semibold text-gray-800">Your vaults</h2>
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
									>Save</button
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
			{#if actionError && (renameTarget !== null || deleteTarget !== null)}
				<p class="mt-2 text-sm text-red-600">{actionError}</p>
			{/if}
			<div class="mt-4">
				<a
					href="/profile/setup"
					class="inline-block rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50"
					>+ Add new vault</a
				>
			</div>
		</section>
	{:else if activeTab === 'privacy'}
		<section>
			<h2 class="mb-2 text-lg font-semibold text-gray-800">Privacy Level</h2>
			<p class="mb-4 text-sm text-gray-500">Coming soon — permission presets</p>
		</section>
	{:else if activeTab === 'email'}
		<section>
			<h2 class="mb-2 text-lg font-semibold text-gray-800">Email</h2>
			<p class="mb-4 text-sm text-gray-500">Email settings will appear here (Phase 6 Task 15)</p>
		</section>
	{:else if activeTab === 'scheduling'}
		<section>
			<h2 class="mb-2 text-lg font-semibold text-gray-800">Scheduling</h2>
			<p class="mb-4 text-sm text-gray-500">
				Scheduling settings will appear here (Phase 6 Task 20)
			</p>
		</section>
	{:else if activeTab === 'audit'}
		<section>
			<h2 class="mb-2 text-lg font-semibold text-gray-800">Privacy Audit Log</h2>
			<p class="mb-4 text-sm text-gray-500">Audit log will appear here (Phase 6 Task 5)</p>
		</section>
	{/if}

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
</div>
