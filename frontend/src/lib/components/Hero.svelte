<script lang="ts">
	import type { Profile, Link, Skill } from '$lib/types';
	import GearsBackground from './GearsBackground.svelte';
	import SkillsBanner from './SkillsBanner.svelte';

	interface Props {
		profile: Profile;
		links: Link[];
		skills?: Skill[];
	}

	let { profile, links, skills = [] }: Props = $props();

	// Split name into characters for stamp-in entrance animation
	// aria-label on h1 covers screen reader announcement; spans are aria-hidden
	let nameChars = $derived(profile.name.split(''));

	// Split elevator pitch into paragraphs
	let pitchParagraphs = $derived(profile.elevator_pitch.split('\n\n'));
</script>

<!--
  Hero section — full viewport, layered background, content hierarchy
  Size: calc(100svh - var(--nav-height)) per Peter's decision
-->
<section class="hero">
	<!-- Layer 0: Animated gear assembly (behind everything) -->
	<GearsBackground />

	<!-- Layer 1: Noise texture (via ::before pseudo-element in CSS) -->

	<!-- Layer 2: Content -->
	<div class="hero-content container">
		<!--
			Name: letter-stamp stagger entrance animation
			aria-label on h1 provides clean announcement to screen readers
			individual char spans are aria-hidden
		-->
		<h1 class="hero-name" aria-label={profile.name}>
			{#each nameChars as char, i}
				{#if char === ' '}
					<span class="char space" aria-hidden="true">&nbsp;</span>
				{:else}
					<span
						class="char"
						aria-hidden="true"
						style="--char-index: {i};"
					>{char}</span>
				{/if}
			{/each}
		</h1>

		<!-- Title: fades in after name completes -->
		<p class="hero-title">{profile.title}</p>

		<!-- Elevator pitch: fades in after title -->
		<div class="hero-pitch">
			{#each pitchParagraphs as paragraph}
				<p>{paragraph}</p>
			{/each}
		</div>

		<!-- Meta line: location, availability, remote preference -->
		<div class="hero-meta">
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

		<!-- Link buttons -->
		{#if links.length > 0}
			<div class="hero-links">
				{#each links as link}
					<a
						href={link.url}
						target="_blank"
						rel="noopener noreferrer"
						class="link-item"
					>{link.label}</a>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Layer 3: Skills banner — full bleed strip at bottom of hero -->
	<SkillsBanner {skills} />
</section>

<style>
	/* ============================================================
	   Hero — full viewport section
	   ============================================================ */
	.hero {
		position: relative;
		min-height: calc(100svh - var(--nav-height));
		display: flex;
		flex-direction: column;
		justify-content: center;
		overflow: hidden; /* clips gear layer at viewport edge */

		/* Mobile: padding accounts for nav + breathing room + banner */
		padding-top: calc(var(--nav-height) + 2rem);
		padding-bottom: 5rem; /* space for skills banner */
	}

	/* Noise texture overlay — subtle film grain over the background */
	.hero::before {
		content: '';
		position: absolute;
		inset: 0;
		background-image: url('/textures/noise.png');
		background-repeat: repeat;
		opacity: 0.04;
		pointer-events: none;
		z-index: 1;
	}

	.hero-content {
		position: relative;
		z-index: 2;
	}

	/* ============================================================
	   Name — letter-stamp entrance animation
	   ============================================================ */
	.hero-name {
		font-family: var(--font-heading);
		font-size: clamp(2.25rem, 8vw, 2.75rem);
		font-weight: 700;
		letter-spacing: 0.06em;
		color: var(--color-gold);
		line-height: 1.1;
	}

	/*
		Each character stamps in with a slight press-into-surface feel.
		13 chars × 40ms = 520ms stagger, completes at 920ms total.
		animation-fill-mode: forwards keeps each char visible after animating.
	*/
	.char {
		display: inline-block;
		opacity: 0;
		transform: translateY(4px) scaleY(1.05);
		animation: stamp-in 400ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
		animation-delay: calc(var(--char-index, 0) * 40ms);
	}

	@keyframes stamp-in {
		to {
			opacity: 1;
			transform: translateY(0) scaleY(1);
		}
	}

	/* ============================================================
	   Title — fades in after name completes (600ms delay)
	   ============================================================ */
	.hero-title {
		font-family: var(--font-body);
		font-size: 1rem;
		color: var(--color-text-muted);
		margin-top: 0.375rem;
		line-height: 1.4;

		opacity: 0;
		transform: translateY(6px);
		animation: fade-up 400ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
		animation-delay: 600ms;
	}

	/* ============================================================
	   Elevator pitch — fades in at 800ms
	   ============================================================ */
	.hero-pitch {
		margin-top: 1.25rem;
		opacity: 0;
		animation: fade-in 500ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
		animation-delay: 800ms;
	}

	.hero-pitch p {
		font-size: 0.9375rem;
		line-height: 1.75;
		color: var(--color-text);
		/* Mobile: no max-width constraint, full column */
	}

	.hero-pitch p + p {
		margin-top: 0.75rem;
	}

	/* ============================================================
	   Meta line — fades in at 1000ms
	   ============================================================ */
	.hero-meta {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem 0.875rem;
		margin-top: 1.25rem;
		font-size: 0.8125rem;
		color: var(--color-text-ghost);

		opacity: 0;
		animation: fade-in 400ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
		animation-delay: 1000ms;
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

	/* ============================================================
	   Links — fades in at 1000ms (same as meta)
	   ============================================================ */
	.hero-links {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-top: 1.75rem;

		opacity: 0;
		animation: fade-in 400ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
		animation-delay: 1000ms;
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

	/* ============================================================
	   Shared entrance animations
	   ============================================================ */
	@keyframes fade-in {
		to { opacity: 1; }
	}

	@keyframes fade-up {
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	/* ============================================================
	   Reduced motion — all content immediately visible, no animation
	   The global app.css rule covers this too (animation: none !important)
	   These declarations ensure correct final state without relying on !important
	   ============================================================ */
	@media (prefers-reduced-motion: reduce) {
		.char {
			opacity: 1;
			transform: none;
			animation: none;
		}

		.hero-title,
		.hero-pitch,
		.hero-meta,
		.hero-links {
			opacity: 1;
			transform: none;
			animation: none;
		}
	}

	/* ============================================================
	   Tablet — 768px+
	   ============================================================ */
	@media (min-width: 768px) {
		.hero {
			padding-top: calc(var(--nav-height) + 3rem);
			padding-bottom: 5.5rem;
		}

		.hero-name {
			font-size: clamp(2.5rem, 5vw, 3.25rem);
		}

		.hero-title {
			font-size: 1.125rem;
		}

		.hero-pitch p {
			font-size: 1rem;
			max-width: 52ch;
		}

		.hero-meta {
			font-size: 0.875rem;
			gap: 0.625rem 0.875rem;
		}
	}

	/* ============================================================
	   Desktop — 1024px+
	   ============================================================ */
	@media (min-width: 1024px) {
		.hero {
			min-height: calc(100vh - var(--nav-height));
			padding-top: calc(var(--nav-height) + 4rem);
			padding-bottom: 6rem;
		}

		.hero-name {
			font-size: clamp(2.75rem, 4vw, 3.75rem);
		}

		.hero-title {
			font-size: 1.25rem;
		}

		.hero-links {
			gap: 0.75rem;
		}
	}

	/* ============================================================
	   Wide desktop — 1440px+
	   ============================================================ */
	@media (min-width: 1440px) {
		.hero-name {
			font-size: 3.75rem; /* clamp ceiling */
		}
	}
</style>
