<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { Article } from '$lib/types';
	import ArticleCard from '$lib/components/ArticleCard.svelte';

	let articles: Article[] = $state([]);
	let error: string | null = $state(null);

	onMount(async () => {
		try {
			articles = await api.getArticles();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load';
		}
	});
</script>

<svelte:head>
	<title>Articles — Peter Manahan</title>
</svelte:head>

<div class="container page">
	<h1>Articles</h1>
	{#if error}
		<p class="error">{error}</p>
	{:else if articles.length === 0}
		<p class="empty">No articles yet.</p>
	{:else}
		{#each articles as article}
			<ArticleCard {article} />
		{/each}
	{/if}
</div>

<style>
	.page {
		padding: 3rem 0;
	}

	h1 {
		font-size: 2rem;
		margin-bottom: 1rem;
	}

	.empty, .error {
		color: var(--color-text-muted);
	}

	.error { color: var(--color-gap); }
</style>
