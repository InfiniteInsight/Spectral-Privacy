<script lang="ts">
	/* eslint-disable no-unused-vars */
	import type { ProfileInput, PhoneNumber, EmailAddress } from '$lib/api/profile';

	interface Props {
		profile: Partial<ProfileInput>;
		onUpdate: (updates: Partial<ProfileInput>) => void;
	}

	let { profile, onUpdate }: Props = $props();

	let emailAddresses = $state<EmailAddress[]>(profile.email_addresses || []);
	let emailErrors = $state<(string | null)[]>([]);
	let phoneNumbers = $state<PhoneNumber[]>(profile.phone_numbers || []);
	let phoneErrors = $state<(string | null)[]>([]);

	function addEmailAddress() {
		emailAddresses = [...emailAddresses, { email: '', email_type: 'Personal' }];
		emailErrors = [...emailErrors, null];
	}

	function removeEmailAddress(index: number) {
		emailAddresses = emailAddresses.filter((_, i) => i !== index);
		emailErrors = emailErrors.filter((_, i) => i !== index);
		onUpdate({ email_addresses: emailAddresses });
	}

	function normalizeEmail(email: string): string | null {
		const trimmed = email.trim();
		if (!trimmed) return null;

		// Basic validation
		if (!trimmed.includes('@') || trimmed.split('@').length !== 2) {
			return null;
		}

		return trimmed.toLowerCase();
	}

	function validateEmail(email: string): string | null {
		if (!email.trim()) {
			return null; // Empty is okay (optional field)
		}

		const normalized = normalizeEmail(email);
		if (!normalized) {
			return 'Invalid email format';
		}

		return null;
	}

	function updateEmailAddress(index: number, field: keyof EmailAddress, value: unknown) {
		emailAddresses = emailAddresses.map((email, i) => {
			if (i === index) {
				const updated = { ...email, [field]: value };
				if (field === 'email') {
					// Validate and update error
					emailErrors[i] = validateEmail(value as string);
					// Add normalized version
					const normalized = normalizeEmail(value as string);
					if (normalized) {
						updated.email_normalized = normalized;
					}
				}
				return updated;
			}
			return email;
		});
		onUpdate({ email_addresses: emailAddresses });
	}

	function addPhoneNumber() {
		phoneNumbers = [...phoneNumbers, { number: '', phone_type: 'Mobile' }];
		phoneErrors = [...phoneErrors, null];
	}

	function removePhoneNumber(index: number) {
		phoneNumbers = phoneNumbers.filter((_, i) => i !== index);
		phoneErrors = phoneErrors.filter((_, i) => i !== index);
		onUpdate({ phone_numbers: phoneNumbers });
	}

	function normalizePhoneNumber(phone: string): string | null {
		// Extract digits only
		const digits = phone.replace(/\D/g, '');

		// Handle US country code
		if (digits.length === 11 && digits.startsWith('1')) {
			return digits.substring(1);
		} else if (digits.length === 10) {
			return digits;
		}

		return null;
	}

	function validatePhoneNumber(phone: string): string | null {
		if (!phone.trim()) {
			return null; // Empty is okay (optional field)
		}

		const normalized = normalizePhoneNumber(phone);
		if (!normalized) {
			const digits = phone.replace(/\D/g, '');
			if (digits.length < 10) {
				return `Too short (need 10 digits, have ${digits.length})`;
			} else {
				return `Too long (need 10 digits, have ${digits.length})`;
			}
		}

		return null;
	}

	function updatePhoneNumber(index: number, field: keyof PhoneNumber, value: unknown) {
		phoneNumbers = phoneNumbers.map((phone, i) => {
			if (i === index) {
				const updated = { ...phone, [field]: value };
				if (field === 'number') {
					// Validate and update error
					phoneErrors[i] = validatePhoneNumber(value as string);
					// Add normalized version
					const normalized = normalizePhoneNumber(value as string);
					if (normalized) {
						updated.number_normalized = normalized;
					}
				}
				return updated;
			}
			return phone;
		});
		onUpdate({ phone_numbers: phoneNumbers });
	}

	export function validate(): boolean {
		// Require at least one email address
		const hasValidEmail =
			emailAddresses.length > 0 &&
			emailAddresses.some((email) => email.email.trim() !== '') &&
			emailErrors.every((error) => error === null);

		if (!hasValidEmail) {
			alert('Please add at least one email address');
			return false;
		}

		// Check all phone numbers are valid
		if (!phoneErrors.every((error) => error === null)) {
			alert('Please fix the invalid phone numbers');
			return false;
		}

		return true;
	}
</script>

<div class="space-y-6">
	<div>
		<label class="block text-sm font-medium mb-2"> Email Addresses </label>
		<p class="text-sm text-gray-600 mb-3">
			Adding email addresses helps identify records across more data brokers. Emails are
			case-insensitive and will be normalized for matching.
		</p>

		{#if emailAddresses.length > 0}
			<div class="space-y-3 mb-3">
				{#each emailAddresses as email, i (i)}
					<div>
						<div class="flex gap-2">
							<input
								type="email"
								value={email.email}
								oninput={(e) => updateEmailAddress(i, 'email', e.currentTarget.value)}
								class="flex-1 px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500 {emailErrors[
									i
								]
									? 'border-red-500'
									: ''}"
								placeholder="your.email@example.com"
							/>
							<select
								value={email.email_type}
								onchange={(e) => updateEmailAddress(i, 'email_type', e.currentTarget.value)}
								class="px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500"
							>
								<option value="Personal">Personal</option>
								<option value="Work">Work</option>
								<option value="Other">Other</option>
							</select>
							<button
								onclick={() => removeEmailAddress(i)}
								class="px-3 py-2 text-red-600 hover:bg-red-50 rounded-md"
								aria-label="Remove email address"
							>
								Remove
							</button>
						</div>
						{#if emailErrors[i]}
							<p class="text-sm text-red-600 mt-1">{emailErrors[i]}</p>
						{/if}
					</div>
				{/each}
			</div>
		{/if}

		<button onclick={addEmailAddress} class="text-blue-600 hover:text-blue-700 text-sm font-medium">
			+ Add Email Address
		</button>
	</div>

	<div>
		<label class="block text-sm font-medium mb-2"> Phone Numbers </label>
		<p class="text-sm text-gray-600 mb-3">
			Adding phone numbers helps identify records across more data brokers. Enter in any format —
			they'll be normalized for matching (e.g., "(555) 123-4567" matches "555-123-4567").
		</p>

		{#if phoneNumbers.length > 0}
			<div class="space-y-3 mb-3">
				{#each phoneNumbers as phone, i (i)}
					<div>
						<div class="flex gap-2">
							<input
								type="tel"
								value={phone.number}
								oninput={(e) => updatePhoneNumber(i, 'number', e.currentTarget.value)}
								class="flex-1 px-3 py-2 border rounded-md focus:ring-2 focus:ring-blue-500 {phoneErrors[
									i
								]
									? 'border-red-500'
									: ''}"
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
						{#if phoneErrors[i]}
							<p class="text-sm text-red-600 mt-1">{phoneErrors[i]}</p>
						{/if}
					</div>
				{/each}
			</div>
		{/if}

		<button onclick={addPhoneNumber} class="text-blue-600 hover:text-blue-700 text-sm font-medium">
			+ Add Phone Number
		</button>
	</div>
</div>
