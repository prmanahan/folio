<script lang="ts">
	import ChatTab from './ChatTab.svelte';
	import FitTab from './FitTab.svelte';

	let { open = $bindable(false) } = $props();

	let activeTab = $state<'chat' | 'jobfit'>('chat');

	function close() {
		open = false;
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			close();
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			close();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="backdrop" role="presentation" onclick={handleBackdropClick}></div>
{/if}

<div class="ai-pane" class:open>
	<div class="pane-header">
		<span class="pane-title">Transmission Terminal</span>
		<div class="tabs">
			<button
				class="tab"
				class:active={activeTab === 'chat'}
				onclick={() => (activeTab = 'chat')}
			>
				Chat
			</button>
			<button
				class="tab"
				class:active={activeTab === 'jobfit'}
				onclick={() => (activeTab = 'jobfit')}
			>
				Job Fit
			</button>
		</div>
		<button class="close-btn" onclick={close} aria-label="Close AI pane">✕</button>
	</div>

	<div class="pane-content" class:no-padding={activeTab === 'chat'}>
		{#if activeTab === 'chat'}
			<ChatTab />
		{:else}
			<FitTab />
		{/if}
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		z-index: 199;
	}

	.ai-pane {
		position: fixed;
		top: 0;
		right: 0;
		height: 100vh;
		width: 400px;
		background: var(--color-bg-deep);
		border-left: 1px solid var(--raw-brass);
		z-index: 200;
		display: flex;
		flex-direction: column;
		transform: translateX(100%);
		transition: transform 0.3s ease;
		box-shadow: -4px 0 24px rgba(0, 0, 0, 0.5);
		font-family: var(--font-mono);
	}

	.ai-pane.open {
		transform: translateX(0);
	}

	.pane-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.625rem 1rem;
		border-bottom: 1px solid rgba(176, 141, 87, 0.25);
		background: rgba(176, 141, 87, 0.06);
		flex-shrink: 0;
	}

	.pane-title {
		font-family: var(--font-heading);
		font-size: 0.625rem;
		text-transform: uppercase;
		letter-spacing: 0.15em;
		color: var(--raw-brass);
	}

	.tabs {
		display: flex;
		gap: 0.25rem;
	}

	.tab {
		font-family: var(--font-heading);
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.10em;
		padding: 0.3rem 0.75rem;
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		background: none;
		color: var(--color-text-ghost);
		cursor: pointer;
		transition: color 0.2s ease, border-color 0.2s ease;
	}

	.tab:hover {
		color: var(--color-text-muted);
	}

	.tab.active {
		color: var(--color-accent-light);
		border-color: rgba(176, 141, 87, 0.30);
		background: rgba(176, 141, 87, 0.08);
	}

	.close-btn {
		background: none;
		border: none;
		color: var(--color-text-ghost);
		font-size: 1rem;
		cursor: pointer;
		padding: 0.25rem 0.5rem;
		border-radius: var(--radius-sm);
		line-height: 1;
		transition: color 0.15s;
		min-width: 44px;
		min-height: 44px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.close-btn:hover {
		color: var(--color-accent-light);
	}

	.pane-content {
		flex: 1;
		padding: 1.5rem 1rem;
		overflow-y: auto;
	}

	.pane-content.no-padding {
		padding: 0;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	@media (max-width: 767px) {
		.ai-pane {
			width: 100vw;
		}
	}
</style>
