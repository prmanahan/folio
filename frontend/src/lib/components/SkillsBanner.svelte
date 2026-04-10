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

	// Intersection Observer to stop/resume scrolling when off-screen
	let bannerEl: HTMLElement | undefined = $state(undefined);
	let isVisible = $state(true);

	onMount(() => {
		if (!bannerEl) return;

		const observer = new IntersectionObserver(
			(entries) => {
				isVisible = entries[0].isIntersecting;
			},
			{ threshold: 0 }
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
			<!-- Duplicate the list for seamless infinite loop -->
			{#each [...bannerSkills, ...bannerSkills] as skill}
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
