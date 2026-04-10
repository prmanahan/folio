<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { Profile, Link, Skill } from '$lib/types';
	import Hero from '$lib/components/Hero.svelte';
	import HeroSkeleton from '$lib/components/HeroSkeleton.svelte';

	let profile: Profile | null = $state(null);
	let links: Link[] = $state([]);
	let skills: Skill[] = $state([]);
	let error: string | null = $state(null);
	let loading = $state(true);

	// Callback for Ask AI card — dispatched as a custom event to the layout
	// The layout manages AiPane open/close state
	function handleToggleAi() {
		const event = new CustomEvent('toggle-ai-pane', { bubbles: true });
		document.dispatchEvent(event);
	}

	onMount(async () => {
		try {
			[profile, links, skills] = await Promise.all([
				api.getProfile(),
				api.getLinks(),
				api.getSkills(),
			]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load';
		} finally {
			loading = false;
		}
	});
</script>

{#if loading && !profile}
	<HeroSkeleton />
{:else if error && !profile}
	<div class="hub-error container">
		<div class="error-block">
			<p class="error">{error}</p>
		</div>
		<!-- Nav cards render even on error (they are static) -->
		<div class="nav-cards-fallback">
			<a href="/projects" class="card card-nav">
				<span class="card-label">Projects</span>
			</a>
			<a href="/articles" class="card card-nav">
				<span class="card-label">Articles</span>
			</a>
			<div class="card card-nav">
				<span class="card-label">Contact</span>
				<span class="card-desc">Contact links unavailable</span>
			</div>
		</div>
	</div>
{:else if profile}
	<Hero {profile} {links} {skills} onToggleAi={handleToggleAi} />
{/if}

<style>
	.hub-error {
		min-height: 100svh;
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: 2rem;
		padding: 2rem 1.5rem;
	}

	.error-block {
		padding: 2rem;
		text-align: center;
	}

	.error {
		color: var(--color-gap);
		font-size: 0.9375rem;
	}

	.nav-cards-fallback {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.card {
		border-radius: var(--radius-md);
		padding: 1rem 1.25rem;
		min-height: 44px;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		text-decoration: none;
		color: var(--color-text);
	}

	.card-nav {
		border: 1px solid rgba(176, 141, 87, 0.40);
		background: transparent;
		transition: border-color 0.2s ease, background 0.2s ease;
	}

	.card-nav:hover {
		border-color: var(--raw-brass-light);
		background: rgba(176, 141, 87, 0.08);
		text-decoration: none;
	}

	.card-label {
		font-family: var(--font-heading);
		font-size: 0.8125rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--color-accent-light);
	}

	.card-desc {
		font-size: 0.8125rem;
		color: var(--color-text-muted);
	}

	@media (min-width: 768px) {
		.nav-cards-fallback {
			flex-direction: row;
		}

		.nav-cards-fallback .card-nav {
			flex: 1;
		}
	}
</style>
