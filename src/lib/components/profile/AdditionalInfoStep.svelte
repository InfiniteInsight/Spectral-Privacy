<script lang="ts">
	/* eslint-disable no-unused-vars */
	import type { ProfileInput, Relative, Alias } from '$lib/api/profile';
	import RelativeModal from './shared/RelativeModal.svelte';
	import AliasModal from './shared/AliasModal.svelte';

	interface Props {
		profile: Partial<ProfileInput>;
		onUpdate: (updates: Partial<ProfileInput>) => void;
	}

	let { profile, onUpdate }: Props = $props();
	/* eslint-enable no-unused-vars */

	let aliases = $state<Alias[]>(profile.aliases || []);
	let relatives = $state<Relative[]>(profile.relatives || []);
	let showAliasModal = $state(false);
	let showRelativeModal = $state(false);
	let editingAliasIndex = $state<number | null>(null);
	let editingRelativeIndex = $state<number | null>(null);

	function openAliasModal(index?: number) {
		editingAliasIndex = index ?? null;
		showAliasModal = true;
	}

	function saveAlias(alias: Alias) {
		if (editingAliasIndex !== null) {
			aliases = aliases.map((a, i) => (i === editingAliasIndex ? alias : a));
		} else {
			aliases = [...aliases, alias];
		}
		onUpdate({ aliases });
		showAliasModal = false;
	}

	function removeAlias(index: number) {
		aliases = aliases.filter((_, i) => i !== index);
		onUpdate({ aliases });
	}

	function formatAliasDisplay(alias: Alias): string {
		const parts = [alias.first_name, alias.middle_name, alias.last_name].filter(Boolean).join(' ');
		if (alias.nickname) {
			return parts ? `${parts} "${alias.nickname}"` : `"${alias.nickname}"`;
		}
		return parts;
	}

	function openRelativeModal(index?: number) {
		editingRelativeIndex = index ?? null;
		showRelativeModal = true;
	}

	function saveRelative(relative: Relative) {
		if (editingRelativeIndex !== null) {
			relatives = relatives.map((r, i) => (i === editingRelativeIndex ? relative : r));
		} else {
			relatives = [...relatives, relative];
		}
		onUpdate({ relatives });
		showRelativeModal = false;
	}

	function removeRelative(index: number) {
		relatives = relatives.filter((_, i) => i !== index);
		onUpdate({ relatives });
	}

	function formatRelativeDisplay(relative: Relative): string {
		const parts = [relative.first_name, relative.middle_name, relative.last_name]
			.filter(Boolean)
			.join(' ');
		if (relative.maiden_name) {
			return parts
				? `${parts} (maiden: ${relative.maiden_name})`
				: `(maiden: ${relative.maiden_name})`;
		}
		return parts || 'Unnamed';
	}

	export function validate(): boolean {
		// All fields optional - always valid
		return true;
	}
</script>

<div class="space-y-6">
	<!-- Aliases Section -->
	<div>
		<label class="block text-sm font-medium mb-2"> Aliases & Former Names </label>
		<p class="text-sm text-gray-600 mb-3">
			Include nicknames, maiden names, or other names you've used
		</p>

		{#if aliases.length > 0}
			<div class="space-y-2 mb-3">
				{#each aliases as alias, i (i)}
					<div class="flex items-center justify-between p-3 bg-gray-50 rounded-md">
						<div class="text-sm font-medium">{formatAliasDisplay(alias)}</div>
						<div class="flex gap-2">
							<button
								onclick={() => openAliasModal(i)}
								class="text-blue-600 hover:text-blue-700 text-sm"
							>
								Edit
							</button>
							<button
								onclick={() => removeAlias(i)}
								class="text-red-600 hover:text-red-700 text-sm"
							>
								Remove
							</button>
						</div>
					</div>
				{/each}
			</div>
		{/if}

		<button
			onclick={() => openAliasModal()}
			class="text-blue-600 hover:text-blue-700 text-sm font-medium"
		>
			+ Add Alias
		</button>
	</div>

	<!-- Relatives Section -->
	<div>
		<label class="block text-sm font-medium mb-2"> Family Members & Relatives </label>
		<p class="text-sm text-gray-600 mb-3">
			Data brokers often list relatives - adding them helps identify these records
		</p>

		{#if relatives.length > 0}
			<div class="space-y-2 mb-3">
				{#each relatives as relative, i (i)}
					<div class="flex items-center justify-between p-3 bg-gray-50 rounded-md">
						<div class="text-sm">
							<div class="font-medium">{formatRelativeDisplay(relative)}</div>
							<div class="text-gray-600">{relative.relationship}</div>
						</div>
						<div class="flex gap-2">
							<button
								onclick={() => openRelativeModal(i)}
								class="text-blue-600 hover:text-blue-700 text-sm"
							>
								Edit
							</button>
							<button
								onclick={() => removeRelative(i)}
								class="text-red-600 hover:text-red-700 text-sm"
							>
								Remove
							</button>
						</div>
					</div>
				{/each}
			</div>
		{/if}

		<button
			onclick={() => openRelativeModal()}
			class="text-blue-600 hover:text-blue-700 text-sm font-medium"
		>
			+ Add Relative
		</button>
	</div>
</div>

{#if showAliasModal}
	<AliasModal
		initialAlias={editingAliasIndex !== null ? aliases[editingAliasIndex] : undefined}
		onSave={saveAlias}
		onCancel={() => (showAliasModal = false)}
	/>
{/if}

{#if showRelativeModal}
	<RelativeModal
		initialRelative={editingRelativeIndex !== null ? relatives[editingRelativeIndex] : undefined}
		onSave={saveRelative}
		onCancel={() => (showRelativeModal = false)}
	/>
{/if}
