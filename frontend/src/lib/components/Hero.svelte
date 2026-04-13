<script lang="ts">
	import type { Profile, Link, Skill } from '$lib/types';
	import GearsBackground from './GearsBackground.svelte';
	import SkillsBanner from './SkillsBanner.svelte';

	interface Props {
		profile: Profile;
		links: Link[];
		skills?: Skill[];
		onToggleAi?: () => void;
	}

	let { profile, links, skills = [], onToggleAi }: Props = $props();

	// Allowed URL protocols for contact links
	const SAFE_PROTOCOLS = ['https:', 'http:', 'mailto:', 'tel:'];

	let safeLinks = $derived(
		links.filter((link) => {
			try {
				// mailto: and tel: don't parse well with URL constructor
				// Check prefix directly for those
				const lower = link.url.toLowerCase().trim();
				return SAFE_PROTOCOLS.some((p) => lower.startsWith(p));
			} catch {
				return false;
			}
		})
	);

	// Split name into characters for stamp-in entrance animation
	let nameChars = $derived(profile.name.split(''));

	// Split elevator pitch into paragraphs
	let pitchParagraphs = $derived(profile.elevator_pitch.split('\n\n'));
</script>

<section class="hero">
	<!-- Layer 0: Animated gear assembly -->
	<GearsBackground />

	<!-- Layer 1: Noise texture (via ::before) -->

	<!-- Layer 2: Content — two-column hub layout -->
	<div class="hub container">
		<!-- Left column: Identity block -->
		<div class="identity">
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

			<p class="hero-title">{profile.title}</p>

			{#if profile.availability_status}
				<div class="availability-badge" class:open={profile.availability_status === 'open'}>
					<span class="badge-dot" aria-hidden="true"></span>
					{profile.availability_status === 'open' ? 'Open to opportunities' : profile.availability_status}
				</div>
			{/if}
		</div>

		<!-- Right column: Content + cards -->
		<div class="content">
			<!-- Elevator pitch -->
			<div class="hero-pitch">
				{#each pitchParagraphs as paragraph}
					<p>{paragraph}</p>
				{/each}
			</div>

			<!-- Meta line -->
			<div class="hero-meta">
				{#if profile.location}
					<span class="meta-item">{profile.location}</span>
				{/if}
				{#if profile.remote_preference}
					<span class="meta-item">{profile.remote_preference}</span>
				{/if}
			</div>

			<!-- Ask AI card — primary CTA, filled brass -->
			<button class="card card-ai" onclick={onToggleAi} aria-label="Ask AI" data-testid="card-ask-ai">
				<span class="card-icon" aria-hidden="true">&#9881;</span>
				<span class="card-label">Ask AI</span>
				<span class="card-desc">Chat with an AI about my experience</span>
			</button>

			<!-- Navigation cards row: Projects · Articles · Resume -->
			<div class="nav-cards">
				<a href="/projects" class="card card-nav" data-testid="card-projects">
					<span class="card-label">Projects</span>
					<span class="card-desc">Things I've built</span>
				</a>

				<a href="/articles" class="card card-nav" data-testid="card-articles">
					<span class="card-label">Articles</span>
					<span class="card-desc">Thoughts and write-ups</span>
				</a>

				<a href="/resume" class="card card-nav" data-testid="card-resume">
					<span class="card-label">Resume</span>
					<span class="card-desc">Skills, experience, education</span>
				</a>
			</div>

			<!-- Contact card — full-width, brass outline, below nav row -->
			<!-- Functionally distinct: external links, not an internal route -->
			<div class="card card-nav card-contact" data-testid="card-contact">
				<span class="card-label">Contact</span>
				{#if safeLinks.length > 0}
					<div class="contact-links">
						{#each safeLinks as link}
							<a
								href={link.url}
								target="_blank"
								rel="noopener noreferrer"
								class="contact-link"
								aria-label={link.label === 'Resume' ? 'Resume PDF' : link.label}
							>{link.label === 'Resume' ? 'Resume PDF' : link.label}</a>
						{/each}
					</div>
				{:else}
					<span class="card-desc">Contact links unavailable</span>
				{/if}
			</div>
		</div>
	</div>

	<!-- Layer 3: Skills banner at bottom -->
	<SkillsBanner {skills} />
</section>

<style>
	/* ============================================================
	   Hero — full viewport hub layout
	   ============================================================ */
	.hero {
		position: relative;
		min-height: 100svh;
		display: flex;
		flex-direction: column;
		justify-content: center;
		overflow: hidden;
		padding-top: 2rem;
		padding-bottom: 5rem;
	}

	/* Noise texture overlay */
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

	/* ============================================================
	   Hub — two-column layout container
	   ============================================================ */
	.hub {
		position: relative;
		z-index: 2;
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	/* ============================================================
	   Identity block (left column on desktop)
	   ============================================================ */
	.identity {
		display: flex;
		flex-direction: column;
	}

	.hero-name {
		font-family: var(--font-heading);
		font-size: clamp(2.25rem, 8vw, 2.75rem);
		font-weight: 700;
		letter-spacing: 0.06em;
		color: var(--color-gold);
		line-height: 1.1;
		white-space: nowrap;
	}

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
	   Availability badge — teal accent
	   ============================================================ */
	.availability-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		margin-top: 0.625rem;
		font-size: 0.8125rem;
		color: var(--color-text-ghost);
		opacity: 0;
		animation: fade-in 400ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
		animation-delay: 700ms;
	}

	.availability-badge.open {
		color: var(--raw-teal-light);
	}

	.badge-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--raw-teal);
		flex-shrink: 0;
	}

	.availability-badge.open .badge-dot {
		background: var(--raw-teal-light);
		box-shadow: 0 0 6px var(--raw-teal);
	}

	/* ============================================================
	   Content column
	   ============================================================ */
	.content {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.hero-pitch {
		opacity: 0;
		animation: fade-in 500ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
		animation-delay: 800ms;
	}

	.hero-pitch p {
		font-size: 0.9375rem;
		line-height: 1.75;
		color: var(--color-text);
	}

	.hero-pitch p + p {
		margin-top: 0.75rem;
	}

	.hero-meta {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem 0.875rem;
		font-size: 0.8125rem;
		color: var(--color-text-ghost);
		opacity: 0;
		animation: fade-in 400ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
		animation-delay: 1000ms;
	}

	/* ============================================================
	   Cards
	   ============================================================ */
	.card {
		border-radius: var(--radius-md);
		padding: 1rem 1.25rem;
		min-height: 44px;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		transition: border-color 0.2s ease, background 0.2s ease;
		cursor: pointer;
		text-decoration: none;
		color: var(--color-text);
	}

	.card-label {
		font-family: var(--font-heading);
		font-size: 0.8125rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.card-desc {
		font-size: 0.8125rem;
		color: var(--color-text-muted);
		line-height: 1.4;
	}

	.card-icon {
		font-size: 1.25rem;
		margin-bottom: 0.25rem;
	}

	/* Ask AI card — filled brass, primary CTA */
	.card-ai {
		background: linear-gradient(135deg, var(--raw-brass-dark) 0%, var(--raw-brass) 100%);
		border: 1px solid var(--raw-brass);
		color: var(--raw-bg-base);
		width: 100%;
		text-align: left;
		font-family: inherit;
		opacity: 0;
		animation: fade-in 400ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
		animation-delay: 1100ms;
	}

	.card-ai .card-label {
		color: var(--raw-bg-base);
	}

	.card-ai .card-desc {
		color: rgba(26, 22, 18, 0.7);
	}

	.card-ai:hover {
		background: linear-gradient(135deg, var(--raw-brass) 0%, var(--raw-brass-light) 100%);
		border-color: var(--raw-brass-light);
	}

	/* Navigation cards — brass outline, secondary */
	.card-nav {
		border: 1px solid rgba(176, 141, 87, 0.40);
		background: transparent;
	}

	.card-nav:hover {
		border-color: var(--raw-brass-light);
		background: rgba(176, 141, 87, 0.08);
		text-decoration: none;
	}

	.card-nav .card-label {
		color: var(--color-accent-light);
	}

	.nav-cards {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		opacity: 0;
		animation: fade-in 400ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
		animation-delay: 1200ms;
	}

	/* Contact card — full-width, sits below nav row as its own zone */
	.card-contact {
		cursor: default;
		width: 100%;
		opacity: 0;
		animation: fade-in 400ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
		animation-delay: 1300ms;
	}

	.contact-links {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-top: 0.25rem;
	}

	.contact-link {
		font-family: var(--font-heading);
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		padding: 0.375rem 0.75rem;
		border: 1px solid rgba(176, 141, 87, 0.30);
		border-radius: var(--radius-btn);
		color: var(--color-text-muted);
		min-height: 44px;
		display: inline-flex;
		align-items: center;
		transition: color 0.2s ease, border-color 0.2s ease;
	}

	.contact-link:hover {
		color: var(--color-accent-light);
		border-color: var(--color-accent);
		text-decoration: none;
	}

	/* ============================================================
	   Shared animations
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
	   Reduced motion
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
		.availability-badge,
		.card-ai,
		.nav-cards,
		.card-contact {
			opacity: 1;
			transform: none;
			animation: none;
		}
	}

	/* ============================================================
	   Tablet — 768px+: two-column layout
	   ============================================================ */
	@media (min-width: 768px) {
		.hub {
			flex-direction: row;
			align-items: flex-start;
			gap: 3rem;
		}

		.identity {
			flex: 0 0 38%;
			max-width: 360px;
			position: sticky;
			top: 3rem;
		}

		.content {
			flex: 1;
			min-width: 0;
		}

		/* Identity column is ~38% of viewport, capped at max-width 360px.
		   "Peter Manahan" in Cinzel at 2.5rem + 0.06em tracking fits ~320px.
		   3.8vw at 768px = 29px ≈ 1.8rem; upper bound capped at 2.5rem to
		   prevent overflow in the fixed-width identity column. */
		.hero-name {
			font-size: clamp(1.75rem, 3.8vw, 2.5rem);
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
		}

		.nav-cards {
			flex-direction: row;
		}

		.nav-cards .card-nav {
			flex: 1;
		}
	}

	/* ============================================================
	   Desktop — 1024px+
	   ============================================================ */
	@media (min-width: 1024px) {
		.hero {
			padding-top: 4rem;
			padding-bottom: 6rem;
		}

		.hub {
			gap: 4rem;
		}

		/* At 1024px, identity column is still capped at 360px.
		   2.5rem is the safe upper bound for "Peter Manahan" in Cinzel. */
		.hero-name {
			font-size: clamp(2rem, 3vw, 2.5rem);
		}

		.hero-title {
			font-size: 1.25rem;
		}
	}

	/* Wide desktop — no additional overrides needed.
	   hero-name is capped at 2.5rem via clamp at 1024px+ and
	   identity column max-width: 360px, both are stable at all widths. */
</style>
