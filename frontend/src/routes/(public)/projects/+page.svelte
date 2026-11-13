<script lang="ts">
	import { onMount } from 'svelte';

	interface Repo {
		name: string;
		description: string;
	}
	interface Group {
		owner: string;
		repos: Repo[];
	}

	let groups: Group[] = $state([]);
	let error: string | null = $state(null);

	function isRecord(v: unknown): v is Record<string, unknown> {
		return typeof v === 'object' && v !== null && !Array.isArray(v);
	}

	/** Parse the list file (spec Data Model) at the boundary; any other shape is an error. */
	function parseList(body: unknown): Group[] {
		if (!isRecord(body) || typeof body.fetched_at !== 'string' || !Array.isArray(body.groups)) {
			throw new Error('Failed to load');
		}
		// Duplicates are a wrong shape: the keyed each below throws on a repeated key.
		const owners = new Set<string>();
		return body.groups.map((g: unknown): Group => {
			if (!isRecord(g) || typeof g.owner !== 'string' || !Array.isArray(g.repos)) {
				throw new Error('Failed to load');
			}
			if (owners.has(g.owner)) throw new Error('Failed to load');
			owners.add(g.owner);
			const names = new Set<string>();
			return {
				owner: g.owner,
				repos: g.repos.map((r: unknown): Repo => {
					if (!isRecord(r) || typeof r.name !== 'string' || typeof r.description !== 'string') {
						throw new Error('Failed to load');
					}
					if (names.has(r.name)) throw new Error('Failed to load');
					names.add(r.name);
					return { name: r.name, description: r.description };
				}),
			};
		});
	}

	// Loaded in the browser, not at build time: the list file exists only on the deployed server (R-16).
	onMount(async () => {
		try {
			const res = await fetch('/projects.json');
			if (!res.ok) throw new Error('Failed to load');
			groups = parseList(await res.json());
		} catch {
			groups = [];
			error = 'Failed to load';
		}
	});
</script>

<svelte:head>
	<title>Projects — Peter Manahan</title>
</svelte:head>

<div class="container page">
	<h1>Projects</h1>
	{#if error}
		<p class="error" role="alert">{error}</p>
	{:else if groups.length === 0}
		<p class="empty">No projects yet.</p>
	{:else}
		{#each groups as group (group.owner)}
			<section class="group">
				<h2>{group.owner}</h2>
				<ul class="repos">
					{#each group.repos as repo (repo.name)}
						<li>
							<a
								href="https://github.com/{group.owner}/{repo.name}"
								target="_blank"
								rel="noopener noreferrer">{repo.name}</a
							>{#if repo.description}
								<p class="description">{repo.description}</p>
							{/if}
						</li>
					{/each}
				</ul>
			</section>
		{/each}
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

	.group {
		margin-bottom: 2rem;
	}

	h2 {
		font-size: clamp(1.125rem, 4vw, 1.375rem);
		margin-bottom: 1rem;
	}

	.repos {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(300px, 100%), 1fr));
		gap: 1.5rem;
		list-style: none;
		padding: 0;
		margin: 0;
	}

	.repos a {
		font-weight: 600;
		text-decoration: underline;
	}

	.description {
		color: var(--color-text-muted);
		margin-top: 0.25rem;
	}

	.empty, .error {
		color: var(--color-text-muted);
	}

	.error {
		color: var(--color-gap);
	}
</style>
