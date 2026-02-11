<script lang="ts">
	/* eslint-disable no-unused-vars */
	import { FormField, DisabledFieldNotice } from './shared';
	import type { ProfileInput } from '$lib/api';

	interface Props {
		data: Partial<ProfileInput>;
		onchange: (data: Partial<ProfileInput>) => void;
	}

	let { data = $bindable({}), onchange }: Props = $props();
	/* eslint-enable no-unused-vars */

	// Validation state
	let errors = $state<Record<string, string>>({});

	// Validate email
	function validateEmail(value: string): string {
		if (!value.trim()) return 'Email is required';

		// Simplified RFC 5322 email validation
		const emailRegex = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;
		if (!emailRegex.test(value)) return 'Invalid email format';

		if (value.length > 254) return 'Email is too long';

		return '';
	}

	// Handle email change
	function handleEmailChange(value: string) {
		data.email = value;
		errors.email = validateEmail(value);
		onchange(data);
	}

	// Expose validation function for parent
	export function validate(): boolean {
		const newErrors: Record<string, string> = {};

		newErrors.email = validateEmail(data.email || '');

		errors = newErrors;

		return !Object.values(newErrors).some((err) => err !== '');
	}
</script>

<div class="space-y-4">
	<div>
		<h2 class="text-xl font-semibold text-gray-900 mb-2">Contact Information</h2>
		<p class="text-sm text-gray-600 mb-6">
			Provide your email address. This is used to match your information on data broker sites.
		</p>
	</div>

	<FormField
		label="Email Address"
		id="email"
		type="email"
		value={data.email || ''}
		error={errors.email}
		required={true}
		placeholder="john.doe@example.com"
		onchange={handleEmailChange}
	/>

	<DisabledFieldNotice fieldName="Phone Numbers" />

	<div class="mt-4 p-3 bg-blue-50 border border-blue-200 rounded-md">
		<p class="text-xs text-blue-800">
			<strong>Why we need this:</strong> Data brokers often index profiles by email address. Providing
			your email helps us find and remove more of your information.
		</p>
	</div>
</div>
