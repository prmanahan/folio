<script lang="ts">
	import { api } from '$lib/api';
	import type { FitVerdict } from '$lib/types';
	import FitVerdictComponent from './FitVerdict.svelte';

	let jobDescription = $state('');
	let verdict = $state<FitVerdict | null>(null);
	let loading = $state(false);
	let error = $state<string | null>(null);

	async function analyze() {
		if (!jobDescription.trim() || loading) return;
		loading = true;
		error = null;
		verdict = null;

		try {
			verdict = await api.fitAnalysis(jobDescription);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Something went wrong. Please try again.';
		} finally {
			loading = false;
		}
	}

	function reset() {
		verdict = null;
		error = null;
		jobDescription = '';
	}
</script>

<div class="fit-tab">
	{#if verdict}
		<div class="result-area">
			<FitVerdictComponent {verdict} />
		</div>
		<div class="action-bar">
			<button class="try-another-btn" onclick={reset}>
				Try Another
			</button>
		</div>
	{:else}
		<div class="input-area">
			<label for="jd-input" class="input-label">Paste a job description</label>
			<textarea
				id="jd-input"
				class="jd-textarea"
				placeholder="Paste the full job description here..."
				bind:value={jobDescription}
				disabled={loading}
				rows={10}
			></textarea>

			{#if error}
				<div class="error-message" role="alert">{error}</div>
			{/if}

			<button
				class="analyze-btn"
				onclick={analyze}
				disabled={!jobDescription.trim() || loading}
			>
				{#if loading}
					<span class="spinner"></span>
					Analyzing...
				{:else}
					Analyze Fit
				{/if}
			</button>
		</div>
	{/if}
</div>

<style>
	.fit-tab {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.input-area {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 0.25rem 0;
	}

	.input-label {
		font-family: var(--font-heading);
		font-size: 0.6875rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		color: var(--color-text-muted);
	}

	.jd-textarea {
		width: 100%;
		padding: 0.625rem 0.75rem;
		background: var(--color-bg-deep);
		border: 1px solid rgba(176, 141, 87, 0.30);
		border-radius: var(--radius-sm);
		color: var(--color-text);
		font-family: var(--font-mono);
		font-size: 0.8125rem;
		line-height: 1.6;
		resize: vertical;
		min-height: 180px;
		box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.4);
		transition: border-color 0.2s;
	}

	.jd-textarea:focus {
		outline: none;
		border-color: var(--raw-brass-light);
		box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.4), 0 0 0 2px rgba(176, 141, 87, 0.15);
	}

	.jd-textarea::placeholder {
		color: #6a5a4a;
	}

	.jd-textarea:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.analyze-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: linear-gradient(180deg, #c9a45e 0%, #9a7040 100%);
		border: 1px solid var(--raw-brass-light);
		border-radius: var(--radius-btn);
		color: var(--raw-bg-base);
		font-family: var(--font-heading);
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.10em;
		cursor: pointer;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.15) inset,
			0 3px 8px rgba(0, 0, 0, 0.5);
		transition: filter 0.2s ease;
		align-self: flex-start;
	}

	.analyze-btn:hover:not(:disabled) {
		filter: brightness(1.1);
	}

	.analyze-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.error-message {
		padding: 0.625rem 0.875rem;
		background: rgba(192, 84, 74, 0.15);
		border: 1px solid rgba(192, 84, 74, 0.4);
		border-radius: var(--radius-sm);
		color: var(--raw-copper-light);
		font-size: 0.8125rem;
	}

	.result-area {
		flex: 1;
		overflow-y: auto;
		padding-bottom: 0.75rem;
	}

	.action-bar {
		padding-top: 0.75rem;
		border-top: 1px solid var(--color-border);
		flex-shrink: 0;
	}

	.try-another-btn {
		font-family: var(--font-heading);
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.10em;
		padding: 0.5rem 1rem;
		border: 1px solid rgba(176, 141, 87, 0.40);
		border-radius: var(--radius-btn);
		background: transparent;
		color: var(--color-text-muted);
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.try-another-btn:hover {
		color: var(--color-accent-light);
		border-color: var(--color-accent);
		background: rgba(176, 141, 87, 0.08);
	}

	.spinner {
		display: inline-block;
		width: 14px;
		height: 14px;
		border: 2px solid rgba(26, 22, 18, 0.4);
		border-top-color: var(--raw-bg-base);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
		flex-shrink: 0;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}
</style>
