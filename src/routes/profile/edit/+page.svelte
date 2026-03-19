<script lang="ts">
	import ProfileWizard from '$lib/components/profile/ProfileWizard.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<ProfileWizard mode="edit" profileId={data.profile.id} initialData={data.profile} />
