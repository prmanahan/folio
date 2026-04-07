<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { api } from '$lib/api';
	import type { Project } from '$lib/types';
	import { marked } from 'marked';
	import DOMPurify from 'dompurify';

	let project: Project | null = $state(null);
	let error: string | null = $state(null);

	onMount(async () => {
		try {
			const slug = page.params.slug!;
			project = await api.getProject(slug);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load';
		}
	});
</script>

<svelte:head>
	<title>{project ? `${project.title} — Projects` : 'Project'} — Peter Manahan</title>
</svelte:head>

<div class="container page">
	{#if error}
		<p class="error">{error}</p>
	{:else if project}
		<a href="/projects" class="back">&larr; Projects</a>
		<h1>{project.title}</h1>
		{#if project.tech_stack && project.tech_stack.length > 0}
			<div class="tags">
				{#each project.tech_stack as tech}
					<span class="tag">{tech}</span>
				{/each}
			</div>
		{/if}
		{#if project.url}
			<p class="project-url"><a href={project.url} target="_blank" rel="noopener noreferrer">View project ({project.url.replace(/^https?:\/\//, '').replace(/\/$/, '')}) &rarr;</a></p>
		{/if}
		<div class="prose">
			{@html DOMPurify.sanitize(marked.parse(project.description) as string)}
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
		display: inline-block;
		margin-bottom: 1rem;
	}

	h1 {
		font-size: 2rem;
		margin-bottom: 0.75rem;
	}

	.tags {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.tag {
		padding: 0.2rem 0.5rem;
		background: var(--color-surface);
		border-radius: 4px;
		font-size: 0.8rem;
		font-family: var(--font-mono);
		color: var(--color-text-muted);
	}

	.project-url {
		margin-bottom: 1.5rem;
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

	.loading, .error {
		color: var(--color-text-muted);
		padding: 2rem 0;
	}

	.error { color: var(--color-gap); }
</style>
