<script lang="ts">
	import { profileStore } from '$lib/stores';
	import type { ProfileInput } from '$lib/api';
	import BasicInfoStep from './BasicInfoStep.svelte';
	import ContactInfoStep from './ContactInfoStep.svelte';
	import AddressInfoStep from './AddressInfoStep.svelte';
	import ReviewStep from './ReviewStep.svelte';
	import { goto } from '$app/navigation';

	// Current step (0-3)
	let currentStep = $state(0);

	// Form data
	let formData = $state<Partial<ProfileInput>>({});

	// References to step components for validation
	let basicInfoRef: BasicInfoStep;
	let contactInfoRef: ContactInfoStep;
	let addressInfoRef: AddressInfoStep;

	// Step configuration
	const steps = [
		{ title: 'Basic Info', component: BasicInfoStep },
		{ title: 'Contact Info', component: ContactInfoStep },
		{ title: 'Address Info', component: AddressInfoStep },
		{ title: 'Review', component: ReviewStep }
	];

	// Handle data changes from steps
	function handleDataChange(data: Partial<ProfileInput>) {
		formData = { ...formData, ...data };
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
		// Final validation
		if (
			!formData.first_name ||
			!formData.last_name ||
			!formData.email ||
			!formData.address_line1 ||
			!formData.city ||
			!formData.state ||
			!formData.zip_code
		) {
			alert('Please fill in all required fields');
			return;
		}

		const profile = await profileStore.createProfile(formData as ProfileInput);

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
					bind:data={formData}
					onchange={handleDataChange}
				/>
			{:else if currentStep === 2}
				<AddressInfoStep
					bind:this={addressInfoRef}
					bind:data={formData}
					onchange={handleDataChange}
				/>
			{:else if currentStep === 3}
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
