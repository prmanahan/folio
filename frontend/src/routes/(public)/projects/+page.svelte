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

	/** R-24: a request not settled after 15 s is aborted and counts as a network error. */
	const TIMEOUT_MS = 15_000;
	/** R-24's one line, shown for every failed load. */
	const UNAVAILABLE = 'The project list is temporarily unavailable; try again shortly.';
	// The two owners the list is drawn from; R-24 links to both when the list is unavailable.
	const OWNERS = ['mnemra', 'prmanahan'] as const;

	type LoadState = 'loading' | 'ready' | 'unavailable';
	let groups: Group[] = $state([]);
	let status: LoadState = $state('loading');

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

	// Loaded in the browser from the held list (R-24). The timeout is setTimeout + AbortController,
	// not AbortSignal.timeout: the edge test's page.clock can fake only the former (T17, LL-PW-4).
	onMount(() => {
		const controller = new AbortController();
		const timer = setTimeout(() => controller.abort(), TIMEOUT_MS);
		void (async () => {
			try {
				const res = await fetch('/api/lists/projects', { signal: controller.signal });
				if (!res.ok) throw new Error('Failed to load');
				const parsed = parseList(await res.json());
				// A response arriving after the abort is never applied.
				if (controller.signal.aborted) throw new Error('Failed to load');
				groups = parsed;
				status = 'ready';
			} catch {
				groups = [];
				status = 'unavailable';
			} finally {
				clearTimeout(timer);
			}
		})();
		return () => {
			clearTimeout(timer);
			controller.abort();
		};
	});
</script>

<svelte:head>
	<title>Projects — Peter Manahan</title>
</svelte:head>

<div class="container page">
	<h1>Projects</h1>
	{#if status === 'loading'}
		<!-- R-24: only the heading while pending. -->
	{:else if status === 'unavailable'}
		<p class="error" role="alert">{UNAVAILABLE}</p>
		<ul class="owner-links">
			{#each OWNERS as owner (owner)}
				<li>
					<a href="https://github.com/{owner}" target="_blank" rel="noopener noreferrer"
						>{owner} on GitHub</a
					>
				</li>
			{/each}
		</ul>
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

	.owner-links {
		list-style: none;
		padding: 0;
		margin: 0.5rem 0 0;
	}

	.owner-links a {
		text-decoration: underline;
	}
</style>
