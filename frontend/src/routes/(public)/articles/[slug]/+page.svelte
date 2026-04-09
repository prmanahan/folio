<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { api } from '$lib/api';
	import type { Article } from '$lib/types';
	import { marked } from 'marked';
	import DOMPurify from 'dompurify';

	let article: Article | null = $state(null);
	let error: string | null = $state(null);

	onMount(async () => {
		try {
			const slug = page.params.slug!;
			article = await api.getArticle(slug);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load';
		}
	});
</script>

<svelte:head>
	<title>{article ? `${article.title} — Articles` : 'Article'} — Peter Manahan</title>
</svelte:head>

<div class="container page">
	{#if error}
		<p class="error">{error}</p>
	{:else if article}
		<a href="/articles" class="back">&larr; Articles</a>
		<h1>{article.title}</h1>
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
		<div class="prose">
			{@html DOMPurify.sanitize(marked.parse(article.content) as string)}
		</div>
	{:else}
		<p class="loading">Loading...</p>
	{/if}
</div>

<style>
	.page {
		padding: 3rem 0;
	}

	.back {
		font-size: 0.9rem;
		color: var(--color-text-muted);
		display: inline-flex;
		align-items: center;
		min-height: 44px;
		margin-bottom: 1rem;
	}

	h1 {
		font-size: clamp(1.5rem, 5vw, 2rem);
		margin-bottom: 0.5rem;
	}

	.meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 2rem;
	}

	.date {
		font-size: 0.85rem;
		color: var(--color-text-muted);
	}

	.tags {
		display: flex;
		gap: 0.4rem;
	}

	.tag {
		padding: 0.15rem 0.5rem;
		background: var(--color-surface);
		border-radius: 3px;
		font-size: 0.8rem;
		color: var(--color-text-muted);
	}

	.prose {
		line-height: 1.8;
		font-size: 1rem;
	}

	.prose :global(h2) { font-size: 1.4rem; margin-top: 2rem; margin-bottom: 0.75rem; }
	.prose :global(h3) { font-size: 1.2rem; margin-top: 1.5rem; margin-bottom: 0.5rem; }
	.prose :global(p) { margin-bottom: 1rem; }
	.prose :global(ul), .prose :global(ol) { margin-bottom: 1rem; padding-left: 1.5rem; }
	.prose :global(code) { font-family: var(--font-mono); background: var(--color-surface); padding: 0.15rem 0.3rem; border-radius: 3px; font-size: 0.9em; }
	.prose :global(pre) { background: var(--color-surface); padding: 1rem; border-radius: 6px; overflow-x: auto; margin-bottom: 1rem; }
	.prose :global(pre code) { background: none; padding: 0; }
	.prose :global(img) { max-width: 100%; height: auto; }
	.prose :global(table) { display: block; overflow-x: auto; width: 100%; }

	.loading, .error {
		color: var(--color-text-muted);
		padding: 2rem 0;
	}

	.error { color: var(--color-gap); }
</style>
