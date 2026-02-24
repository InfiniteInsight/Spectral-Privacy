<script lang="ts">
	import { scanStore, vaultStore } from '$lib/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import type { Finding, ZeroResultBroker } from '$lib/api/scan';
	import { scanAPI } from '$lib/api/scan';
	import { removalAPI } from '$lib/api/removal';

	const scanJobId = $derived($page.params.id);
	let expandedFindings = $state<Set<string>>(new Set());
	let actionError = $state<string | null>(null);
	let isSubmitting = $state(false);
	let possibleMatches = $state<ZeroResultBroker[]>([]);
	let dismissedMatches = $state<Set<string>>(new Set());

	onMount(async () => {
		if (!scanJobId) {
			goto('/scan/start');
			return;
		}

		if (!vaultStore.currentVaultId) {
			goto('/');
			return;
		}

		// Load pending findings
		await scanStore.loadFindings(vaultStore.currentVaultId, scanJobId, 'PendingVerification');

		// Load possible matches
		await loadPossibleMatches();
	});

	async function loadPossibleMatches() {
		if (!vaultStore.currentVaultId || !scanJobId) return;

		try {
			const matches = await scanAPI.getPossibleMatches(vaultStore.currentVaultId, scanJobId);
			possibleMatches = matches;
		} catch (err) {
			console.error('Failed to load possible matches:', err);
		}
	}

	async function handleAcceptMatch(zeroResultBrokerId: string, matchedFindingId: string) {
		if (!vaultStore.currentVaultId || !scanJobId) return;

		try {
			await scanAPI.acceptMatch(
				vaultStore.currentVaultId,
				scanJobId,
				zeroResultBrokerId,
				matchedFindingId
			);

			// Reload findings to get the new finding
			await scanStore.loadFindings(vaultStore.currentVaultId, scanJobId, 'PendingVerification');

			// Remove the match from possible matches
			possibleMatches = possibleMatches
				.map((broker) => {
					if (broker.broker_id === zeroResultBrokerId) {
						return {
							...broker,
							possible_matches: broker.possible_matches.filter(
								(m) => m.finding.id !== matchedFindingId
							)
						};
					}
					return broker;
				})
				.filter((broker) => broker.possible_matches.length > 0);
		} catch (err) {
			console.error('Failed to accept match:', err);
		}
	}

	async function handleDismissMatch(zeroResultBrokerId: string, matchedFindingId: string) {
		if (!vaultStore.currentVaultId) return;

		try {
			await scanAPI.dismissMatch(vaultStore.currentVaultId, zeroResultBrokerId, matchedFindingId);

			const dismissKey = `${zeroResultBrokerId}:${matchedFindingId}`;
			dismissedMatches = new Set([...dismissedMatches, dismissKey]);
		} catch (err) {
			console.error('Failed to dismiss match:', err);
		}
	}

	const visibleMatches = $derived.by(() => {
		return possibleMatches
			.map((broker) => ({
				...broker,
				possible_matches: broker.possible_matches.filter((match) => {
					const dismissKey = `${broker.broker_id}:${match.finding.id}`;
					return !dismissedMatches.has(dismissKey);
				})
			}))
			.filter((broker) => broker.possible_matches.length > 0);
	});

	// Group findings by broker
	const groupedFindings = $derived.by(() => {
		const groups = new Map<string, Finding[]>();

		for (const finding of scanStore.findings) {
			if (!groups.has(finding.broker_id)) {
				groups.set(finding.broker_id, []);
			}
			groups.get(finding.broker_id)!.push(finding);
		}

		return groups;
	});

	const totalFindings = $derived(scanStore.findings.length);
	const confirmedCount = $derived(
		scanStore.findings.filter((f) => f.verification_status === 'Confirmed').length
	);

	async function handleVerify(findingId: string, isMatch: boolean) {
		if (!vaultStore.currentVaultId) return;

		actionError = null;
		try {
			await scanStore.verifyFinding(vaultStore.currentVaultId, findingId, isMatch);
		} catch (err) {
			actionError = 'Failed to update finding. Please try again.';
			console.error('Verification failed:', err);
		}
	}

	async function handleSubmit() {
		if (confirmedCount === 0 || !scanJobId || !vaultStore.currentVaultId || isSubmitting) {
			return;
		}

		isSubmitting = true;
		actionError = null;
		try {
			// Step 1: Create removal attempts (existing command)
			const removalAttemptIds = await scanStore.submitRemovals(
				vaultStore.currentVaultId,
				scanJobId
			);

			// Step 2: Start batch processing
			const result = await removalAPI.processBatch(vaultStore.currentVaultId, removalAttemptIds);

			// Validate that items were actually queued
			if (result.queued_count === 0) {
				actionError = 'No items were queued for processing. Please try again.';
				console.error('Batch processing failed - nothing queued:', result);
				return;
			}

			// Step 3: Navigate to progress page
			goto(`/removals/progress/${scanJobId}`);
		} catch (err) {
			actionError = 'Failed to submit removals. Please try again.';
			console.error('Submission failed:', err);
		} finally {
			isSubmitting = false;
		}
	}

	function toggleExpanded(findingId: string) {
		const newSet = new Set(expandedFindings);
		if (newSet.has(findingId)) {
			newSet.delete(findingId);
		} else {
			newSet.add(findingId);
		}
		expandedFindings = newSet;
	}

	function formatPhoneNumbers(phones: string[]): string {
		if (phones.length === 0) return 'None';
		if (phones.length <= 2) return phones.join(', ');
		return `${phones[0]}, ${phones[1]} (+${phones.length - 2} more)`;
	}
