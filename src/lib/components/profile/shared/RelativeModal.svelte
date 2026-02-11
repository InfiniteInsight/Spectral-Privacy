<script lang="ts">
	/* eslint-disable no-unused-vars */
	import type { Relative } from '$lib/api/profile';

	interface Props {
		initialRelative?: Relative;
		onSave: (relative: Relative) => void;
		onCancel: () => void;
	}

	let { initialRelative, onSave, onCancel }: Props = $props();

	let formData = $state<Relative>(
		initialRelative || {
			name: '',
			relationship: 'Other'
		}
	);

	function handleSave() {
		if (!formData.name.trim()) {
			return;
		}

		onSave({
			name: formData.name.trim(),
			relationship: formData.relationship
		});
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			onCancel();
		}
	}
</script>

<div
	class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
	onclick={handleBackdropClick}
	role="dialog"
	aria-modal="true"
	aria-labelledby="modal-title"
>
	<div class="bg-white rounded-lg p-6 max-w-md w-full mx-4">
		<h2 id="modal-title" class="text-xl font-semibold mb-4">
			{initialRelative ? 'Edit' : 'Add'} Relative
		</h2>

		<div class="space-y-4">
			<div>
				<label class="block text-sm font-medium mb-1" for="name">
					Name <span class="text-red-500">*</span>
				</label>
				<input
					id="name"
					type="text"
					bind:value={formData.name}
					class="w-full px-3 py-2 border rounded-md"
					placeholder="Jane Doe"
					required
				/>
			</div>

			<div>
				<label class="block text-sm font-medium mb-1" for="relationship">
					Relationship <span class="text-red-500">*</span>
				</label>
				<select
					id="relationship"
					bind:value={formData.relationship}
					class="w-full px-3 py-2 border rounded-md"
					required
				>
					<option value="Spouse">Spouse</option>
					<option value="Partner">Partner</option>
					<option value="Parent">Parent</option>
					<option value="Child">Child</option>
					<option value="Sibling">Sibling</option>
					<option value="Other">Other</option>
				</select>
			</div>
		</div>

		<div class="flex justify-end gap-3 mt-6">
			<button onclick={onCancel} class="px-4 py-2 border rounded-md hover:bg-gray-50">
				Cancel
			</button>
			<button
				onclick={handleSave}
				class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
				disabled={!formData.name.trim()}
			>
				Save
			</button>
		</div>
	</div>
</div>
