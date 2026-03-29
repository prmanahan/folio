<script lang="ts">
	import type { Project } from '$lib/types';

	let { project }: { project: Project } = $props();
</script>

<a href="/projects/{project.slug}" class="card">
	<h3>{project.title}</h3>
	<p class="summary">{project.summary}</p>
	{#if project.tech_stack && project.tech_stack.length > 0}
		<div class="tags">
			{#each project.tech_stack as tech}
				<span class="tag">{tech}</span>
			{/each}
		</div>
	{/if}
</a>

<style>
	.card {
		display: block;
		padding: 1.5rem;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-top: 2px solid var(--raw-brass);
		border-radius: var(--radius-md);
		color: var(--color-text);
		box-shadow:
			0 4px 16px rgba(0, 0, 0, 0.5),
			inset 0 1px 0 rgba(232, 201, 122, 0.08);
		transition: transform 0.2s ease, box-shadow 0.2s ease;
	}

	.card:hover {
		transform: translateY(-2px);
		box-shadow:
			0 8px 24px rgba(0, 0, 0, 0.6),
			inset 0 1px 0 rgba(232, 201, 122, 0.12);
		text-decoration: none;
	}

	h3 {
		font-family: var(--font-heading);
		font-size: 1.0625rem;
		font-weight: 400;
		letter-spacing: 0.04em;
		color: var(--color-text);
		transition: color 0.2s ease;
	}

	.card:hover h3 {
		color: var(--color-gold);
	}

	.summary {
		margin-top: 0.5rem;
		font-size: 0.875rem;
		color: var(--color-text-muted);
		line-height: 1.5;
	}

	.tags {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
		margin-top: 0.75rem;
	}

	.tag {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		color: var(--color-text-ghost);
		background: rgba(176, 141, 87, 0.08);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		padding: 0.125rem 0.5rem;
	}
</style>
