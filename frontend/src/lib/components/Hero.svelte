<script lang="ts">
	import type { Profile, Link } from '$lib/types';

	let { profile, links }: { profile: Profile; links: Link[] } = $props();
</script>

<section class="hero">
	<div class="container">
		<h1>{profile.name}</h1>
		<p class="title">{profile.title}</p>
		{#each profile.elevator_pitch.split('\n\n') as paragraph}
			<p class="pitch">{paragraph}</p>
		{/each}

		<div class="meta">
			{#if profile.location}
				<span class="meta-item">{profile.location}</span>
			{/if}
			{#if profile.availability_status}
				<span class="meta-item status" class:open={profile.availability_status === 'open'}>
					{profile.availability_status === 'open' ? 'Open to opportunities' : profile.availability_status}
				</span>
			{/if}
			{#if profile.remote_preference}
				<span class="meta-item">{profile.remote_preference}</span>
			{/if}
		</div>

		{#if links.length > 0}
			<div class="links">
				{#each links as link}
					<a href={link.url} target="_blank" rel="noopener noreferrer" class="link-item">
						{link.label}
					</a>
				{/each}
			</div>
		{/if}
		<div class="hero-rule" aria-hidden="true"></div>
	</div>
</section>

<style>
	.hero {
		padding: 5rem 0 3.5rem;
	}

	h1 {
		font-family: var(--font-heading);
		font-size: clamp(2rem, 5vw, 3rem);
		font-weight: 900;
		letter-spacing: 0.08em;
		color: var(--color-gold);
		line-height: 1.1;
	}

	.title {
		font-family: var(--font-body);
		font-size: 1.175rem;
		color: var(--color-text-muted);
		margin-top: 0.375rem;
		line-height: 1.4;
	}

	.pitch {
		margin-top: 0;
		margin-bottom: 0.75rem;
		font-size: 1rem;
		max-width: 640px;
		line-height: 1.7;
		color: var(--color-text);
	}

	.pitch:first-of-type {
		margin-top: 1.25rem;
	}

	.pitch:last-of-type {
		margin-bottom: 0;
	}

	.meta {
		display: flex;
		flex-wrap: wrap;
		gap: 0.875rem;
		margin-top: 1.25rem;
		font-size: 0.875rem;
		color: var(--color-text-ghost);
	}

	.status.open {
		color: var(--color-accent-light);
		font-weight: 400;
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}

	.status.open::before {
		content: '';
		display: inline-block;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--color-accent-light);
		box-shadow: 0 0 6px var(--color-accent);
		flex-shrink: 0;
	}

	.links {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		margin-top: 1.75rem;
	}

	.link-item {
		font-family: var(--font-heading);
		font-size: 0.75rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		padding: 0.5rem 1rem;
		border: 1px solid rgba(176, 141, 87, 0.50);
		border-radius: var(--radius-btn);
		color: var(--color-text-muted);
		background: transparent;
		transition: color 0.2s ease, border-color 0.2s ease, background 0.2s ease;
		min-height: 44px;
		display: flex;
		align-items: center;
	}

	.link-item:hover {
		color: var(--color-accent-light);
		border-color: var(--color-accent);
		background: rgba(176, 141, 87, 0.08);
		text-decoration: none;
	}

	.hero-rule {
		margin-top: 3rem;
		height: 1px;
		background: linear-gradient(
			90deg,
			transparent 0%,
			rgba(176, 141, 87, 0.5) 20%,
			rgba(232, 201, 122, 0.6) 50%,
			rgba(176, 141, 87, 0.5) 80%,
			transparent 100%
		);
	}
</style>
