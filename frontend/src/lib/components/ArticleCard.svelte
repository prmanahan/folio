<script lang="ts">
	import type { Article } from '$lib/types';

	let { article }: { article: Article } = $props();
</script>

<a href="/articles/{article.slug}" class="card">
	<h3>{article.title}</h3>
	<p class="summary">{article.summary}</p>
	<div class="meta">
		{#if article.published_at}
			<span class="date">{article.published_at}</span>
		{/if}
		{#if article.tags && article.tags.length > 0}
			<div class="tags">
				{#each article.tags as tag}
					<span class="tag">{tag}</span>
				{/each}
			</div>
		{/if}
	</div>
</a>

<style>
	.card {
		display: block;
		padding: 1.5rem 0;
		border-bottom: 1px solid var(--color-border-subtle);
		color: var(--color-text);
	}

	.card:hover {
		text-decoration: none;
	}

	.card:hover h3 {
		color: var(--color-gold);
	}

	h3 {
		font-family: var(--font-heading);
		font-size: 1.0625rem;
		font-weight: 400;
		letter-spacing: 0.04em;
		transition: color 0.2s ease;
	}

	.summary {
		margin-top: 0.35rem;
		font-size: 0.875rem;
		color: var(--color-text-muted);
	}

	.meta {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-top: 0.5rem;
	}

	.date {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--color-text-ghost);
	}

	.tags {
		display: flex;
		gap: 0.4rem;
	}

	.tag {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		color: var(--color-text-ghost);
		background: rgba(176, 141, 87, 0.08);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		padding: 0.1rem 0.4rem;
	}
</style>
