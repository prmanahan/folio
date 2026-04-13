<script lang="ts">
	import type { Skill } from '$lib/types';
	import { onMount } from 'svelte';

	interface Props {
		skills: Skill[];
	}

	let { skills }: Props = $props();

	// Filter to strong + moderate only — banner shows what Peter knows, not gaps
	let bannerSkills = $derived(
		skills.filter((s) => s.category === 'strong' || s.category === 'moderate')
	);

	// Seamless loop: the track animates translateX(0) → translateX(-50%).
	// For the loop to be gapless, the first half of the track (= one logical copy)
	// must be wider than the viewport at all times. With sparse data (e.g. 2 skills),
	// a single copy is only ~200px — far too narrow for a 1440px+ viewport.
	//
	// Fix: repeat the skill set REPS times per half. The track contains 2×REPS copies
	// total. At -50%, the second half starts exactly where the first half started,
	// producing a seamless loop regardless of viewport width.
	//
	// REPS=10 guarantees coverage: even with 2 skills at ~100px each,
	// 10×2×100px = 2000px > any common viewport width.
	const REPS = 10;
	let trackItems = $derived(
		Array.from({ length: REPS }, () => bannerSkills).flat()
	);

	// Intersection Observer to stop/resume scrolling when off-screen.
	// Default isVisible = true so the animation starts immediately on mount —
	// the observer only pauses it after the banner has left the viewport.
	let bannerEl: HTMLElement | undefined = $state(undefined);
	let isVisible = $state(true);

	onMount(() => {
		if (!bannerEl) return;

		const observer = new IntersectionObserver(
			(entries) => {
				isVisible = entries[0].isIntersecting;
			},
			// rootMargin keeps the banner "visible" until it's fully off-screen,
			// preventing an immediate pause on first mount when the hero is tall.
			{ threshold: 0, rootMargin: '200px 0px 200px 0px' }
		);

		observer.observe(bannerEl);

		return () => observer.disconnect();
	});
</script>

<!--
  SkillsBanner — conveyor belt marquee at the bottom of the hero
  aria-hidden: SkillsSection below the fold handles AT users with a clean list.
  This is purely decorative/visual brand reinforcement.
-->
<div
	class="skills-banner"
	aria-hidden="true"
	bind:this={bannerEl}
>
	{#if bannerSkills.length > 0}
		<div
			class="banner-track"
			class:paused={!isVisible}
		>
			<!--
				Track contains 2×REPS copies of bannerSkills.
				Animation: translateX(0) → translateX(-50%).
				At -50%, the second REPS copies are in exactly the position
				the first REPS copies started — seamless loop with no gap.
			-->
			{#each [...trackItems, ...trackItems] as skill}
				<span class="banner-item">
					<span class="separator">◆</span>
					<span class="skill-name">{skill.skill_name}</span>
				</span>
			{/each}
		</div>
	{/if}
</div>

<style>
	.skills-banner {
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		height: 2.5rem;
		width: 100vw;
		/* Break out of container max-width — conveyor runs wall-to-wall */
		margin-left: calc(50% - 50vw);
		overflow: hidden;
		background: rgba(176, 141, 87, 0.10);
		border-top: 1px solid rgba(176, 141, 87, 0.25);
		z-index: 3;
	}

	.banner-track {
		display: flex;
		align-items: center;
		width: max-content;
		height: 100%;
		/* Uses translateX — no layout recalculation, GPU composited */
		animation: conveyor 60s linear infinite;
		will-change: transform;
	}

	/* Stop scrolling when banner is off-screen */
	.banner-track.paused {
		animation-play-state: paused;
	}

	/* Hover pause — desktop only, lets user read a skill name */
	@media (hover: hover) {
		.banner-track:hover {
			animation-play-state: paused;
		}
	}

	@keyframes conveyor {
		from { transform: translateX(0); }
		to   { transform: translateX(-50%); }
	}

	.banner-item {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0 0.625rem;
		white-space: nowrap;
	}

	.separator {
		color: var(--raw-brass);
		opacity: 0.5;
		font-size: 0.5rem;
		line-height: 1;
	}

	.skill-name {
		font-family: var(--font-heading);
		font-size: 0.625rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.14em;
		color: var(--color-text-ghost);
	}

	/* Tablet: slightly larger */
	@media (min-width: 768px) {
		.skills-banner {
			height: 2.75rem;
		}

		.banner-track {
			animation-duration: 45s;
		}

		.skill-name {
			font-size: 0.6875rem;
		}
	}

	/* Desktop: faster scroll */
	@media (min-width: 1024px) {
		.banner-track {
			animation-duration: 40s;
		}
	}

	/* Reduced motion: static, no scrolling, first set visible at left edge */
	@media (prefers-reduced-motion: reduce) {
		.banner-track {
			animation: none;
			will-change: auto;
		}
	}
</style>
