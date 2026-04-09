<script lang="ts">
	import type { Education } from '$lib/types';

	let { education }: { education: Education[] } = $props();
</script>

<section class="education">
	<div class="container">
		<h2>Education</h2>
		{#each education as edu}
			<div class="edu-item">
				<div class="edu-header">
					<h3>{edu.degree}</h3>
					<span class="years">{edu.start_year} — {edu.end_year}</span>
				</div>
				<p class="institution">{edu.institution}{#if edu.location} · {edu.location}{/if}</p>
			</div>
		{/each}
	</div>
</section>

<style>
	.education {
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

	.edu-item {
		padding: 1rem 0;
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.edu-item:last-child {
		border-bottom: none;
	}

	.edu-header {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
	}

	h3 {
		font-family: var(--font-heading);
		font-size: 1.05rem;
		font-weight: 400;
		letter-spacing: 0.04em;
		color: var(--color-text);
	}

	.years {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--color-text-ghost);
	}

	.institution {
		color: var(--color-text-muted);
		font-size: 0.9rem;
		margin-top: 0.15rem;
	}

	@media (max-width: 767px) {
		.edu-header { flex-direction: column; gap: 0.125rem; }
	}
</style>
