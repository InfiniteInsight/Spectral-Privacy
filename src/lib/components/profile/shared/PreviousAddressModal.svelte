<script lang="ts">
	/* eslint-disable no-unused-vars */
	import type { PreviousAddress } from '$lib/api/profile';

	interface Props {
		initialAddress?: PreviousAddress;
		onSave: (address: PreviousAddress) => void;
		onCancel: () => void;
	}

	let { initialAddress, onSave, onCancel }: Props = $props();

	let formData = $state<PreviousAddress>(
		initialAddress || {
			address_line1: '',
			address_line2: '',
			city: '',
			state: '',
			zip_code: '',
			lived_from: '',
			lived_to: ''
		}
	);

	function handleSave() {
		// Validate required fields
		if (!formData.address_line1 || !formData.city || !formData.state || !formData.zip_code) {
			return;
		}

		// Clean up empty optional fields
		const cleaned: PreviousAddress = {
			address_line1: formData.address_line1,
			city: formData.city,
			state: formData.state,
			zip_code: formData.zip_code
		};

		if (formData.address_line2) cleaned.address_line2 = formData.address_line2;
		if (formData.lived_from) cleaned.lived_from = formData.lived_from;
		if (formData.lived_to) cleaned.lived_to = formData.lived_to;

		onSave(cleaned);
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
			{initialAddress ? 'Edit' : 'Add'} Previous Address
		</h2>

		<div class="space-y-4">
			<div>
				<label class="block text-sm font-medium mb-1" for="addr1">
					Street Address <span class="text-red-500">*</span>
				</label>
				<input
					id="addr1"
					type="text"
					bind:value={formData.address_line1}
					class="w-full px-3 py-2 border rounded-md"
					placeholder="123 Main Street"
					required
				/>
			</div>

			<div>
				<label class="block text-sm font-medium mb-1" for="addr2"> Apt/Suite (optional) </label>
				<input
					id="addr2"
					type="text"
					bind:value={formData.address_line2}
					class="w-full px-3 py-2 border rounded-md"
					placeholder="Apt 4B"
				/>
			</div>

			<div class="grid grid-cols-2 gap-4">
				<div>
					<label class="block text-sm font-medium mb-1" for="city">
						City <span class="text-red-500">*</span>
					</label>
					<input
						id="city"
						type="text"
						bind:value={formData.city}
						class="w-full px-3 py-2 border rounded-md"
						placeholder="Chicago"
						required
					/>
				</div>

				<div>
					<label class="block text-sm font-medium mb-1" for="state">
						State <span class="text-red-500">*</span>
					</label>
					<input
						id="state"
						type="text"
						bind:value={formData.state}
						class="w-full px-3 py-2 border rounded-md"
						placeholder="IL"
						maxlength="2"
						required
					/>
				</div>
			</div>

			<div>
				<label class="block text-sm font-medium mb-1" for="zip">
					ZIP Code <span class="text-red-500">*</span>
				</label>
				<input
					id="zip"
					type="text"
					bind:value={formData.zip_code}
					class="w-full px-3 py-2 border rounded-md"
					placeholder="60601"
					maxlength="10"
					required
				/>
			</div>

			<div class="grid grid-cols-2 gap-4">
				<div>
					<label class="block text-sm font-medium mb-1" for="from"> Lived From </label>
					<input
						id="from"
						type="date"
						bind:value={formData.lived_from}
						class="w-full px-3 py-2 border rounded-md"
					/>
				</div>

				<div>
					<label class="block text-sm font-medium mb-1" for="to"> Lived To </label>
					<input
						id="to"
						type="date"
						bind:value={formData.lived_to}
						class="w-full px-3 py-2 border rounded-md"
					/>
				</div>
			</div>
		</div>

		<div class="flex justify-end gap-3 mt-6">
			<button onclick={onCancel} class="px-4 py-2 border rounded-md hover:bg-gray-50">
				Cancel
			</button>
			<button
				onclick={handleSave}
				class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
				disabled={!formData.address_line1 ||
					!formData.city ||
					!formData.state ||
					!formData.zip_code}
			>
				Save
			</button>
		</div>
	</div>
</div>
