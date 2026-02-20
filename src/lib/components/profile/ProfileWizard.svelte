<script lang="ts">
	import { profileStore, vaultStore } from '$lib/stores';
	import type { ProfileInput, ProfileCompleteness } from '$lib/api/profile';
	import { getProfileCompleteness } from '$lib/api/profile';
	import BasicInfoStep from './BasicInfoStep.svelte';
	import ContactInfoStep from './ContactInfoStep.svelte';
	import AddressInfoStep from './AddressInfoStep.svelte';
	import AdditionalInfoStep from './AdditionalInfoStep.svelte';
	import ReviewStep from './ReviewStep.svelte';
	import CompletenessIndicator from './shared/CompletenessIndicator.svelte';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	// Check if profile already exists on mount
	onMount(async () => {
		if (vaultStore.currentVaultId) {
			await profileStore.loadProfiles(vaultStore.currentVaultId);
			if (profileStore.profiles.length > 0) {
				// Profile already exists, redirect to dashboard
				goto('/');
			}
		}
	});

	// Current step (0-4)
	let currentStep = $state(0);

	// Form data
	let formData = $state<Partial<ProfileInput>>({});

	// Completeness
	let completeness = $state<ProfileCompleteness | null>(null);

	// References to step components for validation
	let basicInfoRef: BasicInfoStep;
	let contactInfoRef: ContactInfoStep;
	let addressInfoRef: AddressInfoStep;
	let additionalInfoRef: AdditionalInfoStep;

	// Step configuration
	const steps = [
		{
			number: 1,
			title: 'Basic Info',
			subtitle: 'Name and date of birth',
			component: BasicInfoStep
		},
		{
			number: 2,
			title: 'Contact',
			subtitle: 'Email and phone numbers',
			component: ContactInfoStep
		},
		{
			number: 3,
			title: 'Addresses',
			subtitle: 'Current and previous addresses',
			component: AddressInfoStep
		},
		{
			number: 4,
			title: 'Additional Info',
			subtitle: 'Aliases and relatives',
			component: AdditionalInfoStep
		},
		{
			number: 5,
			title: 'Review',
			subtitle: 'Verify and submit',
			component: ReviewStep
		}
	];

	// Update completeness
	async function updateCompleteness() {
		try {
			completeness = await getProfileCompleteness();
		} catch (error) {
			console.error('Failed to get completeness:', error);
		}
	}

	// Handle data changes from steps
	function handleDataChange(data: Partial<ProfileInput>) {
		formData = { ...formData, ...data };
	}

	// Handle step update (for Phase 2 steps)
	function handleStepUpdate(updates: Partial<ProfileInput>) {
		formData = { ...formData, ...updates };
		updateCompleteness();
	}

	// Validate current step
	function validateCurrentStep(): boolean {
		if (currentStep === 0 && basicInfoRef) {
			return basicInfoRef.validate();
		}
		if (currentStep === 1 && contactInfoRef) {
			return contactInfoRef.validate();
		}
		if (currentStep === 2 && addressInfoRef) {
			return addressInfoRef.validate();
		}
		if (currentStep === 3 && additionalInfoRef) {
			return additionalInfoRef.validate();
		}
		return true; // Review step has no validation
	}

	// Navigate to next step
	function handleNext() {
		if (!validateCurrentStep()) {
			return;
		}
		if (currentStep < steps.length - 1) {
			currentStep++;
		}
	}

	// Navigate to previous step
	function handleBack() {
		if (currentStep > 0) {
			currentStep--;
		}
	}

	// Navigate to specific step (from review page edit buttons)
	function handleEdit(stepIndex: number) {
		currentStep = stepIndex;
	}

	// Save profile
	async function handleSave() {
		// Final validation - check all required fields
		const missingFields: string[] = [];

		if (!formData.first_name?.trim()) {
			missingFields.push('First Name');
		}
		if (!formData.last_name?.trim()) {
			missingFields.push('Last Name');
		}
		// Check new email_addresses array (or fallback to old email field)
		const hasEmail =
			(formData.email_addresses && formData.email_addresses.length > 0) || formData.email?.trim();
		if (!hasEmail) {
			missingFields.push('Email Address');
		}
		if (!formData.address_line1?.trim()) {
			missingFields.push('Street Address');
		}
		if (!formData.city?.trim()) {
			missingFields.push('City');
		}
		if (!formData.state?.trim()) {
			missingFields.push('State');
		}
		if (!formData.zip_code?.trim()) {
			missingFields.push('ZIP Code');
		}

		if (missingFields.length > 0) {
			alert(
				`Please fill in the following required fields:\n\n• ${missingFields.join('\n• ')}\n\nGo back to the relevant step to complete these fields.`
			);
			return;
		}

		// Populate old email field from email_addresses array for backward compatibility
		if (formData.email_addresses && formData.email_addresses.length > 0 && !formData.email) {
			formData.email = formData.email_addresses[0].email;
		}

		const profile = await profileStore.createProfile(
			vaultStore.currentVaultId!,
			formData as ProfileInput
		);

		if (profile) {
			// Success - redirect to dashboard
			goto('/');
		} else {
			// Error message is in profileStore.error
			alert(`Failed to create profile: ${profileStore.error}`);
		}
	}
