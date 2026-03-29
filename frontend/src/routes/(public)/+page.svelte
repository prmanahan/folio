<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { Profile, Link, Skill, Experience, Education } from '$lib/types';
	import Hero from '$lib/components/Hero.svelte';
	import SkillsSection from '$lib/components/SkillsSection.svelte';
	import ExperienceSection from '$lib/components/ExperienceSection.svelte';
	import EducationSection from '$lib/components/EducationSection.svelte';

	let profile: Profile | null = $state(null);
	let links: Link[] = $state([]);
	let skills: Skill[] = $state([]);
	let experiences: Experience[] = $state([]);
	let education: Education[] = $state([]);
	let error: string | null = $state(null);

	onMount(async () => {
		try {
			[profile, links, skills, experiences, education] = await Promise.all([
				api.getProfile(),
				api.getLinks(),
				api.getSkills(),
				api.getExperience(),
				api.getEducation(),
			]);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load';
		}
	});
</script>

{#if error}
	<div class="container">
		<p class="error">{error}</p>
	</div>
{:else if profile}
	<Hero {profile} {links} />
	{#if skills.length > 0}
		<SkillsSection {skills} />
	{/if}
	{#if experiences.length > 0}
		<ExperienceSection {experiences} />
	{/if}
	{#if education.length > 0}
		<EducationSection {education} />
	{/if}
{:else}
	<div class="container">
		<p class="loading">Loading...</p>
	</div>
{/if}

<style>
	.loading, .error {
		padding: 4rem 0;
		text-align: center;
		color: var(--color-text-muted);
	}

	.error {
		color: var(--color-gap);
	}
</style>
