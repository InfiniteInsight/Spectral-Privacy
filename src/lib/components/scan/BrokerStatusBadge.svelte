<script lang="ts">
	interface Props {
		status: 'Pending' | 'InProgress' | 'Success' | 'Failed' | 'Skipped';
	}

	let { status }: Props = $props();

	const config = $derived.by(() => {
		switch (status) {
			case 'Pending':
				return { color: 'bg-gray-200 text-gray-700', icon: '·' };
			case 'InProgress':
				return { color: 'bg-blue-100 text-blue-700', icon: '⟳' };
			case 'Success':
				return { color: 'bg-green-100 text-green-700', icon: '✓' };
			case 'Failed':
				return { color: 'bg-red-100 text-red-700', icon: '✗' };
			case 'Skipped':
				return { color: 'bg-yellow-100 text-yellow-700', icon: '⊘' };
			default:
				return { color: 'bg-gray-100 text-gray-700', icon: '·' };
		}
	});
</script>

<span class="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium {config.color}">
	<span class="mr-1">{config.icon}</span>
	{status}
</span>
