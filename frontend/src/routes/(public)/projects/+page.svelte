<script lang="ts">
	import { onMount } from 'svelte';
	import { isInvisible } from '#lib/invisible.ts';

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

	/** True when `v` is a record whose keys are exactly `keys` (no unknown key, none missing). */
	function hasExactKeys(v: Record<string, unknown>, keys: readonly string[]): boolean {
		const own = Object.keys(v);
		return own.length === keys.length && keys.every((k) => own.includes(k));
	}

	/** The Data Model's rejected class: \p{Cc}, U+2028, U+2029, and the generated invisible table. */
	function hasRejectedCodePoint(text: string): boolean {
		for (const ch of text) {
			const cp = ch.codePointAt(0) ?? 0;
			// \p{Cs}: `for...of` yields an unpaired surrogate as its own element (R-4's lone-surrogate rule).
			if (/\p{Cc}|\p{Cs}/u.test(ch) || cp === 0x2028 || cp === 0x2029 || isInvisible(cp)) return true;
		}
		return false;
	}

	/** `YYYY-MM-DDTHH:MM:SS.sssZ` and a real calendar instant (Rust `parse_fetched_at`). */
	const FETCHED_AT_SHAPE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/;
	function isFetchedAt(s: string): boolean {
		// `toISOString()` throws RangeError on an invalid date (for example Feb 30 parses to NaN); the
		// caller's try lands that on the unavailable line. A rolled-over date (Feb 30 -> Mar 2) differs
		// from `s`, so the equality rejects it.
		return FETCHED_AT_SHAPE.test(s) && new Date(s).toISOString() === s;
	}

	/** R-8 order: strictly increasing by ASCII-lowercased name, by code unit. Equal names (case twins) fail. */
	function asciiLower(s: string): string {
		return s.replace(/[A-Z]/g, (c) => c.toLowerCase());
	}

	const NAME_PATTERN = /^[A-Za-z0-9._-]{1,100}$/;
	const MAX_DESCRIPTION_CODE_POINTS = 350;

	/**
	 * Parse the held list (adopted Data Model, R-8/R-10 rules) at the boundary; any other document is an
	 * error. Exactly two groups, `mnemra` then `prmanahan`, each with at least one repo; names match
	 * NAME_PATTERN and strictly increase per owner (R-8, ASCII-lowercased), so a repeated name, which the
	 * keyed each below would throw on, is rejected here; `fetched_at` has the exact grammar;
	 * descriptions are at most 350 code points with no rejected-class or lone-surrogate code point;
	 * no unknown key at any level.
	 */
	function parseList(body: unknown): Group[] {
		const fail = (): never => {
			throw new Error('Failed to load');
		};
		if (!isRecord(body) || !hasExactKeys(body, ['fetched_at', 'groups'])) return fail();
		if (typeof body.fetched_at !== 'string' || !isFetchedAt(body.fetched_at)) return fail();
		if (!Array.isArray(body.groups)) return fail();
		if (body.groups.length !== OWNERS.length) return fail();
		return body.groups.map((g: unknown, i): Group => {
			if (!isRecord(g) || !hasExactKeys(g, ['owner', 'repos'])) return fail();
			const owner = OWNERS[i];
			if (typeof g.owner !== 'string' || g.owner !== owner) return fail();
			if (!Array.isArray(g.repos) || g.repos.length === 0) return fail();
			let prevName: string | null = null;
			return {
				owner,
				repos: g.repos.map((r: unknown): Repo => {
					if (!isRecord(r) || !hasExactKeys(r, ['name', 'description'])) return fail();
					if (typeof r.name !== 'string' || typeof r.description !== 'string') return fail();
					if (!NAME_PATTERN.test(r.name)) return fail();
					// Strictly increasing also rejects a repeated name and a case-only twin.
					if (prevName !== null && !(asciiLower(prevName) < asciiLower(r.name))) return fail();
					if ([...r.description].length > MAX_DESCRIPTION_CODE_POINTS) return fail();
					if (hasRejectedCodePoint(r.description)) return fail();
					prevName = r.name;
					return { name: r.name, description: r.description };
				}),
			};
		});
	}

	// Loaded in the browser from the held list (R-24). The timeout is setTimeout + AbortController,
	// not AbortSignal.timeout: the edge test's page.clock can fake only the former (T17, LL-PW-4).
	onMount(() => {
		const controller = new AbortController();
		let disposed = false;
		const showUnavailable = () => {
			// After unmount nothing is rendered, so no state is written.
			if (disposed) return;
			groups = [];
			status = 'unavailable';
		};
		// The timer shows the line itself: it does not rely on fetch rejecting when aborted.
		const timer = setTimeout(() => {
			controller.abort();
			showUnavailable();
		}, TIMEOUT_MS);
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
				showUnavailable();
			} finally {
				clearTimeout(timer);
			}
		})();
		return () => {
			disposed = true;
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
