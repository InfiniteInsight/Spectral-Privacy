<script lang="ts">
	/* eslint-disable no-unused-vars */
	import { FormField, StateSelect, DisabledFieldNotice } from './shared';
	import type { ProfileInput } from '$lib/api';

	interface Props {
		data: Partial<ProfileInput>;
		onchange: (data: Partial<ProfileInput>) => void;
	}

	let { data = $bindable({}), onchange }: Props = $props();
	/* eslint-enable no-unused-vars */

	// Validation state
	let errors = $state<Record<string, string>>({});

	// Validate address fields
	function validateAddressLine(value: string, fieldName: string): string {
		if (!value.trim()) return `${fieldName} is required`;
		if (value.length > 200) return `${fieldName} is too long (max 200 characters)`;
		return '';
	}

	function validateCity(value: string): string {
		if (!value.trim()) return 'City is required';
		if (value.length > 100) return 'City name is too long';
		// Cities can contain letters, spaces, hyphens, apostrophes, and periods (e.g., "St. Louis")
		if (!/^[a-zA-Z\s.'-]+$/.test(value)) {
			return 'City can only contain letters, spaces, periods, hyphens, and apostrophes';
		}
		return '';
	}

	function validateState(value: string): string {
		if (!value || value === '') return 'State is required';
		return '';
	}

	function validateZipCode(value: string): string {
		if (!value.trim()) return 'ZIP code is required';
		// Accept 5-digit or 5+4 format
		if (!/^\d{5}(-\d{4})?$/.test(value)) {
			return 'Invalid ZIP code format (use 12345 or 12345-6789)';
		}
		return '';
	}

	// Handle field changes
	function handleAddressLine1Change(value: string) {
		data.address_line1 = value;
		errors.address_line1 = validateAddressLine(value, 'Address line 1');
		onchange(data);
	}

	function handleAddressLine2Change(value: string) {
		data.address_line2 = value || undefined;
		if (value) {
			errors.address_line2 = validateAddressLine(value, 'Address line 2');
		} else {
			errors.address_line2 = '';
		}
		onchange(data);
	}

	function handleCityChange(value: string) {
		data.city = value;
		errors.city = validateCity(value);
		onchange(data);
	}

	function handleStateChange(value: string) {
		data.state = value;
		errors.state = validateState(value);
		onchange(data);
	}

	function handleZipCodeChange(value: string) {
		data.zip_code = value;
		errors.zip_code = validateZipCode(value);
		onchange(data);
	}

	// Expose validation function for parent
	export function validate(): boolean {
		const newErrors: Record<string, string> = {};

		newErrors.address_line1 = validateAddressLine(data.address_line1 || '', 'Address line 1');
		if (data.address_line2) {
			newErrors.address_line2 = validateAddressLine(data.address_line2, 'Address line 2');
		}
		newErrors.city = validateCity(data.city || '');
		newErrors.state = validateState(data.state || '');
		newErrors.zip_code = validateZipCode(data.zip_code || '');

		errors = newErrors;

		return !Object.values(newErrors).some((err) => err !== '');
	}
</script>

<div class="space-y-4">
	<div>
		<h2 class="text-xl font-semibold text-gray-900 mb-2">Address Information</h2>
		<p class="text-sm text-gray-600 mb-6">
			Provide your current residential address. This helps us identify and remove your information
			from data broker databases.
		</p>
	</div>

	<FormField
		label="Address Line 1"
		id="address-line1"
		type="text"
		value={data.address_line1 || ''}
		error={errors.address_line1}
		required={true}
		placeholder="123 Main Street"
		onchange={handleAddressLine1Change}
	/>

	<FormField
		label="Address Line 2"
		id="address-line2"
		type="text"
		value={data.address_line2 || ''}
		error={errors.address_line2}
		required={false}
		placeholder="Apt 4B (optional)"
		onchange={handleAddressLine2Change}
	/>

	<FormField
		label="City"
		id="city"
		type="text"
		value={data.city || ''}
		error={errors.city}
		required={true}
		placeholder="San Francisco"
		onchange={handleCityChange}
	/>

	<StateSelect
		label="State"
		id="state"
		value={data.state || ''}
		error={errors.state}
		required={true}
		onchange={handleStateChange}
	/>

	<FormField
		label="ZIP Code"
		id="zip-code"
		type="text"
		value={data.zip_code || ''}
		error={errors.zip_code}
		required={true}
		placeholder="94102"
		onchange={handleZipCodeChange}
	/>

	<DisabledFieldNotice fieldName="Previous Addresses" />

	<div class="mt-4 p-3 bg-blue-50 border border-blue-200 rounded-md">
		<p class="text-xs text-blue-800">
			<strong>US Addresses Only:</strong> Currently, Spectral supports US addresses only. International
			support is coming in a future update.
		</p>
	</div>
</div>
