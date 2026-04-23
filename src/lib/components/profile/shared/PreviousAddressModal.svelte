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

	const currentYear = new Date().getFullYear();
	const years = Array.from({ length: currentYear - 1919 }, (_, i) => currentYear - i);
	const months = [
		{ value: '01', label: '01 - January' },
		{ value: '02', label: '02 - February' },
		{ value: '03', label: '03 - March' },
		{ value: '04', label: '04 - April' },
		{ value: '05', label: '05 - May' },
		{ value: '06', label: '06 - June' },
		{ value: '07', label: '07 - July' },
		{ value: '08', label: '08 - August' },
		{ value: '09', label: '09 - September' },
		{ value: '10', label: '10 - October' },
		{ value: '11', label: '11 - November' },
		{ value: '12', label: '12 - December' }
	];

	function parseYearMonth(value: string | undefined): { year: string; month: string } {
		if (!value) return { year: '', month: '' };
		const parts = value.split('-');
		return { year: parts[0] ?? '', month: parts[1] ?? '' };
	}

	let fromParts = $state(parseYearMonth(initialAddress?.lived_from));
	let toParts = $state(parseYearMonth(initialAddress?.lived_to));

	function updateFrom() {
		formData.lived_from =
			fromParts.year && fromParts.month ? `${fromParts.year}-${fromParts.month}` : '';
	}

	function updateTo() {
		formData.lived_to = toParts.year && toParts.month ? `${toParts.year}-${toParts.month}` : '';
	}

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

			<div class="space-y-4">
				<div>
					<label class="block text-sm font-medium mb-1">Lived From</label>
					<div class="flex gap-1">
						<select
							bind:value={fromParts.month}
							onchange={updateFrom}
							class="flex-1 px-2 py-2 border rounded-md text-sm"
						>
							<option value="">Month</option>
							{#each months as m}
								<option value={m.value}>{m.label}</option>
							{/each}
						</select>
						<select
							bind:value={fromParts.year}
							onchange={updateFrom}
							class="w-24 px-2 py-2 border rounded-md text-sm"
						>
							<option value="">Year</option>
							{#each years as y}
								<option value={String(y)}>{y}</option>
							{/each}
						</select>
					</div>
				</div>

				<div>
					<label class="block text-sm font-medium mb-1">Lived To</label>
					<div class="flex gap-1">
						<select
							bind:value={toParts.month}
							onchange={updateTo}
							class="flex-1 px-2 py-2 border rounded-md text-sm"
						>
							<option value="">Month</option>
							{#each months as m}
								<option value={m.value}>{m.label}</option>
							{/each}
						</select>
						<select
							bind:value={toParts.year}
							onchange={updateTo}
							class="w-24 px-2 py-2 border rounded-md text-sm"
						>
							<option value="">Year</option>
							{#each years as y}
								<option value={String(y)}>{y}</option>
							{/each}
						</select>
					</div>
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
