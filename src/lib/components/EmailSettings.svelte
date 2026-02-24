<script lang="ts">
	interface SettingItem {
		label: string;
		value: string | string[];
	}

	interface Props {
		settings: SettingItem[];
	}

	let { settings }: Props = $props();
</script>

<div>
	<p class="font-medium text-gray-900">SMTP & IMAP Settings:</p>
	<ul class="ml-4 mt-1 list-disc space-y-1">
		{#each settings as setting}
			<li>
				{setting.label}:
				{#if Array.isArray(setting.value)}
					{#each setting.value as val, i}
						<code class="rounded bg-gray-100 px-1">{val}</code>
						{#if i < setting.value.length - 1}
							or
						{/if}
					{/each}
				{:else}
					<code class="rounded bg-gray-100 px-1">{setting.value}</code>
				{/if}
			</li>
		{/each}
	</ul>
</div>