</script>

<div
	class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 flex items-center justify-center p-4"
>
	<div class="bg-white rounded-lg shadow-xl p-8 w-full max-w-2xl">
		<!-- Completeness Indicator -->
		{#if completeness}
			<div class="mb-6">
				<CompletenessIndicator {completeness} />
			</div>
		{/if}

		<!-- Progress Indicator -->
		<div class="mb-8">
			<div class="flex justify-between items-center">
				{#each steps as step, index}
					<div class="flex flex-col items-center flex-1">
						<div
							class="w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium mb-2 transition-colors"
							class:bg-primary-600={index <= currentStep}
							class:text-white={index <= currentStep}
							class:bg-gray-200={index > currentStep}
							class:text-gray-600={index > currentStep}
							style={index <= currentStep ? 'background-color: #0284c7; color: white;' : ''}
						>
							{index + 1}
						</div>
						<div
							class="text-xs font-medium text-center"
							class:text-primary-600={index <= currentStep}
							class:text-gray-500={index > currentStep}
							style={index <= currentStep ? 'color: #0284c7;' : ''}
						>
							{step.title}
						</div>
					</div>
					{#if index < steps.length - 1}
						<div
							class="flex-1 h-1 mx-2 mb-6 transition-colors"
							class:bg-primary-600={index < currentStep}
							class:bg-gray-200={index >= currentStep}
							style={index < currentStep ? 'background-color: #0284c7;' : ''}
						></div>
					{/if}
				{/each}
			</div>
		</div>

		<!-- Step Content -->
		<div class="mb-8">
			{#if currentStep === 0}
				<BasicInfoStep bind:this={basicInfoRef} bind:data={formData} onchange={handleDataChange} />
			{:else if currentStep === 1}
				<ContactInfoStep
					bind:this={contactInfoRef}
					profile={formData}
					onUpdate={handleStepUpdate}
				/>
			{:else if currentStep === 2}
				<AddressInfoStep
					bind:this={addressInfoRef}
					profile={formData}
					onUpdate={handleStepUpdate}
				/>
			{:else if currentStep === 3}
				<AdditionalInfoStep
					bind:this={additionalInfoRef}
					profile={formData}
					onUpdate={handleStepUpdate}
				/>
			{:else if currentStep === 4}
				<ReviewStep data={formData} onEdit={handleEdit} />
			{/if}
		</div>

		<!-- Navigation Buttons -->
		<div class="flex justify-between gap-4">
			{#if currentStep > 0}
				<button
					onclick={handleBack}
					class="px-6 py-3 border border-gray-300 text-gray-700 rounded-lg font-medium hover:bg-gray-50 transition-colors"
				>
					Back
				</button>
			{:else}
				<div></div>
			{/if}

			{#if currentStep < steps.length - 1}
				<button
					onclick={handleNext}
					class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors ml-auto"
					style="background-color: #0284c7; color: white;"
				>
					Next
				</button>
			{:else}
				<button
					onclick={handleSave}
					disabled={profileStore.loading}
					class="px-6 py-3 bg-green-600 text-white rounded-lg font-medium hover:bg-green-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors ml-auto"
				>
					{profileStore.loading ? 'Saving...' : 'Save Profile'}
				</button>
			{/if}
		</div>

		<!-- Error Display -->
		{#if profileStore.error}
			<div class="mt-4 p-3 bg-red-50 border border-red-200 rounded-md">
				<p class="text-sm text-red-800">{profileStore.error}</p>
			</div>
		{/if}
	</div>
</div>
