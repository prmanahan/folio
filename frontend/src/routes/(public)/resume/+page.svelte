<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { Skill, Experience, Education } from '$lib/types';
	import SkillsPillMatrix from '$lib/components/SkillsPillMatrix.svelte';
	import ExperienceSection from '$lib/components/ExperienceSection.svelte';
	import EducationSection from '$lib/components/EducationSection.svelte';

	let skills: Skill[] = $state([]);
	let experiences: Experience[] = $state([]);
	let education: Education[] = $state([]);
	let error: string | null = $state(null);
	let loading = $state(true);

	onMount(async () => {
		try {
			[skills, experiences, education] = await Promise.all([
				api.getSkills(),
				api.getExperience(),
				api.getEducation(),
			]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load';
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>Resume — Peter Manahan</title>
</svelte:head>

<div class="resume-page">
	<div class="container page">
		<h1 class="page-heading">Resume</h1>

		{#if error}
			<p class="error">{error}</p>
		{:else if loading}
			<div class="loading" role="status" aria-busy="true" aria-label="Loading resume">
				<p class="loading-text">Loading…</p>
			</div>
		{:else}
			<!--
				Section order per amendment: Skills → Experience → Education.
				Each section has an `id` anchor for deep linking (#skills, #experience, #education).
				ExperienceSection and EducationSection render their own <section> + <h2> internally —
				the .section-anchor wrapper provides the ID target without creating nested sections.
			-->

			<!-- Section 1: Skills -->
			<div class="section-anchor" id="skills" data-testid="resume-section-skills">
				<section aria-labelledby="skills-heading">
					<h2 id="skills-heading" class="section-heading">Skills</h2>
					{#if skills.length > 0}
						<SkillsPillMatrix {skills} />
					{:else}
						<p class="empty">No skills listed.</p>
					{/if}
				</section>
			</div>

			<!-- Section 2: Experience -->
			<div class="section-anchor" id="experience" data-testid="resume-section-experience">
				<ExperienceSection experiences={experiences} />
			</div>

			<!-- Section 3: Education -->
			<div class="section-anchor" id="education" data-testid="resume-section-education">
				<EducationSection education={education} />
			</div>
		{/if}
	</div>
</div>

<style>
	.resume-page {
		padding-bottom: 4rem;
	}

	.page {
		padding-top: 2.5rem;
	}

	.page-heading {
		font-family: var(--font-heading);
		font-size: clamp(1.75rem, 4vw, 2.25rem);
		font-weight: 600;
		letter-spacing: 0.06em;
		color: var(--color-gold);
		margin-bottom: 2.5rem;
	}

	.section-anchor {
		/* Offset scroll-to-anchor by header height so content isn't hidden */
		scroll-margin-top: calc(var(--nav-height) + 1rem);
	}

	.section-heading {
		font-family: var(--font-heading);
		font-size: 1.125rem;
		font-weight: 600;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--color-text-ghost);
		margin-bottom: 1.25rem;
		padding-bottom: 0.5rem;
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.loading {
		padding: 3rem 0;
	}

	.loading-text {
		color: var(--color-text-ghost);
		font-size: 0.875rem;
	}

	.error {
		color: var(--color-gap);
		font-size: 0.9375rem;
		padding: 2rem 0;
	}

	.empty {
		color: var(--color-text-ghost);
		font-size: 0.875rem;
		padding: 1rem 0;
	}
</style>
