<script lang="ts">
	/* eslint-disable no-unused-vars */
	import { FormField } from './shared';
	import type { ProfileInput } from '$lib/api';

	interface Props {
		data: Partial<ProfileInput>;
		onchange: (data: Partial<ProfileInput>) => void;
		ssnLast4?: string;
	}

	let { data = $bindable({}), onchange, ssnLast4 }: Props = $props();
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

	// DOB dropdowns
	const currentYear = new Date().getFullYear();
	const years = Array.from({ length: 108 }, (_, i) => currentYear - 13 - i);
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

	let dobMonth = $state('');
	let dobDay = $state('');
	let dobYear = $state('');

	// Initialise dropdowns from existing data
	$effect(() => {
		if (data.date_of_birth && !dobYear) {
			const parts = data.date_of_birth.split('-');
			if (parts.length === 3) {
				dobYear = parts[0];
				dobMonth = parts[1];
				dobDay = parts[2];
			}
		}
	});

	const daysInMonth = $derived(() => {
		if (!dobMonth || !dobYear) return 31;
		return new Date(Number(dobYear), Number(dobMonth), 0).getDate();
	});

	const days = $derived(() => Array.from({ length: daysInMonth() }, (_, i) => i + 1));

	function handleDobChange() {
		if (dobYear && dobMonth && dobDay) {
			const day = dobDay.padStart(2, '0');
			const value = `${dobYear}-${dobMonth}-${day}`;
			data.date_of_birth = value;
			errors.date_of_birth = validateDateOfBirth(value);
			onchange(data);
		} else {
			data.date_of_birth = undefined;
			errors.date_of_birth = '';
			onchange(data);
		}
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

	function validateSsn(value: string): string {
		if (!value) return ''; // Optional
		const digits = value.replace(/\D/g, '');
		if (digits.length !== 9) return 'SSN must be 9 digits (e.g. 123-45-6789)';
		return '';
	}

	function handleSsnChange(value: string) {
		data.ssn = value || undefined;
		errors.ssn = validateSsn(value);
		onchange(data);
	}

	// Expose validation function for parent
	export function validate(): boolean {
		const newErrors: Record<string, string> = {};

		newErrors.first_name = validateFirstName(data.first_name || '');
		newErrors.last_name = validateLastName(data.last_name || '');
		newErrors.middle_name = validateMiddleName(data.middle_name || '');
		newErrors.date_of_birth = validateDateOfBirth(data.date_of_birth || '');
		newErrors.ssn = validateSsn(data.ssn || '');

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

	<div class="mb-4">
		<label class="block text-sm font-medium text-gray-700 mb-2">Date of Birth</label>
		<div class="flex gap-2">
			<select
				bind:value={dobMonth}
				onchange={handleDobChange}
				class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
			>
				<option value="">Month</option>
				{#each months as m}
					<option value={m.value}>{m.label}</option>
				{/each}
			</select>
			<select
				bind:value={dobDay}
				onchange={handleDobChange}
				class="w-24 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
			>
				<option value="">Day</option>
				{#each days() as d}
					<option value={String(d)}>{d}</option>
				{/each}
			</select>
			<select
				bind:value={dobYear}
				onchange={handleDobChange}
				class="w-28 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
			>
				<option value="">Year</option>
				{#each years as y}
					<option value={String(y)}>{y}</option>
				{/each}
			</select>
		</div>
		{#if errors.date_of_birth}
			<p class="mt-1 text-sm text-red-600">{errors.date_of_birth}</p>
		{/if}
	</div>

	<FormField
		label="Social Security Number"
		id="ssn"
		type="password"
		value={data.ssn || ''}
		error={errors.ssn}
		required={false}
		placeholder={ssnLast4 && !data.ssn
			? `Leave blank to keep ***-**-${ssnLast4}`
			: '123-45-6789 (optional)'}
		onchange={handleSsnChange}
	/>
	{#if ssnLast4 && !data.ssn}
		<p class="text-xs text-gray-500 -mt-3">
			SSN on file ending in {ssnLast4}. Leave blank to keep it, or enter a new one to replace it.
		</p>
	{/if}

	<div class="mt-4 p-3 bg-blue-50 border border-blue-200 rounded-md">
		<p class="text-xs text-blue-800">
			<strong>Privacy Note:</strong> All information is encrypted and stored locally on your device. Spectral
			has no cloud or servers. Your data will only be transmitted directly to data brokers when you submit
			removal requests — this is necessary to request your information be removed.
		</p>
	</div>
</div>
