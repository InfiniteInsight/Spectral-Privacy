<script lang="ts">
	import { getPendingFollowups, dismissFollowup, type PendingFollowup } from '$lib/api/followups';

	interface Props {
		vaultId: string;
	}

	let { vaultId }: Props = $props();

	let followups = $state<PendingFollowup[]>([]);
	let loading = $state(true);
	let dismissing = $state<Set<string>>(new Set());

	$effect(() => {
		load();
	});

	async function load() {
		try {
			followups = await getPendingFollowups(vaultId);
		} catch (e) {
			console.error('Failed to load follow-ups:', e);
		} finally {
			loading = false;
		}
	}

	async function handleDismiss(id: string) {
		dismissing = new Set([...dismissing, id]);
		try {
			await dismissFollowup(vaultId, id);
			followups = followups.filter((f) => f.id !== id);
		} catch (e) {
			console.error('Failed to dismiss follow-up:', e);
		} finally {
			dismissing = new Set([...dismissing].filter((x) => x !== id));
		}
	}

	function isOverdue(followUpAt: string): boolean {
		return new Date(followUpAt) <= new Date();
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString(undefined, {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	function brokerLabel(brokerId: string): string {
		return brokerId.charAt(0).toUpperCase() + brokerId.slice(1).replace(/-/g, ' ');
	}
</script>

{#if !loading && followups.length > 0}
	<div class="mb-6 space-y-3">
		<h2 class="text-sm font-semibold uppercase tracking-wide text-gray-700">
			Follow-Up Reminders
			<span
				class="ml-2 inline-flex items-center justify-center rounded-full bg-amber-500 px-2 py-0.5 text-xs font-bold text-white"
			>
				{followups.length}
			</span>
		</h2>

		{#each followups as followup (followup.id)}
			{@const overdue = isOverdue(followup.follow_up_at)}
			<div
				class="flex items-start justify-between rounded-lg border p-4 {overdue
					? 'border-amber-300 bg-amber-50'
					: 'border-blue-200 bg-blue-50'}"
			>
				<div class="min-w-0 flex-1">
					<div class="mb-1 flex items-center gap-2">
						<span
							class="inline-block rounded-full px-2 py-0.5 text-xs font-medium {overdue
								? 'bg-amber-200 text-amber-900'
								: 'bg-blue-200 text-blue-900'}"
						>
							{overdue ? 'Follow-up overdue' : 'Follow-up scheduled'}
						</span>
						<span class="text-xs text-gray-500">{brokerLabel(followup.broker_id)}</span>
					</div>

					<p class="text-sm text-gray-800">
						{overdue
							? `Your removal request to ${followup.recipient} needs a follow-up — due ${formatDate(followup.follow_up_at)}.`
							: `Scheduled to follow up with ${followup.recipient} on ${formatDate(followup.follow_up_at)}.`}
					</p>

					{#if overdue}
						<p class="mt-1 text-xs text-amber-800">
							Connect SMTP + an LLM provider in Settings → Email to have Spectral send this
							follow-up automatically.
						</p>
					{/if}
				</div>

				<button
					onclick={() => handleDismiss(followup.id)}
					disabled={dismissing.has(followup.id)}
					class="ml-4 shrink-0 rounded px-3 py-1.5 text-xs font-medium transition-colors disabled:opacity-50
                           {overdue
						? 'bg-amber-200 text-amber-900 hover:bg-amber-300'
						: 'bg-blue-200 text-blue-900 hover:bg-blue-300'}"
				>
					{dismissing.has(followup.id) ? 'Dismissing…' : 'Dismiss'}
				</button>
			</div>
		{/each}
	</div>
{/if}
