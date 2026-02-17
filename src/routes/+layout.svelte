<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import NavBar from '$lib/components/nav/NavBar.svelte';
	import { vaultStore } from '$lib/stores/vault.svelte';

	let { children } = $props();

	$effect(() => {
		vaultStore.loadVaults();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<a href="#main-content" class="skip-link">Skip to main content</a>

<div id="app" class="min-h-screen bg-gray-50">
	<NavBar />
	<main id="main-content" tabindex="-1">
		{@render children()}
	</main>
</div>
