<script lang="ts">
	import type { Experience } from '$lib/types';

	let { experiences }: { experiences: Experience[] } = $props();
</script>

<section class="experience">
	<div class="container">
		<h2>Experience</h2>
		<div class="timeline">
			{#each experiences as exp}
				<div class="role">
					<div class="role-header">
						<div>
							<h3>{exp.title}</h3>
							<p class="company">{exp.company_name}{#if exp.location} · {exp.location}{/if}</p>
						</div>
						<span class="dates">
							{exp.start_date}{#if exp.is_current || exp.end_date} — {exp.is_current ? 'Present' : exp.end_date}{/if}
						</span>
					</div>
					{#if exp.summary}
						<p class="summary">{exp.summary}</p>
					{/if}
					{#if exp.bullet_points && exp.bullet_points.length > 0}
						<ul>
							{#each exp.bullet_points as point}
								<li>{point}</li>
							{/each}
						</ul>
					{/if}
				</div>
			{/each}
		</div>
	</div>
</section>

<style>
	.experience {
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
	}

	.role {
		padding: 0 0 2rem 1.25rem;
		border-left: 1px solid rgba(176, 141, 87, 0.30);
		border-bottom: none;
		position: relative;
	}

	.role:last-child {
		padding-bottom: 0;
	}

	.role::before {
		content: '';
		position: absolute;
		left: -4px;
		top: 0.375rem;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--raw-brass);
		border: 2px solid var(--color-bg);
	}

	.role:first-child::before {
		background: var(--raw-brass-light);
		box-shadow: 0 0 8px var(--raw-brass);
	}

	.role-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 1rem;
	}

	h3 {
		font-family: var(--font-heading);
		font-size: 1.125rem;
		font-weight: 400;
		letter-spacing: 0.04em;
		color: var(--color-text);
	}

	.company {
		color: var(--color-text-muted);
		font-size: 0.9rem;
		margin-top: 0.15rem;
	}

	.dates {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--color-text-ghost);
		white-space: nowrap;
	}

	.summary {
		margin-top: 0.75rem;
		font-size: 0.9375rem;
		color: var(--color-text-muted);
	}

	ul {
		list-style: none;
		margin-top: 0.5rem;
		padding-left: 0;
	}

	li {
		padding: 0.25rem 0 0.25rem 1rem;
		position: relative;
		font-size: 0.9375rem;
		color: var(--color-text);
		margin-top: 0.25rem;
		line-height: 1.5;
	}

	li::before {
		content: '\2014';
		position: absolute;
		left: 0;
		color: var(--raw-brass);
		opacity: 0.5;
	}

	@media (max-width: 767px) {
		.role-header {
			flex-direction: column;
			gap: 0.125rem;
		}
	}
</style>
