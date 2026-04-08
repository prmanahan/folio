<script lang="ts">
	import type { Skill } from '$lib/types';

	let { skills }: { skills: Skill[] } = $props();

	const grouped = $derived.by(() => {
		const groups: Record<string, Skill[]> = {};
		for (const skill of skills) {
			const cat = skill.category;
			if (!groups[cat]) groups[cat] = [];
			groups[cat].push(skill);
		}
		return groups;
	});

	const categoryOrder = ['strong', 'moderate', 'gap'];
	const categoryLabels: Record<string, string> = {
		strong: 'Strong',
		moderate: 'Moderate',
		gap: 'Learning',
	};
</script>

<section class="skills">
	<div class="container">
		<h2><span class="gear-accent" aria-hidden="true"></span>Skills</h2>
		{#each categoryOrder as cat}
			{@const catSkills = grouped[cat]}
			{#if catSkills && catSkills.length > 0}
				<div class="category">
					<h3 class="category-label {cat}">{categoryLabels[cat] ?? cat}</h3>
					<div class="skill-grid">
						{#each catSkills as skill}
							<div class="skill-item">
								<span class="skill-name">{skill.skill_name}</span>
								<span class="skill-meta">
									{skill.years_experience}y
									{#if skill.last_used}· {skill.last_used}{/if}
								</span>
							</div>
						{/each}
					</div>
				</div>
			{/if}
		{/each}
	</div>
</section>

<style>
	.skills {
		padding: 3rem 0;
		border-top: 1px solid var(--color-border);
	}

	h2 {
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 600;
		letter-spacing: 0.06em;
		color: var(--color-text);
		margin-bottom: 1.75rem;
		display: flex;
		align-items: center;
	}

	.gear-accent {
		display: inline-block;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: conic-gradient(
			var(--raw-brass) 0deg 30deg, transparent 30deg 60deg,
			var(--raw-brass) 60deg 90deg, transparent 90deg 120deg,
			var(--raw-brass) 120deg 150deg, transparent 150deg 180deg,
			var(--raw-brass) 180deg 210deg, transparent 210deg 240deg,
			var(--raw-brass) 240deg 270deg, transparent 270deg 300deg,
			var(--raw-brass) 300deg 330deg, transparent 330deg 360deg
		);
		opacity: 0.6;
		margin-right: 0.5rem;
		flex-shrink: 0;
	}

	.category {
		margin-bottom: 1.75rem;
	}

	.category-label {
		font-family: var(--font-heading);
		font-size: 0.75rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		margin-bottom: 0.875rem;
	}

	.category-label.strong   { color: var(--color-strong); }
	.category-label.moderate { color: var(--color-moderate); }
	.category-label.gap      { color: var(--color-gap); }

	.skill-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 0.5rem;
	}

	.skill-item {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		padding: 0.5rem 0.875rem;
		background: var(--color-surface);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		font-size: 0.875rem;
		transition: border-color 0.2s ease, background 0.2s ease;
	}

	.skill-item:hover {
		border-color: var(--color-border);
		background: var(--color-surface-high);
	}

	.skill-name {
		font-family: var(--font-body);
		font-weight: 500;
		color: var(--color-text);
		min-width: 0;
	}

	.skill-meta {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--color-text-ghost);
		flex-shrink: 0;
	}
</style>
