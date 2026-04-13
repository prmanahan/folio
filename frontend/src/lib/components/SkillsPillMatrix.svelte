<!--
  SkillsPillMatrix — accessible, scannable skills surface for the Resume destination.
  Renders three proficiency bands (Strong, Moderate, Learning) as compact pill chips.
  No counts, no percentages — category labels are the only proficiency signal.
-->
<script lang="ts">
	import type { Skill } from '$lib/types';

	interface Props {
		skills: Skill[];
	}

	let { skills }: Props = $props();

	const categoryOrder = ['strong', 'moderate', 'gap'] as const;

	const categoryLabels: Record<string, string> = {
		strong: 'Strong',
		moderate: 'Moderate',
		gap: 'Learning',
	};

	const grouped = $derived.by(() => {
		const groups: Record<string, Skill[]> = {};
		for (const skill of skills) {
			const cat = skill.category;
			if (!groups[cat]) groups[cat] = [];
			groups[cat].push(skill);
		}
		return groups;
	});
</script>

<div class="pill-matrix" data-testid="skills-pill-matrix">
	{#each categoryOrder as cat}
		{@const catSkills = grouped[cat]}
		{#if catSkills && catSkills.length > 0}
			<div class="band" data-testid="skill-band-{cat}">
				<h3 class="band-label {cat}">{categoryLabels[cat] ?? cat}</h3>
				<div class="pills" role="list" aria-label="{categoryLabels[cat] ?? cat} skills">
					{#each catSkills as skill}
						<span class="pill {cat}" role="listitem">{skill.skill_name}</span>
					{/each}
				</div>
			</div>
		{/if}
	{/each}
</div>

<style>
	.pill-matrix {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.band {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.band-label {
		font-family: var(--font-heading);
		font-size: 0.6875rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.14em;
		margin: 0;
	}

	.band-label.strong   { color: var(--color-strong); }
	.band-label.moderate { color: var(--color-moderate); }
	.band-label.gap      { color: var(--color-gap); }

	.pills {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
	}

	.pill {
		font-family: var(--font-heading);
		font-size: 0.6875rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		padding: 0.3125rem 0.625rem;
		border-radius: var(--radius-btn);
		border: 1px solid;
		white-space: nowrap;
	}

	/* Strong: brass border accent */
	.pill.strong {
		border-color: var(--color-strong);
		color: var(--color-strong);
		background: transparent;
	}

	/* Moderate: teal border */
	.pill.moderate {
		border-color: var(--color-moderate);
		color: var(--color-moderate);
		background: transparent;
	}

	/* Learning (gap): copper border */
	.pill.gap {
		border-color: var(--color-gap);
		color: var(--color-gap);
		background: transparent;
	}
</style>
