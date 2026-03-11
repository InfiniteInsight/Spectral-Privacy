<script lang="ts">
	import { vaultStore } from '$lib/stores/vault.svelte';
	import { profileStore } from '$lib/stores';
	import { renameVault, deleteVault } from '$lib/api/vault';
	import type { ProfileOutput } from '$lib/api/profile';

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
	let confirmPassword = $state('');
	let showPassword = $state(false);

	// Expanded vault state
	let expandedVault = $state<string | null>(null);
	let vaultProfiles = $state<Record<string, ProfileOutput | null>>({});
	let loadingVaultData = $state<Record<string, boolean>>({});

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
		if (!newVaultId || !newVaultName || !newVaultPassword || !confirmPassword) return;
		if (newVaultPassword !== confirmPassword) {
			actionError = 'Passwords do not match';
			return;
		}
		actionError = null;
		actionLoading = true;
		try {
			await vaultStore.createVault(newVaultId, newVaultName, newVaultPassword);
			newVaultId = '';
			newVaultName = '';
			newVaultPassword = '';
			confirmPassword = '';
			showPassword = false;
			showCreateForm = false;
			await vaultStore.loadVaults();
		} catch (err) {
			actionError = err instanceof Error ? err.message : String(err);
		} finally {
			actionLoading = false;
		}
	}

	async function toggleVaultExpansion(vaultId: string) {
		// If clicking the already expanded vault, collapse it
		if (expandedVault === vaultId) {
			expandedVault = null;
			return;
		}

		// Only allow expansion if vault is unlocked
		if (!vaultStore.unlockedVaultIds.has(vaultId)) {
			return;
		}

		// Expand the vault
		expandedVault = vaultId;

		// Load vault data if not already loaded
		if (!vaultProfiles[vaultId]) {
			loadingVaultData[vaultId] = true;
			try {
				await profileStore.loadProfiles(vaultId);
				if (profileStore.profiles.length > 0) {
					const profileId = profileStore.profiles[0].id;
					await profileStore.loadProfile(vaultId, profileId);
					vaultProfiles[vaultId] = profileStore.currentProfile;
				} else {
					vaultProfiles[vaultId] = null;
				}
			} catch (err) {
				console.error('Failed to load vault data:', err);
				vaultProfiles[vaultId] = null;
			} finally {
				loadingVaultData[vaultId] = false;
			}
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
			<div class="rounded-lg border border-gray-200 bg-white overflow-hidden">
				{#if renameTarget === vault.vault_id}
					<div class="flex items-center gap-3 p-4">
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
					<!-- Vault header (clickable) -->
					<div
						class="flex items-center justify-between p-4 {vaultStore.unlockedVaultIds.has(
							vault.vault_id
						)
							? 'cursor-pointer hover:bg-gray-50'
							: ''}"
						onclick={(e) => {
							// Don't expand if clicking on buttons
							if ((e.target as HTMLElement).closest('button, a')) return;
							toggleVaultExpansion(vault.vault_id);
						}}
						role="button"
						tabindex="0"
						onkeydown={(e) => {
							if (e.key === 'Enter' || e.key === ' ') {
								e.preventDefault();
								toggleVaultExpansion(vault.vault_id);
							}
						}}
					>
						<div class="flex items-center gap-3">
							<!-- Expand/collapse chevron (only for unlocked vaults) -->
							{#if vaultStore.unlockedVaultIds.has(vault.vault_id)}
								<svg
									class="h-5 w-5 text-gray-400 transition-transform {expandedVault ===
									vault.vault_id
										? 'rotate-90'
										: ''}"
									fill="none"
									viewBox="0 0 24 24"
									stroke="currentColor"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M9 5l7 7-7 7"
									/>
								</svg>
							{/if}
							<div>
								<p class="font-medium text-gray-900">{vault.display_name}</p>
								<p class="text-xs text-gray-400">
									Last accessed: {new Date(vault.last_accessed).toLocaleDateString()}
								</p>
							</div>
						</div>
						<div class="flex gap-2" onclick={(e) => e.stopPropagation()}>
							{#if vaultStore.unlockedVaultIds.has(vault.vault_id)}
								<a
									href="/settings?tab=profile"
									onclick={async () => {
										await vaultStore.setCurrentVault(vault.vault_id);
									}}
									class="rounded-md border border-primary-200 px-3 py-1.5 text-xs text-primary-600 hover:bg-primary-50"
									>Edit Profile</a
								>
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

					<!-- Expanded content -->
					{#if expandedVault === vault.vault_id}
						<div class="border-t border-gray-200 bg-gray-50 p-4">
							{#if loadingVaultData[vault.vault_id]}
								<div class="flex items-center justify-center py-8">
									<div
										class="h-8 w-8 animate-spin rounded-full border-4 border-gray-200 border-t-primary-600"
									></div>
								</div>
							{:else if vaultProfiles[vault.vault_id]}
								{@const profile = vaultProfiles[vault.vault_id]!}
								<div class="space-y-4">
									<div>
										<h3 class="text-sm font-semibold text-gray-700 mb-3">Profile Information</h3>
										<div class="grid grid-cols-2 gap-3 text-sm">
											<div>
												<span class="text-gray-500">Name:</span>
												<span class="ml-2 text-gray-900"
													>{profile.first_name}
													{profile.middle_name || ''}
													{profile.last_name}</span
												>
											</div>
											{#if profile.date_of_birth}
												<div>
													<span class="text-gray-500">Date of Birth:</span>
													<span class="ml-2 text-gray-900"
														>{new Date(profile.date_of_birth).toLocaleDateString()}</span
													>
												</div>
											{/if}
											{#if profile.email}
												<div>
													<span class="text-gray-500">Email:</span>
													<span class="ml-2 text-gray-900">{profile.email}</span>
												</div>
											{/if}
											{#if profile.address_line1}
												<div>
													<span class="text-gray-500">Address:</span>
													<span class="ml-2 text-gray-900"
														>{profile.address_line1}, {profile.city}, {profile.state}</span
													>
												</div>
											{/if}
										</div>
									</div>

									{#if profile.email_addresses && profile.email_addresses.length > 0}
										<div>
											<h3 class="text-sm font-semibold text-gray-700 mb-2">Email Addresses</h3>
											<div class="space-y-1">
												{#each profile.email_addresses as emailAddr}
													<div class="text-sm text-gray-600">
														{emailAddr.email}
														<span class="text-xs text-gray-400">({emailAddr.email_type})</span>
													</div>
												{/each}
											</div>
										</div>
									{/if}

									{#if profile.phone_numbers && profile.phone_numbers.length > 0}
										<div>
											<h3 class="text-sm font-semibold text-gray-700 mb-2">Phone Numbers</h3>
											<div class="space-y-1">
												{#each profile.phone_numbers as phone}
													<div class="text-sm text-gray-600">
														{phone.number}
														<span class="text-xs text-gray-400">({phone.phone_type})</span>
													</div>
												{/each}
											</div>
										</div>
									{/if}

									{#if profile.aliases && profile.aliases.length > 0}
										<div>
											<h3 class="text-sm font-semibold text-gray-700 mb-2">Aliases</h3>
											<div class="space-y-1">
												{#each profile.aliases as alias}
													<div class="text-sm text-gray-600">
														{alias.first_name || ''}
														{alias.middle_name || ''}
														{alias.last_name || ''}
														{alias.nickname ? `"${alias.nickname}"` : ''}
													</div>
												{/each}
											</div>
										</div>
									{/if}

									{#if profile.relatives && profile.relatives.length > 0}
										<div>
											<h3 class="text-sm font-semibold text-gray-700 mb-2">Relatives</h3>
											<div class="space-y-1">
												{#each profile.relatives as relative}
													<div class="text-sm text-gray-600">
														{relative.first_name || ''}
														{relative.middle_name || ''}
														{relative.last_name || ''}
														<span class="text-xs text-gray-400">({relative.relationship})</span>
													</div>
												{/each}
											</div>
										</div>
									{/if}
								</div>
							{:else}
								<div class="py-8 text-center text-sm text-gray-500">
									No profile data found for this vault.
									<a href="/profile/setup" class="ml-1 text-primary-600 hover:text-primary-700"
										>Create a profile</a
									>
								</div>
							{/if}
						</div>
					{/if}
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
						<div class="relative">
							<input
								id="new-password"
								type={showPassword ? 'text' : 'password'}
								bind:value={newVaultPassword}
								class="w-full rounded-md border border-gray-300 px-3 py-2 pr-10 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
								placeholder="Choose a strong password"
								disabled={actionLoading}
							/>
							<button
								type="button"
								onclick={() => (showPassword = !showPassword)}
								class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
								aria-label={showPassword ? 'Hide password' : 'Show password'}
							>
								{#if showPassword}
									<svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"
										/>
									</svg>
								{:else}
									<svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
										/>
										<path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
										/>
									</svg>
								{/if}
							</button>
						</div>
					</div>

					<div>
						<label for="confirm-password" class="block text-sm font-medium text-gray-700 mb-1">
							Confirm Password
						</label>
						<input
							id="confirm-password"
							type={showPassword ? 'text' : 'password'}
							bind:value={confirmPassword}
							class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:border-primary-500 focus:outline-none focus:ring-1 focus:ring-primary-500"
							placeholder="Re-enter your password"
							disabled={actionLoading}
						/>
						{#if confirmPassword && newVaultPassword !== confirmPassword}
							<p class="mt-1 text-xs text-red-600">Passwords do not match</p>
						{/if}
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
								confirmPassword = '';
								showPassword = false;
								actionError = null;
							}}
							disabled={actionLoading}
							class="flex-1 rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50"
							>Cancel</button
						>
						<button
							type="submit"
							disabled={actionLoading ||
								!newVaultId ||
								!newVaultName ||
								!newVaultPassword ||
								!confirmPassword ||
								newVaultPassword !== confirmPassword}
							class="flex-1 rounded-md px-4 py-2 text-sm font-medium text-white disabled:opacity-50 disabled:cursor-not-allowed"
							style="background-color: {actionLoading ||
							!newVaultId ||
							!newVaultName ||
							!newVaultPassword ||
							!confirmPassword ||
							newVaultPassword !== confirmPassword
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
					autocomplete="off"
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
					autocomplete="off"
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