</script>

<div class="min-h-screen bg-gradient-to-br from-primary-50 to-primary-100 p-4">
	<div class="max-w-6xl mx-auto">
		<div class="bg-white rounded-lg shadow-xl p-8">
			<!-- Header -->
			<div class="mb-8">
				<h1 class="text-3xl font-bold text-gray-900 mb-2">Review Findings</h1>
				<p class="text-gray-600">
					{totalFindings} result{totalFindings !== 1 ? 's' : ''} found •
					{confirmedCount} confirmed for removal
				</p>
			</div>

			{#if scanStore.loading}
				<div class="flex items-center justify-center py-12">
					<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
				</div>
			{:else if totalFindings === 0}
				<!-- No Findings -->
				<div class="text-center py-12">
					<p class="text-gray-600 mb-6">
						No results found. Your information may not be on these data brokers.
					</p>
					<button
						onclick={() => goto('/')}
						class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 transition-colors"
						style="background-color: #0284c7; color: white;"
					>
						Return to Dashboard
					</button>
				</div>
			{:else}
				<!-- Instructions -->
				<div class="mb-6 p-4 bg-blue-50 border border-blue-200 rounded-lg">
					<p class="text-sm text-gray-700">
						Review each finding and confirm which ones are accurate matches. Only confirmed findings
						will be submitted for removal.
					</p>
				</div>

				<!-- Action Error Display -->
				{#if actionError}
					<div class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg">
						<p class="text-sm text-red-700">{actionError}</p>
					</div>
				{/if}

				<!-- Grouped Findings -->
				<div class="space-y-6 mb-8">
					{#each [...groupedFindings.entries()] as [brokerId, findings]}
						<div class="border border-gray-200 rounded-lg">
							<!-- Broker Header -->
							<div class="bg-gray-50 px-6 py-4 border-b border-gray-200">
								<h3 class="text-lg font-semibold text-gray-900">
									{brokerId || 'Unknown Broker'}
									<span class="text-sm font-normal text-gray-600 ml-2">
										({findings.length} result{findings.length !== 1 ? 's' : ''})
									</span>
								</h3>
							</div>

							<!-- Findings List -->
							<div class="divide-y divide-gray-200">
								{#each findings as finding}
									{@const isExpanded = expandedFindings.has(finding.id)}
									{@const isConfirmed = finding.verification_status === 'Confirmed'}
									{@const isRejected = finding.verification_status === 'Rejected'}

									<div class="px-6 py-4">
										<!-- Compact View -->
										<div class="flex items-start justify-between gap-4">
											<div class="flex-1">
												<p class="font-medium text-gray-900">
													{finding.extracted_data.name || 'Unknown Name'}
													{#if finding.extracted_data.age}
														<span class="text-gray-600 text-sm ml-2">
															Age {finding.extracted_data.age}
														</span>
													{/if}
												</p>
												{#if finding.extracted_data.addresses.length > 0}
													<p class="text-sm text-gray-600 mt-1">
														{finding.extracted_data.addresses[0]}
													</p>
												{/if}
												{#if finding.extracted_data.phone_numbers.length > 0}
													<p class="text-sm text-gray-600">
														{formatPhoneNumbers(finding.extracted_data.phone_numbers)}
													</p>
												{/if}
											</div>

											<!-- Actions -->
											<div class="flex items-center gap-2">
												{#if !isConfirmed && !isRejected}
													<button
														onclick={() => handleVerify(finding.id, true)}
														class="px-4 py-2 bg-green-100 text-green-700 rounded-md hover:bg-green-200 transition-colors text-sm font-medium"
													>
														Confirm
													</button>
													<button
														onclick={() => handleVerify(finding.id, false)}
														class="px-4 py-2 bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 transition-colors text-sm font-medium"
													>
														Reject
													</button>
												{:else if isConfirmed}
													<span
														class="px-4 py-2 bg-green-100 text-green-700 rounded-md text-sm font-medium"
													>
														✓ Confirmed
													</span>
												{:else}
													<span
														class="px-4 py-2 bg-gray-100 text-gray-500 rounded-md text-sm font-medium"
													>
														✗ Rejected
													</span>
												{/if}

												<button
													onclick={() => toggleExpanded(finding.id)}
													class="px-3 py-2 text-primary-600 hover:bg-primary-50 rounded-md transition-colors text-sm font-medium"
												>
													{isExpanded ? 'Hide' : 'Details'}
												</button>
											</div>
										</div>

										<!-- Expanded View -->
										{#if isExpanded}
											<div class="mt-4 p-4 bg-gray-50 rounded-md">
												<dl class="grid grid-cols-2 gap-4 text-sm">
													{#if finding.extracted_data.name}
														<div>
															<dt class="font-medium text-gray-700">Name</dt>
															<dd class="text-gray-900">{finding.extracted_data.name}</dd>
														</div>
													{/if}
													{#if finding.extracted_data.age}
														<div>
															<dt class="font-medium text-gray-700">Age</dt>
															<dd class="text-gray-900">{finding.extracted_data.age}</dd>
														</div>
													{/if}
													{#if finding.extracted_data.addresses.length > 0}
														<div class="col-span-2">
															<dt class="font-medium text-gray-700">Addresses</dt>
															<dd class="text-gray-900">
																{#each finding.extracted_data.addresses as address}
																	<div>{address}</div>
																{/each}
															</dd>
														</div>
													{/if}
													{#if finding.extracted_data.phone_numbers.length > 0}
														<div class="col-span-2">
															<dt class="font-medium text-gray-700">Phone Numbers</dt>
															<dd class="text-gray-900">
																{finding.extracted_data.phone_numbers.join(', ')}
															</dd>
														</div>
													{/if}
													{#if finding.extracted_data.emails.length > 0}
														<div class="col-span-2">
															<dt class="font-medium text-gray-700">Emails</dt>
															<dd class="text-gray-900">
																{finding.extracted_data.emails.join(', ')}
															</dd>
														</div>
													{/if}
													{#if finding.extracted_data.relatives.length > 0}
														<div class="col-span-2">
															<dt class="font-medium text-gray-700">Relatives</dt>
															<dd class="text-gray-900">
																{finding.extracted_data.relatives.join(', ')}
															</dd>
														</div>
													{/if}
													<div class="col-span-2">
														<dt class="font-medium text-gray-700">Listing URL</dt>
														<dd class="text-gray-900">
															<a
																href={finding.listing_url}
																target="_blank"
																rel="noopener noreferrer"
																class="text-primary-600 hover:text-primary-700 underline"
															>
																View on {brokerId} ↗
															</a>
														</dd>
													</div>
												</dl>
											</div>
										{/if}
									</div>
								{/each}
							</div>
						</div>
					{/each}
				</div>

				<!-- Possible Matches Section -->
				{#if !scanStore.loading && visibleMatches.length > 0}
					<div class="mt-8 space-y-6">
						<div class="border-t border-gray-300 pt-6">
							<h2 class="text-2xl font-bold text-gray-900 mb-2">Possible Matches</h2>
							<p class="text-gray-600 text-sm mb-6">
								These brokers returned no results, but we found similar people on other brokers.
							</p>
						</div>

						{#each visibleMatches as zeroResultBroker}
							<div class="border border-amber-200 bg-amber-50 rounded-lg">
								<div class="bg-amber-100 px-6 py-4 border-b border-amber-200">
									<h3 class="text-lg font-semibold text-gray-900">
										{zeroResultBroker.broker_id}
										<span class="text-sm font-normal text-gray-600 ml-2">
											({zeroResultBroker.possible_matches.length} possible match{zeroResultBroker
												.possible_matches.length !== 1
												? 'es'
												: ''})
										</span>
									</h3>
								</div>

								<div class="divide-y divide-amber-200">
									{#each zeroResultBroker.possible_matches as match}
										{@const isExpanded = expandedFindings.has(match.finding.id)}

										<div class="px-6 py-4">
											<div class="flex items-start justify-between gap-4">
												<div class="flex-1">
													<div class="flex items-center gap-2 mb-1">
														<p class="font-medium text-gray-900">
															{match.finding.extracted_data.name || 'Unknown'}
															{#if match.finding.extracted_data.age}
																<span class="text-gray-600 text-sm ml-2">
																	Age {match.finding.extracted_data.age}
																</span>
															{/if}
														</p>
														<span class="px-2 py-1 bg-blue-100 text-blue-700 text-xs rounded-full">
															{Math.round(match.similarity_score * 100)}% match
														</span>
													</div>

													<p class="text-sm text-gray-600">
														Found on: <span class="font-medium">{match.source_broker_id}</span>
													</p>

													{#if match.finding.extracted_data.addresses.length > 0}
														<p class="text-sm text-gray-600 mt-1">
															{match.finding.extracted_data.addresses[0]}
															{#if match.location_matched}
																<span class="ml-2 text-green-600 font-medium">✓ Location match</span
																>
															{/if}
														</p>
													{/if}
												</div>

												<div class="flex items-center gap-2">
													<button
														onclick={() =>
															handleAcceptMatch(zeroResultBroker.broker_id, match.finding.id)}
														class="px-4 py-2 bg-green-100 text-green-700 rounded-md hover:bg-green-200 text-sm font-medium"
													>
														This is me
													</button>
													<button
														onclick={() =>
															handleDismissMatch(zeroResultBroker.broker_id, match.finding.id)}
														class="px-4 py-2 bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 text-sm font-medium"
													>
														Not me
													</button>
													<button
														onclick={() => toggleExpanded(match.finding.id)}
														class="px-3 py-2 text-primary-600 hover:bg-primary-50 rounded-md text-sm font-medium"
													>
														{isExpanded ? 'Hide' : 'Details'}
													</button>
												</div>
											</div>

											{#if isExpanded}
												<div class="mt-4 p-4 bg-white rounded-md border border-amber-200">
													<dl class="grid grid-cols-2 gap-4 text-sm">
														<div>
															<dt class="font-medium text-gray-700">Name Similarity</dt>
															<dd class="text-gray-900">
																{Math.round(match.name_similarity * 100)}%
															</dd>
														</div>
														<div>
															<dt class="font-medium text-gray-700">Location Match</dt>
															<dd class="text-gray-900">
																{match.location_matched ? 'Yes ✓' : 'No'}
															</dd>
														</div>
														{#if match.finding.extracted_data.phone_numbers.length > 0}
															<div class="col-span-2">
																<dt class="font-medium text-gray-700">Phone Numbers</dt>
																<dd class="text-gray-900">
																	{match.finding.extracted_data.phone_numbers.join(', ')}
																</dd>
															</div>
														{/if}
														{#if match.finding.extracted_data.emails.length > 0}
															<div class="col-span-2">
																<dt class="font-medium text-gray-700">Emails</dt>
																<dd class="text-gray-900">
																	{match.finding.extracted_data.emails.join(', ')}
																</dd>
															</div>
														{/if}
														{#if match.finding.extracted_data.relatives.length > 0}
															<div class="col-span-2">
																<dt class="font-medium text-gray-700">Relatives</dt>
																<dd class="text-gray-900">
																	{match.finding.extracted_data.relatives.join(', ')}
																</dd>
															</div>
														{/if}
														<div class="col-span-2">
															<dt class="font-medium text-gray-700">Listing URL</dt>
															<dd class="text-gray-900">
																<a
																	href={match.finding.listing_url}
																	target="_blank"
																	rel="noopener noreferrer"
																	class="text-primary-600 hover:text-primary-700 underline"
																>
																	View on {match.source_broker_id} ↗
																</a>
															</dd>
														</div>
													</dl>
												</div>
											{/if}
										</div>
									{/each}
								</div>
							</div>
						{/each}
					</div>
				{/if}

				<!-- Footer Actions -->
				<div class="flex items-center justify-between pt-6 border-t border-gray-200">
					<button
						onclick={() => goto('/')}
						class="px-6 py-3 border border-gray-300 text-gray-700 rounded-lg font-medium hover:bg-gray-50 transition-colors"
					>
						Cancel
					</button>

					<div class="flex items-center gap-4">
						<p class="text-sm text-gray-600">
							{confirmedCount} of {totalFindings} confirmed
						</p>
						<button
							onclick={handleSubmit}
							disabled={confirmedCount === 0 || isSubmitting}
							class="px-6 py-3 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
							style="background-color: #0284c7; color: white;"
						>
							{isSubmitting ? 'Submitting...' : 'Submit Removals'}
						</button>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>
