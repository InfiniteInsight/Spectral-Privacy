<script lang="ts">
	/* eslint-disable no-unused-vars */
	import type { ProfileInput } from '$lib/api';

	interface Props {
		data: Partial<ProfileInput>;
		onEdit: (stepIndex: number) => void;
	}

	let { data, onEdit }: Props = $props();
	/* eslint-enable no-unused-vars */

	// Format date for display (parse as local date to avoid timezone issues)
	function formatDate(dateStr: string | undefined): string {
		if (!dateStr) return 'Not provided';
		// Parse as YYYY-MM-DD in local timezone
		const [year, month, day] = dateStr.split('-').map(Number);
		const date = new Date(year, month - 1, day);
		return date.toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });
	}

	// Format alias name
	function formatAliasName(alias: any): string {
		const parts = [alias.first_name, alias.middle_name, alias.last_name].filter(Boolean).join(' ');
		if (alias.nickname) {
			return parts ? `${parts} "${alias.nickname}"` : `"${alias.nickname}"`;
		}
		return parts || 'Unnamed';
	}

	// Format relative name
	function formatRelativeName(relative: any): string {
		const parts = [relative.first_name, relative.middle_name, relative.last_name]
			.filter(Boolean)
			.join(' ');
		if (relative.maiden_name) {
			return parts
				? `${parts} (maiden: ${relative.maiden_name})`
				: `(maiden: ${relative.maiden_name})`;
		}
		return parts || 'Unnamed';
	}

	// Get full name
	const fullName = $derived(() => {
		const parts = [data.first_name, data.middle_name, data.last_name].filter(Boolean);
		return parts.join(' ') || 'Not provided';
	});

	// Get full address
	const fullAddress = $derived(() => {
		const parts = [
			data.address_line1,
			data.address_line2,
			data.city,
			data.state,
			data.zip_code
		].filter(Boolean);
		if (parts.length === 0) return 'Not provided';

		// Format: "123 Main St, Apt 4B, San Francisco, CA 94102"
		return parts.join(', ');
	});
</script>

