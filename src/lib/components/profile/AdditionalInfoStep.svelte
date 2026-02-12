<script lang="ts">
	/* eslint-disable no-unused-vars */
	import type { ProfileInput, Relative } from '$lib/api/profile';
	import RelativeModal from './shared/RelativeModal.svelte';

	interface Props {
		profile: Partial<ProfileInput>;
		onUpdate: (updates: Partial<ProfileInput>) => void;
	}

	let { profile, onUpdate }: Props = $props();
	/* eslint-enable no-unused-vars */

	let aliases = $state<string[]>(profile.aliases || []);
	let relatives = $state<Relative[]>(profile.relatives || []);
	let showRelativeModal = $state(false);
	let editingRelativeIndex = $state<number | null>(null);

	function addAlias() {
		aliases = [...aliases, ''];
	}

	function updateAlias(index: number, value: string) {
		aliases = aliases.map((a, i) => (i === index ? value : a));
		// Filter out empty aliases before updating
		const nonEmpty = aliases.filter((a) => a.trim());
		onUpdate({ aliases: nonEmpty });
	}

	function removeAlias(index: number) {
		aliases = aliases.filter((_, i) => i !== index);
		onUpdate({ aliases });
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
					<div class="flex gap-2">
						<input
							type="text"
							value={alias}
							oninput={(e) => updateAlias(i, e.currentTarget.value)}
							class="flex-1 px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
							placeholder="Former or alternate name"
						/>
						<button
							onclick={() => removeAlias(i)}
							class="px-3 py-2 text-red-600 hover:bg-red-50 rounded-md"
							aria-label="Remove alias"
						>
							Remove
						</button>
					</div>
				{/each}
			</div>
		{/if}

		<button onclick={addAlias} class="text-blue-600 hover:text-blue-700 text-sm font-medium">
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
							<div class="font-medium">{relative.name}</div>
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

{#if showRelativeModal}
	<RelativeModal
		initialRelative={editingRelativeIndex !== null ? relatives[editingRelativeIndex] : undefined}
		onSave={saveRelative}
		onCancel={() => (showRelativeModal = false)}
	/>
{/if}
