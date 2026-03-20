<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { profileStore } from '$lib/stores';
	import { profileAPI } from '$lib/api/profile';
	import type { ProfileOutput } from '$lib/api/profile';
	import ProfileWizard from '$lib/components/profile/ProfileWizard.svelte';
	import Spinner from '$lib/components/Spinner.svelte';

	let profile = $state<ProfileOutput | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		// Read vault ID from URL — avoids depending on in-memory store state
		// which may be reset after navigation in Tauri on Windows
		const vaultId = $page.url.searchParams.get('vault');
		if (!vaultId) {
			goto('/people');
			return;
		}

		try {
			await profileStore.loadProfiles(vaultId);
			if (profileStore.profiles.length === 0) {
				goto('/profile/setup');
				return;
			}
			profile = await profileAPI.get(vaultId, profileStore.profiles[0].id);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	});
</script>

{#if loading}
	<div class="flex flex-col items-center justify-center py-24">
		<Spinner color="indigo" />
	</div>
{:else if error}
	<div class="mx-auto max-w-2xl px-4 py-8">
		<p class="text-red-600">{error}</p>
		<a href="/people" class="mt-4 inline-block text-sm text-primary-600 hover:underline"
			>← Back to People</a
		>
	</div>
{:else if profile}
	<ProfileWizard
		mode="edit"
		profileId={profile.id}
		initialData={profile}
		onComplete={() => goto('/people')}
		onCancel={() => goto('/people')}
	/>
{/if}
