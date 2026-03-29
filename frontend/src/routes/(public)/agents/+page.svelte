<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import type { Agent } from '$lib/types';

	let agents = $state<Agent[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	let featured = $derived(agents.filter(a => a.is_featured));
	let reviewGates = $derived(agents.filter(a => a.is_review_gate));
	let specialists = $derived(agents.filter(a => !a.is_featured && !a.is_review_gate));

	onMount(async () => {
		try {
			agents = await api.getAgents();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load agents';
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>The Agent Team — Peter Manahan</title>
</svelte:head>

{#if loading}
	<div class="container loading-state">
		<div class="loading-spinner"></div>
	</div>
{:else if error}
	<div class="container error-state">
		<p class="error-msg">{error}</p>
	</div>
{:else}
	<div class="container">
		<div class="page-header">
			<p class="header-eyebrow">AI Engineering</p>
			<h1>The Agent Team</h1>
			<p>
				This site is built and maintained by a team of AI agents I designed and orchestrate.
				Each agent has a defined role, a working style, and a model assignment that reflects
				the cost and quality tradeoff for that kind of work.
				Puck runs the team. Specialists execute. Warden reviews everything before it ships.
			</p>
		</div>

		<div class="brass-rule"></div>

		{#if agents.length === 0}
			<p class="empty">No agents published yet.</p>
		{/if}

		{#each featured as agent (agent.id)}
			<div class="orchestrator-band">
				<div class="orchestrator-avatar">
					{#if agent.avatar_filename}
						<img src="/api/avatars/{agent.avatar_filename}" alt={agent.name} />
					{:else}
						<div class="avatar-placeholder">{agent.name[0]}</div>
					{/if}
					<div class="orchestrator-popout">
						{#if agent.avatar_filename}
							<img src="/api/avatars/{agent.avatar_filename}" alt="{agent.name} — full portrait" />
						{/if}
					</div>
				</div>
				<div class="orchestrator-body">
					<div class="orchestrator-header">
						<span class="orchestrator-name">{agent.name}</span>
						<span class="orchestrator-role">{agent.short_role}</span>
						<span class="model-badge {agent.model.includes('opus') ? 'model-opus' : 'model-sonnet'}">
							{agent.model.includes('opus') ? 'Opus' : 'Sonnet'}
						</span>
					</div>
					<p class="orchestrator-blurb">{agent.personality_blurb}</p>
					<div class="orchestrator-tasks">
						{#each agent.responsibilities as task}
							<span class="task-chip">{task}</span>
						{/each}
					</div>
				</div>
			</div>
		{/each}

		{#if reviewGates.length > 0}
			<div class="classification">
				<span class="class-label">Review gate</span>
				<div class="class-rule"></div>
			</div>

			<div class="dossier">
				{#each reviewGates as agent (agent.id)}
					<div class="dossier-card {agent.model.includes('opus') ? 'tier-opus' : ''}">
						<div class="portrait-col">
							<div class="portrait">
								{#if agent.avatar_filename}
									<img src="/api/avatars/{agent.avatar_filename}" alt={agent.name} />
								{:else}
									<div class="avatar-placeholder">{agent.name[0]}</div>
								{/if}
							</div>
							<div class="portrait-popout">
								{#if agent.avatar_filename}
									<img src="/api/avatars/{agent.avatar_filename}" alt="{agent.name} — full portrait" />
								{/if}
							</div>
							<div class="portrait-name">{agent.name}</div>
							<div class="portrait-id">{agent.short_role}</div>
						</div>
						<div class="content-col">
							<div class="content-top">
								<div class="content-meta">
									<div class="agent-role">{agent.role}</div>
									<div class="agent-name">{agent.name}</div>
								</div>
								<span class="model-badge {agent.model.includes('opus') ? 'model-opus' : 'model-sonnet'}">
									{agent.model.includes('opus') ? 'Opus' : 'Sonnet'}
								</span>
							</div>
							<p class="agent-blurb">{agent.personality_blurb}</p>
							<ul class="responsibilities">
								{#each agent.responsibilities as resp}
									<li>{resp}</li>
								{/each}
							</ul>
						</div>
					</div>
				{/each}
			</div>
		{/if}

		{#if specialists.length > 0}
			<div class="dossier-section-break">
				<span class="dsb-label">Specialists</span>
				<div class="dsb-line"></div>
				<span class="dsb-count">{specialists.length} active</span>
			</div>

			<div class="dossier">
				{#each specialists as agent (agent.id)}
					<div class="dossier-card {agent.model.includes('opus') ? 'tier-opus' : ''}">
						<div class="portrait-col">
							<div class="portrait">
								{#if agent.avatar_filename}
									<img src="/api/avatars/{agent.avatar_filename}" alt={agent.name} />
								{:else}
									<div class="avatar-placeholder">{agent.name[0]}</div>
								{/if}
							</div>
							<div class="portrait-popout">
								{#if agent.avatar_filename}
									<img src="/api/avatars/{agent.avatar_filename}" alt="{agent.name} — full portrait" />
								{/if}
							</div>
							<div class="portrait-name">{agent.name}</div>
							<div class="portrait-id">{agent.short_role}</div>
						</div>
						<div class="content-col">
							<div class="content-top">
								<div class="content-meta">
									<div class="agent-role">{agent.role}</div>
									<div class="agent-name">{agent.name}</div>
								</div>
								<span class="model-badge {agent.model.includes('opus') ? 'model-opus' : 'model-sonnet'}">
									{agent.model.includes('opus') ? 'Opus' : 'Sonnet'}
								</span>
							</div>
							<p class="agent-blurb">{agent.personality_blurb}</p>
							<ul class="responsibilities">
								{#each agent.responsibilities as resp}
									<li>{resp}</li>
								{/each}
							</ul>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/if}

<style>
	/* ── Empty state ── */
	.empty {
		color: var(--color-text-muted);
		font-size: 0.9375rem;
		padding: 1rem 0 3rem;
	}

	/* ── Avatar placeholder ── */
	.avatar-placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(176, 141, 87, 0.1);
		border-radius: 50%;
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 400;
		letter-spacing: 0.04em;
		color: var(--color-gold);
		text-transform: uppercase;
	}

	/* ── Loading / Error states ── */
	.loading-state {
		display: flex;
		justify-content: center;
		padding: 5rem 0;
	}
	.loading-spinner {
		width: 2rem;
		height: 2rem;
		border: 2px solid var(--color-border-subtle);
		border-top-color: var(--color-accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin {
		to { transform: rotate(360deg); }
	}
	.error-state {
		padding: 5rem 0;
		text-align: center;
	}
	.error-msg {
		color: var(--color-text-muted);
		font-size: 0.9375rem;
	}

	/* ── Page Header ── */
	.page-header {
		padding: 3.5rem 0 0;
	}
	.header-eyebrow {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.14em;
		color: var(--color-accent);
		margin-bottom: 0.625rem;
	}
	.page-header h1 {
		font-family: var(--font-heading);
		font-size: clamp(1.75rem, 4vw, 2.5rem);
		font-weight: 400;
		letter-spacing: 0.06em;
		color: var(--color-text);
		line-height: 1.1;
		margin-bottom: 1.25rem;
	}
	.page-header p {
		max-width: 640px;
		font-size: 0.9375rem;
		color: var(--color-text-muted);
		line-height: 1.75;
		margin-bottom: 2.5rem;
	}

	/* ── Brass rule ── */
	.brass-rule {
		height: 1px;
		background: linear-gradient(90deg, transparent 0%, rgba(176,141,87,0.5) 20%, rgba(232,201,122,0.7) 50%, rgba(176,141,87,0.5) 80%, transparent 100%);
		margin-bottom: 2.5rem;
	}

	/* ── Orchestrator hero band ── */
	.orchestrator-band {
		display: flex;
		gap: 1.5rem;
		padding: 1.5rem;
		background: var(--color-surface);
		border: 1px solid rgba(232, 201, 122, 0.3);
		border-left: 3px solid var(--color-gold);
		border-radius: var(--radius-md);
		margin-bottom: 2.5rem;
		box-shadow: 0 4px 16px rgba(0,0,0,0.5);
		align-items: flex-start;
	}
	.orchestrator-avatar {
		width: 88px;
		height: 88px;
		flex-shrink: 0;
		position: relative;
		cursor: pointer;
	}
	.orchestrator-avatar > img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		border-radius: 50%;
		border: 2px solid rgba(232, 201, 122, 0.4);
		background: var(--color-bg);
	}
	.orchestrator-popout {
		position: absolute;
		left: calc(100% + 12px);
		top: 50%;
		transform: translateY(-50%) scale(0.95);
		width: 280px;
		height: 280px;
		border-radius: var(--radius-md);
		border: 1px solid rgba(232, 201, 122, 0.3);
		box-shadow: 0 8px 32px rgba(0,0,0,0.7), 0 0 0 1px rgba(232,201,122,0.1);
		overflow: hidden;
		opacity: 0;
		pointer-events: none;
		transition: opacity 0.2s ease, transform 0.2s ease;
		z-index: 50;
		background: var(--color-bg);
	}
	.orchestrator-popout img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.orchestrator-avatar:hover .orchestrator-popout {
		opacity: 1;
		pointer-events: auto;
		transform: translateY(-50%) scale(1);
	}
	.orchestrator-body { flex: 1; min-width: 0; }
	.orchestrator-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.625rem;
		flex-wrap: wrap;
	}
	.orchestrator-name {
		font-family: var(--font-heading);
		font-size: 1.25rem;
		font-weight: 400;
		letter-spacing: 0.04em;
		color: var(--color-text);
	}
	.orchestrator-role {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--color-gold);
	}
	.orchestrator-blurb {
		font-size: 0.9rem;
		color: var(--color-text-muted);
		line-height: 1.6;
		font-style: italic;
		margin-bottom: 0.875rem;
	}
	.orchestrator-tasks {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
	}
	.task-chip {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		padding: 0.25rem 0.625rem;
		background: rgba(176, 141, 87, 0.1);
		border: 1px solid rgba(176, 141, 87, 0.25);
		border-radius: var(--radius-sm);
		color: var(--color-text-ghost);
	}

	/* ── Model badges ── */
	.model-badge {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		font-weight: 500;
		padding: 0.25rem 0.625rem;
		border-radius: var(--radius-sm);
		letter-spacing: 0.04em;
		white-space: nowrap;
		align-self: flex-start;
	}
	.model-opus {
		background: rgba(176, 141, 87, 0.14);
		border: 1px solid rgba(176, 141, 87, 0.4);
		color: var(--color-gold);
	}
	.model-sonnet {
		background: rgba(77, 138, 138, 0.12);
		border: 1px solid rgba(77, 138, 138, 0.35);
		color: var(--raw-teal-light);
	}

	/* ── Section dividers ── */
	.classification {
		display: flex;
		align-items: center;
		gap: 1rem;
		margin-bottom: 2rem;
	}
	.class-label {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.14em;
		color: var(--color-text-ghost);
		white-space: nowrap;
	}
	.class-rule {
		flex: 1;
		height: 1px;
		background: var(--color-border-subtle);
	}
	.dossier-section-break {
		display: flex;
		align-items: center;
		gap: 1rem;
		margin-top: 0.5rem;
		margin-bottom: 1.5rem;
	}
	.dsb-label {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		color: var(--color-text-ghost);
		white-space: nowrap;
	}
	.dsb-line {
		flex: 1;
		height: 1px;
		background: var(--color-border-subtle);
	}
	.dsb-count {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		color: var(--color-text-ghost);
	}

	/* ── Dossier cards ── */
	.dossier {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		padding-bottom: 5rem;
	}
	.dossier-card {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		overflow: hidden;
		display: grid;
		grid-template-columns: 120px 1fr;
		box-shadow: 0 4px 16px rgba(0,0,0,0.5);
		position: relative;
	}
	.dossier-card::before {
		content: '';
		position: absolute;
		left: 0;
		top: 0;
		bottom: 0;
		width: 2px;
		background: var(--color-accent);
	}
	.dossier-card.tier-opus::before {
		background: var(--color-gold);
	}

	/* ── Portrait column ── */
	.portrait-col {
		background: var(--color-surface-high);
		border-right: 1px solid var(--color-border-subtle);
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 1.25rem 0.75rem;
		gap: 0.625rem;
		position: relative;
	}
	.portrait {
		width: 68px;
		height: 68px;
		border-radius: 50%;
		background: var(--color-bg);
		border: 2px solid var(--color-border);
		overflow: hidden;
		position: relative;
		cursor: pointer;
	}
	.portrait img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.dossier-card.tier-opus .portrait {
		border-color: rgba(232, 201, 122, 0.4);
	}

	/* ── Portrait hover popout ── */
	.portrait-popout {
		position: absolute;
		left: calc(100% + 12px);
		top: 50%;
		transform: translateY(-50%) scale(0.95);
		width: 240px;
		height: 240px;
		border-radius: var(--radius-md);
		border: 1px solid var(--color-border);
		box-shadow: 0 8px 32px rgba(0,0,0,0.7), 0 0 0 1px rgba(176,141,87,0.15);
		overflow: hidden;
		opacity: 0;
		pointer-events: none;
		transition: opacity 0.2s ease, transform 0.2s ease;
		z-index: 50;
		background: var(--color-bg);
	}
	.portrait-popout img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.portrait:hover + .portrait-popout,
	.portrait-popout:hover {
		opacity: 1;
		pointer-events: auto;
		transform: translateY(-50%) scale(1);
	}

	.portrait-name {
		font-family: var(--font-heading);
		font-size: 0.75rem;
		font-weight: 400;
		letter-spacing: 0.06em;
		color: var(--color-text);
		text-align: center;
		line-height: 1.2;
	}
	.portrait-id {
		font-family: var(--font-mono);
		font-size: 0.5625rem;
		color: var(--color-text-ghost);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		text-align: center;
	}

	/* ── Content column ── */
	.content-col {
		padding: 1.25rem 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.content-top {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 0.75rem;
		flex-wrap: wrap;
	}
	.agent-role {
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--color-text-ghost);
		margin-bottom: 0.25rem;
	}
	.agent-name {
		font-family: var(--font-heading);
		font-size: 1.125rem;
		font-weight: 400;
		letter-spacing: 0.04em;
		color: var(--color-text);
		line-height: 1.1;
	}
	.agent-blurb {
		font-size: 0.875rem;
		color: var(--color-text-muted);
		line-height: 1.6;
		font-style: italic;
		border-left: 2px solid var(--color-border-subtle);
		padding-left: 0.875rem;
	}
	.responsibilities {
		list-style: none;
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.25rem 1rem;
		padding: 0;
		margin: 0;
	}
	.responsibilities li {
		font-size: 0.8125rem;
		color: var(--color-text-ghost);
		padding-left: 1rem;
		position: relative;
		line-height: 1.4;
	}
	.responsibilities li::before {
		content: '\25B8';
		position: absolute;
		left: 0;
		color: var(--color-accent);
		font-size: 0.625rem;
		top: 0.175em;
	}

	/* ── Responsive ── */
	@media (max-width: 600px) {
		.responsibilities { grid-template-columns: 1fr; }
		.dossier-card { grid-template-columns: 1fr; }
		.portrait-col {
			flex-direction: row;
			justify-content: flex-start;
			padding: 0.875rem 1rem;
			border-right: none;
			border-bottom: 1px solid var(--color-border-subtle);
		}
		.portrait-popout { display: none; }
		.orchestrator-band { flex-direction: column; align-items: center; text-align: center; }
		.orchestrator-header { justify-content: center; }
		.orchestrator-tasks { justify-content: center; }
		.orchestrator-popout { display: none; }
	}
</style>
