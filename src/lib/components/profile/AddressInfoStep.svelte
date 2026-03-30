<script lang="ts">
	/* eslint-disable no-unused-vars */
	import type { ProfileInput, PreviousAddress } from '$lib/api/profile';
	import PreviousAddressModal from './shared/PreviousAddressModal.svelte';

	interface Props {
		profile: Partial<ProfileInput>;
		onUpdate: (updates: Partial<ProfileInput>) => void;
	}

	let { profile, onUpdate }: Props = $props();
	/* eslint-enable no-unused-vars */

	let previousAddresses = $state<PreviousAddress[]>([]);
	let showAddressModal = $state(false);
	let editingAddressIndex = $state<number | null>(null);
	let validationErrors = $state<Record<string, string>>({});

	// Sync with profile prop changes
	$effect(() => {
		previousAddresses = profile.previous_addresses ? [...profile.previous_addresses] : [];
	});

	function openAddressModal(index?: number) {
		editingAddressIndex = index ?? null;
		showAddressModal = true;
	}

	function saveAddress(address: PreviousAddress) {
		if (editingAddressIndex !== null) {
			previousAddresses = previousAddresses.map((a, i) =>
				i === editingAddressIndex ? address : a
			);
		} else {
			previousAddresses = [...previousAddresses, address];
		}
		onUpdate({ previous_addresses: previousAddresses });
		showAddressModal = false;
	}

	function removeAddress(index: number) {
		previousAddresses = previousAddresses.filter((_, i) => i !== index);
		onUpdate({ previous_addresses: previousAddresses });
	}

	export function validate(): boolean {
		const errors: Record<string, string> = {};

		if (!profile.address_line1?.trim()) {
			errors.address_line1 = 'Street address is required.';
		}
		if (!profile.city?.trim()) {
			errors.city = 'City is required.';
		}
		if (!profile.state?.trim()) {
			errors.state = 'State is required.';
		}
		if (!profile.zip_code?.trim()) {
			errors.zip_code = 'ZIP code is required.';
		} else if (!/^\d{5}(-\d{4})?$/.test(profile.zip_code.trim())) {
			errors.zip_code = 'ZIP code must be in 12345 or 12345-6789 format.';
		}

		validationErrors = errors;
		return Object.keys(errors).length === 0;
	}
</script>

<div class="space-y-6">
	<!-- Current Address Section -->
	<div>
		<h3 class="text-lg font-semibold mb-4">Current Address</h3>

		<div class="space-y-4">
			<div>
				<label class="block text-sm font-medium mb-2" for="address1"> Street Address </label>
				<input
					id="address1"
					type="text"
					value={profile.address_line1 || ''}
					oninput={(e) => onUpdate({ address_line1: e.currentTarget.value })}
					class="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500 {validationErrors.address_line1
						? 'border-red-500'
						: ''}"
					placeholder="123 Main Street"
				/>
				{#if validationErrors.address_line1}
					<p class="mt-1 text-xs text-red-600">{validationErrors.address_line1}</p>
				{/if}
			</div>

			<div>
				<label class="block text-sm font-medium mb-2" for="address2"> Apt/Suite (optional) </label>
				<input
					id="address2"
					type="text"
					value={profile.address_line2 || ''}
					oninput={(e) => onUpdate({ address_line2: e.currentTarget.value })}
					class="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
					placeholder="Apt 4B"
				/>
			</div>

			<div class="grid grid-cols-2 gap-4">
				<div>
					<label class="block text-sm font-medium mb-2" for="city"> City </label>
					<input
						id="city"
						type="text"
						value={profile.city || ''}
						oninput={(e) => onUpdate({ city: e.currentTarget.value })}
						class="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500 {validationErrors.city
							? 'border-red-500'
							: ''}"
						placeholder="Chicago"
					/>
					{#if validationErrors.city}
						<p class="mt-1 text-xs text-red-600">{validationErrors.city}</p>
					{/if}
				</div>

				<div>
					<label class="block text-sm font-medium mb-2" for="state"> State </label>
					<input
						id="state"
						type="text"
						value={profile.state || ''}
						oninput={(e) => onUpdate({ state: e.currentTarget.value })}
						class="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500 {validationErrors.state
							? 'border-red-500'
							: ''}"
						placeholder="IL"
						maxlength="2"
					/>
					{#if validationErrors.state}
						<p class="mt-1 text-xs text-red-600">{validationErrors.state}</p>
					{/if}
				</div>
			</div>

			<div>
				<label class="block text-sm font-medium mb-2" for="zip"> ZIP Code </label>
				<input
					id="zip"
					type="text"
					value={profile.zip_code || ''}
					oninput={(e) => onUpdate({ zip_code: e.currentTarget.value })}
					class="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500 {validationErrors.zip_code
						? 'border-red-500'
						: ''}"
					placeholder="60601"
					maxlength="10"
				/>
				{#if validationErrors.zip_code}
					<p class="mt-1 text-xs text-red-600">{validationErrors.zip_code}</p>
				{/if}
			</div>
		</div>
	</div>

	<!-- Previous Addresses Section -->
	<div>
		<h3 class="text-lg font-semibold mb-2">Previous Addresses</h3>
		<p class="text-sm text-gray-600 mb-3">
			Past addresses help remove outdated records from data brokers
		</p>

		{#if previousAddresses.length > 0}
			<div class="space-y-2 mb-3">
				{#each previousAddresses as addr, i (i)}
					<div class="flex items-start justify-between p-3 bg-gray-50 rounded-md">
						<div class="text-sm">
							<div class="font-medium">{addr.address_line1}</div>
							<div class="text-gray-600">
								{addr.city}, {addr.state}
								{addr.zip_code}
							</div>
							{#if addr.lived_from || addr.lived_to}
								<div class="text-gray-500 text-xs mt-1">
									{addr.lived_from ?? 'Unknown'} – {addr.lived_to ?? 'Unknown'}
								</div>
							{/if}
						</div>
						<div class="flex gap-2">
							<button
								onclick={() => openAddressModal(i)}
								class="text-blue-600 hover:text-blue-700 text-sm"
							>
								Edit
							</button>
							<button
								onclick={() => removeAddress(i)}
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
			onclick={() => openAddressModal()}
			class="text-blue-600 hover:text-blue-700 text-sm font-medium"
		>
			+ Add Previous Address
		</button>
	</div>
</div>

{#if showAddressModal}
	<PreviousAddressModal
		initialAddress={editingAddressIndex !== null
			? previousAddresses[editingAddressIndex]
			: undefined}
		onSave={saveAddress}
		onCancel={() => (showAddressModal = false)}
	/>
{/if}
