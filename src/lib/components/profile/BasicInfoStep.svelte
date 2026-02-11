<script lang="ts">
	/* eslint-disable no-unused-vars */
	import { FormField } from './shared';
	import type { ProfileInput } from '$lib/api';

	interface Props {
		data: Partial<ProfileInput>;
		onchange: (data: Partial<ProfileInput>) => void;
	}

	let { data = $bindable({}), onchange }: Props = $props();
	/* eslint-enable no-unused-vars */

	// Validation state
	let errors = $state<Record<string, string>>({});

	// Validate fields
	function validateFirstName(value: string): string {
		if (!value.trim()) return 'First name is required';
		if (!/^[a-zA-Z\s'-]+$/.test(value))
			return 'Only letters, spaces, hyphens, and apostrophes allowed';
		return '';
	}

	function validateLastName(value: string): string {
		if (!value.trim()) return 'Last name is required';
		if (!/^[a-zA-Z\s'-]+$/.test(value))
			return 'Only letters, spaces, hyphens, and apostrophes allowed';
		return '';
	}

	function validateMiddleName(value: string): string {
		if (value && !/^[a-zA-Z\s'-]+$/.test(value)) {
			return 'Only letters, spaces, hyphens, and apostrophes allowed';
		}
		return '';
	}

	function validateDateOfBirth(value: string): string {
		if (!value) return ''; // Optional field

		const dob = new Date(value);
		const today = new Date();
		const age = today.getFullYear() - dob.getFullYear();
		const monthDiff = today.getMonth() - dob.getMonth();
		const dayDiff = today.getDate() - dob.getDate();

		const actualAge = monthDiff < 0 || (monthDiff === 0 && dayDiff < 0) ? age - 1 : age;

		if (actualAge < 13) return 'Must be at least 13 years old';
		if (actualAge > 120) return 'Invalid date of birth';

		return '';
	}

	// Handle field changes
	function handleFirstNameChange(value: string) {
		data.first_name = value;
		errors.first_name = validateFirstName(value);
		onchange(data);
	}

	function handleMiddleNameChange(value: string) {
		data.middle_name = value || undefined;
		errors.middle_name = validateMiddleName(value);
		onchange(data);
	}

	function handleLastNameChange(value: string) {
		data.last_name = value;
		errors.last_name = validateLastName(value);
		onchange(data);
	}

	function handleDateOfBirthChange(value: string) {
		data.date_of_birth = value || undefined;
		errors.date_of_birth = validateDateOfBirth(value);
		onchange(data);
	}

	// Expose validation function for parent
	export function validate(): boolean {
		const newErrors: Record<string, string> = {};

		newErrors.first_name = validateFirstName(data.first_name || '');
		newErrors.last_name = validateLastName(data.last_name || '');
		newErrors.middle_name = validateMiddleName(data.middle_name || '');
		newErrors.date_of_birth = validateDateOfBirth(data.date_of_birth || '');

		errors = newErrors;

		return !Object.values(newErrors).some((err) => err !== '');
	}
</script>

<div class="space-y-4">
	<div>
		<h2 class="text-xl font-semibold text-gray-900 mb-2">Basic Information</h2>
		<p class="text-sm text-gray-600 mb-6">
			Let's start with your legal name and date of birth. This information is used to find and
			remove your data from data broker sites.
		</p>
	</div>

	<FormField
		label="First Name"
		id="first-name"
		type="text"
		value={data.first_name || ''}
		error={errors.first_name}
		required={true}
		placeholder="John"
		onchange={handleFirstNameChange}
	/>

	<FormField
		label="Middle Name"
		id="middle-name"
		type="text"
		value={data.middle_name || ''}
		error={errors.middle_name}
		required={false}
		placeholder="Optional"
		onchange={handleMiddleNameChange}
	/>

	<FormField
		label="Last Name"
		id="last-name"
		type="text"
		value={data.last_name || ''}
		error={errors.last_name}
		required={true}
		placeholder="Doe"
		onchange={handleLastNameChange}
	/>

	<FormField
		label="Date of Birth"
		id="date-of-birth"
		type="date"
		value={data.date_of_birth || ''}
		error={errors.date_of_birth}
		required={false}
		onchange={handleDateOfBirthChange}
	/>

	<div class="mt-4 p-3 bg-blue-50 border border-blue-200 rounded-md">
		<p class="text-xs text-blue-800">
			<strong>Privacy Note:</strong> All information is encrypted and stored securely on your device.
			It is never sent to Spectral's servers.
		</p>
	</div>
</div>
