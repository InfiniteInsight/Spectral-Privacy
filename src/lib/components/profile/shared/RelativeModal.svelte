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
			first_name: '',
			middle_name: '',
			last_name: '',
			maiden_name: '',
			relationship: 'Other'
		}
	);

	function handleSave() {
		// At least one name field must be filled
		if (
			!formData.first_name?.trim() &&
			!formData.middle_name?.trim() &&
			!formData.last_name?.trim() &&
			!formData.maiden_name?.trim()
		) {
			return;
		}

		onSave({
			first_name: formData.first_name?.trim() || undefined,
			middle_name: formData.middle_name?.trim() || undefined,
			last_name: formData.last_name?.trim() || undefined,
			maiden_name: formData.maiden_name?.trim() || undefined,
			relationship: formData.relationship
		});
	}

	const isValid = $derived(
		formData.first_name?.trim() ||
			formData.middle_name?.trim() ||
			formData.last_name?.trim() ||
			formData.maiden_name?.trim()
	);

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
	tabindex="0"
>
	<div class="bg-white rounded-lg p-6 max-w-md w-full mx-4">
		<h2 id="modal-title" class="text-xl font-semibold mb-4">
			{initialRelative ? 'Edit' : 'Add'} Relative
		</h2>

		<p class="text-sm text-gray-600 mb-4">
			Fill in at least one name field. Include maiden name if applicable (for tracking relatives who
			changed their last name).
		</p>

		<div class="space-y-4">
			<div>
				<label class="block text-sm font-medium mb-1" for="first_name"> First Name </label>
				<input
					id="first_name"
					type="text"
					bind:value={formData.first_name}
					class="w-full px-3 py-2 border rounded-md"
					placeholder="Jane"
				/>
			</div>

			<div>
				<label class="block text-sm font-medium mb-1" for="middle_name"> Middle Name </label>
				<input
					id="middle_name"
					type="text"
					bind:value={formData.middle_name}
					class="w-full px-3 py-2 border rounded-md"
					placeholder="Marie"
				/>
			</div>

			<div>
				<label class="block text-sm font-medium mb-1" for="last_name"> Last Name </label>
				<input
					id="last_name"
					type="text"
					bind:value={formData.last_name}
					class="w-full px-3 py-2 border rounded-md"
					placeholder="Doe"
				/>
			</div>

			<div>
				<label class="block text-sm font-medium mb-1" for="maiden_name">
					Maiden Name (if applicable)
				</label>
				<input
					id="maiden_name"
					type="text"
					bind:value={formData.maiden_name}
					class="w-full px-3 py-2 border rounded-md"
					placeholder="Smith"
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
				disabled={!isValid}
			>
				Save
			</button>
		</div>
	</div>
</div>