<div class="space-y-6">
	<div>
		<h2 class="text-xl font-semibold text-gray-900 mb-2">Review Your Information</h2>
		<p class="text-sm text-gray-600 mb-6">
			Please review your information below. You can go back to edit any section.
		</p>
	</div>

	<!-- Basic Information Section -->
	<div class="border border-gray-200 rounded-lg p-4">
		<div class="flex items-center justify-between mb-3">
			<h3 class="text-base font-medium text-gray-900">Basic Information</h3>
			<button
				onclick={() => onEdit(0)}
				class="text-sm text-primary-600 hover:text-primary-700 font-medium"
				style="color: #0284c7;"
			>
				Edit
			</button>
		</div>
		<dl class="space-y-2 text-sm">
			<div>
				<dt class="text-gray-500">Full Name</dt>
				<dd class="text-gray-900">{fullName()}</dd>
			</div>
			<div>
				<dt class="text-gray-500">Date of Birth</dt>
				<dd class="text-gray-900">{formatDate(data.date_of_birth)}</dd>
			</div>
		</dl>
	</div>

	<!-- Contact Information Section -->
	<div class="border border-gray-200 rounded-lg p-4">
		<div class="flex items-center justify-between mb-3">
			<h3 class="text-base font-medium text-gray-900">Contact Information</h3>
			<button
				onclick={() => onEdit(1)}
				class="text-sm text-primary-600 hover:text-primary-700 font-medium"
				style="color: #0284c7;"
			>
				Edit
			</button>
		</div>
		<dl class="space-y-2 text-sm">
			<!-- Email Addresses -->
			{#if data.email_addresses && data.email_addresses.length > 0}
				<div>
					<dt class="text-gray-500">Email Addresses</dt>
					<dd class="text-gray-900">
						{#each data.email_addresses as email}
							<div>{email.email} ({email.email_type})</div>
						{/each}
					</dd>
				</div>
			{:else}
				<div>
					<dt class="text-gray-500">Email</dt>
					<dd class="text-gray-900">{data.email || 'Not provided'}</dd>
				</div>
			{/if}
			<!-- Phone Numbers -->
			{#if data.phone_numbers && data.phone_numbers.length > 0}
				<div>
					<dt class="text-gray-500">Phone Numbers</dt>
					<dd class="text-gray-900">
						{#each data.phone_numbers as phone}
							<div>{phone.number} ({phone.phone_type})</div>
						{/each}
					</dd>
				</div>
			{/if}
		</dl>
	</div>

	<!-- Address Information Section -->
	<div class="border border-gray-200 rounded-lg p-4">
		<div class="flex items-center justify-between mb-3">
			<h3 class="text-base font-medium text-gray-900">Address Information</h3>
			<button
				onclick={() => onEdit(2)}
				class="text-sm text-primary-600 hover:text-primary-700 font-medium"
				style="color: #0284c7;"
			>
				Edit
			</button>
		</div>
		<dl class="space-y-2 text-sm">
			<div>
				<dt class="text-gray-500">Current Address</dt>
				<dd class="text-gray-900">{fullAddress()}</dd>
			</div>
			<!-- Previous Addresses -->
			{#if data.previous_addresses && data.previous_addresses.length > 0}
				<div>
					<dt class="text-gray-500">Previous Addresses</dt>
					<dd class="text-gray-900">
						{#each data.previous_addresses as addr}
							<div class="mb-2">
								<div>{addr.address_line1}</div>
								{#if addr.address_line2}
									<div>{addr.address_line2}</div>
								{/if}
								<div>{addr.city}, {addr.state} {addr.zip_code}</div>
								{#if addr.lived_from || addr.lived_to}
									<div class="text-xs text-gray-500">
										{addr.lived_from ?? '?'} – {addr.lived_to ?? '?'}
									</div>
								{/if}
							</div>
						{/each}
					</dd>
				</div>
			{/if}
		</dl>
	</div>

	<!-- Additional Information Section -->
	<div class="border border-gray-200 rounded-lg p-4">
		<div class="flex items-center justify-between mb-3">
			<h3 class="text-base font-medium text-gray-900">Additional Information</h3>
			<button
				onclick={() => onEdit(3)}
				class="text-sm text-primary-600 hover:text-primary-700 font-medium"
				style="color: #0284c7;"
			>
				Edit
			</button>
		</div>
		<dl class="space-y-2 text-sm">
			<!-- Aliases -->
			{#if data.aliases && data.aliases.length > 0}
				<div>
					<dt class="text-gray-500">Aliases</dt>
					<dd class="text-gray-900">
						{#each data.aliases as alias}
							<div>{formatAliasName(alias)}</div>
						{/each}
					</dd>
				</div>
			{/if}
			<!-- Relatives -->
			{#if data.relatives && data.relatives.length > 0}
				<div>
					<dt class="text-gray-500">Family Members</dt>
					<dd class="text-gray-900">
						{#each data.relatives as relative}
							<div>{formatRelativeName(relative)} ({relative.relationship})</div>
						{/each}
					</dd>
				</div>
			{/if}
			{#if (!data.aliases || data.aliases.length === 0) && (!data.relatives || data.relatives.length === 0)}
				<div>
					<dt class="text-gray-500">No additional information</dt>
				</div>
			{/if}
		</dl>
	</div>

	<div class="mt-6 p-4 bg-green-50 border border-green-200 rounded-md">
		<div class="flex items-start gap-2">
			<svg
				class="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5"
				fill="currentColor"
				viewBox="0 0 20 20"
			>
				<path
					fill-rule="evenodd"
					d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
					clip-rule="evenodd"
				/>
			</svg>
			<div class="flex-1">
				<p class="text-sm font-medium text-green-800">Ready to save</p>
				<p class="text-xs text-green-700 mt-1">
					Your information will be encrypted and stored securely on your device.
				</p>
			</div>
		</div>
	</div>
</div>
