<script lang="ts">
	import VaultSwitcher from './VaultSwitcher.svelte';
</script>

<nav class="border-b border-gray-200 bg-white">
	<div class="mx-auto flex max-w-5xl items-center justify-between px-4 py-3">
		<div class="flex items-center gap-6">
			<a href="/" class="text-lg font-bold text-primary-700">Spectral</a>
			<a href="/people" class="text-sm text-gray-600 hover:text-gray-900">People</a>
			<a href="/removals" class="text-sm text-gray-600 hover:text-gray-900">History</a>
			<a href="/score" class="text-sm text-gray-600 hover:text-gray-900">Score</a>
			<a href="/brokers" class="text-sm text-gray-600 hover:text-gray-900">Brokers</a>
			<a href="/discovery" class="text-sm text-gray-600 hover:text-gray-900">Discovery</a>
		</div>
		<div class="flex items-center gap-3">
			<VaultSwitcher />
			<a
				href="/settings"
				aria-label="Settings"
				class="rounded-md p-1.5 text-gray-500 hover:bg-gray-100"
			>
				<svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
					/>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
					/>
				</svg>
			</a>
		</div>
	</div>
</nav>
