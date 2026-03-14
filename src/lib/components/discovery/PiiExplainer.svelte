<script lang="ts">
	let expanded = $state(true);
</script>

<div class="mb-6 rounded-lg border border-blue-200 bg-blue-50">
	<button
		onclick={() => (expanded = !expanded)}
		class="cursor-pointer w-full flex items-center justify-between p-4 text-left"
	>
		<div class="flex items-center gap-2">
			<span class="text-lg">&#x1F6E1;</span>
			<h2 class="text-base font-semibold text-blue-900">What is PII and why scan for it?</h2>
		</div>
		<span class="text-blue-600 text-xl">{expanded ? '▼' : '▶'}</span>
	</button>

	{#if expanded}
		<div class="px-4 pb-4 text-sm text-blue-800 space-y-4">
			<div>
				<h3 class="font-semibold mb-1">What is Personally Identifiable Information (PII)?</h3>
				<p>
					PII is any information that can identify you: name, email, phone, SSN, address, date of
					birth. When stored in unprotected files, it creates privacy and security risks.
				</p>
			</div>

			<div>
				<h3 class="font-semibold mb-1">Why scan for PII?</h3>
				<ul class="list-disc list-inside space-y-1 ml-2">
					<li><strong>Data breaches:</strong> Exposed PII can be stolen for identity theft</li>
					<li><strong>Privacy:</strong> Old files may contain sensitive info you forgot about</li>
					<li><strong>Peace of mind:</strong> Know exactly where your sensitive data lives</li>
				</ul>
			</div>

			<div class="mt-4 p-3 bg-blue-100 rounded-lg">
				<p class="font-medium text-blue-900">&#x1F512; Your privacy is protected</p>
				<p class="text-blue-700 text-xs mt-1">
					All scanning happens locally. No data leaves your computer.
				</p>
			</div>
		</div>
	{/if}
</div>
