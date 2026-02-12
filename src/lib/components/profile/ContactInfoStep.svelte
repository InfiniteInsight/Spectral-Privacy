<script lang="ts">
	/* eslint-disable no-unused-vars */
	import type { ProfileInput, PhoneNumber } from '$lib/api/profile';

	interface Props {
		profile: Partial<ProfileInput>;
		onUpdate: (updates: Partial<ProfileInput>) => void;
	}

	let { profile, onUpdate }: Props = $props();

	let phoneNumbers = $state<PhoneNumber[]>(profile.phone_numbers || []);

	function addPhoneNumber() {
		phoneNumbers = [...phoneNumbers, { number: '', phone_type: 'Mobile' }];
	}

	function removePhoneNumber(index: number) {
		phoneNumbers = phoneNumbers.filter((_, i) => i !== index);
		onUpdate({ phone_numbers: phoneNumbers });
	}

	function updatePhoneNumber(index: number, field: keyof PhoneNumber, value: unknown) {
		phoneNumbers = phoneNumbers.map((phone, i) =>
			i === index ? { ...phone, [field]: value } : phone
		);
		onUpdate({ phone_numbers: phoneNumbers });
	}

	export function validate(): boolean {
		// All fields optional - always valid
		return true;
	}
</script>

<div class="space-y-6">
	<div>
		<label class="block text-sm font-medium mb-2" for="email"> Email Address </label>
		<input
			id="email"
			type="email"
			value={profile.email || ''}
			oninput={(e) => onUpdate({ email: e.currentTarget.value })}
			class="w-full px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
			placeholder="your.email@example.com"
		/>
		<p class="text-xs text-gray-600 mt-1">Used to match records on data broker sites</p>
	</div>

	<div>
		<label class="block text-sm font-medium mb-2"> Phone Numbers </label>
		<p class="text-sm text-gray-600 mb-3">
			Adding phone numbers helps identify records across more data brokers
		</p>

		{#if phoneNumbers.length > 0}
			<div class="space-y-2 mb-3">
				{#each phoneNumbers as phone, i (i)}
					<div class="flex gap-2">
						<input
							type="tel"
							value={phone.number}
							oninput={(e) => updatePhoneNumber(i, 'number', e.currentTarget.value)}
							class="flex-1 px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
							placeholder="(555) 123-4567"
						/>
						<select
							value={phone.phone_type}
							onchange={(e) => updatePhoneNumber(i, 'phone_type', e.currentTarget.value)}
							class="px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
						>
							<option value="Mobile">Mobile</option>
							<option value="Home">Home</option>
							<option value="Work">Work</option>
						</select>
						<button
							onclick={() => removePhoneNumber(i)}
							class="px-3 py-2 text-red-600 hover:bg-red-50 rounded-md"
							aria-label="Remove phone number"
						>
							Remove
						</button>
					</div>
				{/each}
			</div>
		{/if}

		<button onclick={addPhoneNumber} class="text-blue-600 hover:text-blue-700 text-sm font-medium">
			+ Add Phone Number
		</button>
	</div>
</div>
