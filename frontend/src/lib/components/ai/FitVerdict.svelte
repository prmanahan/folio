<script lang="ts">
	import type { FitVerdict } from '$lib/types';

	let { verdict }: { verdict: FitVerdict } = $props();

	const verdictConfig = {
		strong_fit: { label: 'Strong Fit', cssClass: 'verdict-strong' },
		worth_conversation: { label: 'Worth a Conversation', cssClass: 'verdict-conversation' },
		probably_not: { label: 'Probably Not a Fit', cssClass: 'verdict-not' },
	};

	const config = $derived(verdictConfig[verdict.verdict]);
</script>

<div class="fit-verdict">
	<div class="verdict-badge-row">
		<span class="verdict-badge {config.cssClass}">{config.label}</span>
	</div>

	<h2 class="headline">{verdict.headline}</h2>

	<p class="opening">{verdict.opening}</p>

	{#if verdict.gaps.length > 0}
		<section class="section">
			<h3 class="section-heading">Gaps</h3>
			<div class="gap-list">
				{#each verdict.gaps as gap}
					<div class="gap-card">
						<div class="gap-header">
							<span class="gap-requirement">{gap.requirement}</span>
							<span class="gap-arrow">→</span>
							<span class="gap-title">{gap.gap_title}</span>
						</div>
						<p class="gap-explanation">{gap.explanation}</p>
					</div>
				{/each}
			</div>
		</section>
	{/if}

	{#if verdict.transfers.length > 0}
		<section class="section">
			<h3 class="section-heading">Transferable Skills</h3>
			<div class="transfer-list">
				{#each verdict.transfers as transfer}
					<div class="transfer-row">
						<span class="transfer-skill">{transfer.skill}</span>
						<span class="transfer-arrow">→</span>
						<span class="transfer-relevance">{transfer.relevance}</span>
					</div>
				{/each}
			</div>
		</section>
	{/if}

	<section class="section recommendation-section">
		<h3 class="section-heading">Recommendation</h3>
		<p class="recommendation">{verdict.recommendation}</p>
	</section>
</div>

<style>
	.fit-verdict {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.verdict-badge-row {
		display: flex;
	}

	.verdict-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		border-radius: var(--radius-sm);
		font-family: var(--font-heading);
		font-size: 0.6875rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.10em;
		border: 1.5px solid currentColor;
		background: transparent;
	}

	.headline {
		font-family: var(--font-heading);
		font-size: 1rem;
		font-weight: 400;
		letter-spacing: 0.04em;
		color: var(--color-text);
		line-height: 1.4;
	}

	.opening {
		font-size: 0.875rem;
		color: var(--color-text);
		line-height: 1.6;
	}

	.section {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.section-heading {
		font-family: var(--font-heading);
		font-size: 0.625rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		color: var(--color-text-ghost);
	}

	.gap-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.gap-card {
		padding: 0.75rem;
		background: var(--color-surface);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.gap-header {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		flex-wrap: wrap;
	}

	.gap-requirement {
		font-size: 0.8125rem;
		font-weight: 600;
		color: var(--color-text);
	}

	.gap-arrow {
		font-size: 0.8125rem;
		color: var(--color-text-ghost);
	}

	.gap-title {
		font-size: 0.8125rem;
		color: var(--color-gap);
		font-weight: 500;
	}

	.gap-explanation {
		font-size: 0.8125rem;
		color: var(--color-text-muted);
		line-height: 1.5;
	}

	.transfer-list {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.transfer-row {
		display: flex;
		align-items: baseline;
		gap: 0.375rem;
		flex-wrap: wrap;
		padding: 0.5rem 0.75rem;
		background: var(--color-surface);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
	}

	.transfer-skill {
		font-size: 0.8125rem;
		font-weight: 600;
		color: var(--color-text);
		white-space: nowrap;
	}

	.transfer-arrow {
		font-size: 0.8125rem;
		color: var(--color-text-ghost);
		flex-shrink: 0;
	}

	.transfer-relevance {
		font-size: 0.8125rem;
		color: var(--color-text-muted);
		line-height: 1.5;
	}

	.recommendation-section {
		padding-top: 0.25rem;
		border-top: 1px solid var(--color-border-subtle);
	}

	.recommendation {
		font-size: 0.875rem;
		color: var(--color-text);
		line-height: 1.6;
	}
</style>
