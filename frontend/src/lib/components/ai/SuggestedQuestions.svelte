<script lang="ts">
	import { api } from '$lib/api';
	import type { FaqSuggestion } from '$lib/types';

	let { onSelect }: { onSelect: (question: string) => void } = $props();

	let suggestions = $state<FaqSuggestion[]>([]);

	$effect(() => {
		api.getFaqSuggestions().then((data) => {
			suggestions = data;
		}).catch(() => {
			// silently fail — suggestions are non-critical
		});
	});
</script>

{#if suggestions.length > 0}
	<div class="suggested-questions">
		<p class="label">Try asking:</p>
		<div class="chips">
			{#each suggestions as suggestion (suggestion.id)}
				<button class="chip" onclick={() => onSelect(suggestion.question)}>
					{suggestion.question}
				</button>
			{/each}
		</div>
	</div>
{/if}

<style>
	.suggested-questions {
		padding: 1rem 0 0.5rem;
	}

	.label {
		font-family: var(--font-heading);
		font-size: 0.625rem;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		color: var(--color-text-ghost);
		margin-bottom: 0.5rem;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.chip {
		padding: 0.375rem 0.75rem;
		border: 1px solid rgba(176, 141, 87, 0.35);
		border-radius: var(--radius-sm);
		background: rgba(176, 141, 87, 0.06);
		color: var(--color-text-muted);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		cursor: pointer;
		transition: all 0.2s ease;
		text-align: left;
	}

	.chip:hover {
		background: rgba(176, 141, 87, 0.15);
		border-color: var(--color-accent);
		color: var(--color-accent-light);
	}
</style>
