<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { Project } from '$lib/types';
	import ProjectCard from '$lib/components/ProjectCard.svelte';

	let projects: Project[] = $state([]);
	let error: string | null = $state(null);

	onMount(async () => {
		try {
			projects = await api.getProjects();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load';
		}
	});
</script>

<svelte:head>
	<title>Projects — Peter Manahan</title>
</svelte:head>

<div class="container page">
	<h1>Projects</h1>
	{#if error}
		<p class="error">{error}</p>
	{:else if projects.length === 0}
		<p class="empty">No projects yet.</p>
	{:else}
		<div class="grid">
			{#each projects as project}
				<ProjectCard {project} />
			{/each}
		</div>
	{/if}
</div>

<style>
	.page {
		padding: 3rem 0;
	}

	h1 {
		font-size: clamp(1.5rem, 5vw, 2rem);
		margin-bottom: 2rem;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(300px, 100%), 1fr));
		gap: 1.5rem;
	}

	.empty, .error {
		color: var(--color-text-muted);
	}

	.error {
		color: var(--color-gap);
	}
</style>
