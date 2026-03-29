<script lang="ts">
	import { marked } from 'marked';
	import DOMPurify from 'dompurify';
	import type { ChatMessage } from '$lib/types';

	let { message }: { message: ChatMessage } = $props();

	let renderedContent = $derived(
		message.role === 'assistant'
			? DOMPurify.sanitize(marked(message.content) as string)
			: ''
	);
</script>

<div class="message" class:user={message.role === 'user'} class:assistant={message.role === 'assistant'}>
	{#if message.role === 'user'}
		<div class="bubble user-bubble">
			{message.content}
		</div>
	{:else}
		<div class="bubble assistant-bubble">
			{#if message.content}
				<!-- eslint-disable-next-line svelte/no-at-html-tags -->
				{@html renderedContent}
			{:else}
				<span class="typing-indicator">
					<span></span>
					<span></span>
					<span></span>
				</span>
			{/if}
		</div>
	{/if}
</div>

<style>
	.message {
		display: flex;
		margin-bottom: 0.625rem;
	}

	.message.user {
		justify-content: flex-end;
	}

	.message.assistant {
		justify-content: flex-start;
	}

	.bubble {
		max-width: 90%;
		padding: 0.75rem 1rem;
		font-size: 0.875rem;
		line-height: 1.5;
		word-break: break-word;
		font-family: var(--font-mono);
	}

	.user-bubble {
		background: rgba(176, 141, 87, 0.12);
		border-left: 3px solid var(--raw-brass);
		border-radius: 0;
		color: var(--color-text);
	}

	.assistant-bubble {
		background: rgba(26, 22, 18, 0.6);
		border-left: 3px solid var(--raw-teal);
		border-radius: 0;
		color: var(--color-text-muted);
	}

	.assistant-bubble :global(p) {
		margin-bottom: 0.5em;
	}

	.assistant-bubble :global(p:last-child) {
		margin-bottom: 0;
	}

	.assistant-bubble :global(ul),
	.assistant-bubble :global(ol) {
		padding-left: 1.25em;
		margin-bottom: 0.5em;
	}

	.assistant-bubble :global(li) {
		margin-bottom: 0.25em;
	}

	.assistant-bubble :global(code) {
		font-family: var(--font-mono);
		font-size: 0.8125rem;
		background: rgba(176, 141, 87, 0.10);
		padding: 0.1em 0.3em;
		border-radius: var(--radius-sm);
	}

	.assistant-bubble :global(pre) {
		background: rgba(176, 141, 87, 0.08);
		border: 1px solid var(--color-border-subtle);
		padding: 0.5em 0.75em;
		border-radius: var(--radius-sm);
		overflow-x: auto;
		margin-bottom: 0.5em;
	}

	.assistant-bubble :global(pre code) {
		background: none;
		padding: 0;
	}

	.assistant-bubble :global(strong) {
		font-weight: 600;
		color: var(--color-text);
	}

	.typing-indicator {
		display: inline-flex;
		gap: 6px;
		align-items: center;
		height: 1em;
		color: var(--raw-brass);
		letter-spacing: 0.3em;
	}

	.typing-indicator span {
		display: inline-block;
		width: 4px;
		height: 4px;
		border-radius: 50%;
		background: var(--raw-brass);
		animation: morse 1.5s step-end infinite;
	}

	.typing-indicator span:nth-child(2) {
		animation-delay: 0.3s;
	}

	.typing-indicator span:nth-child(3) {
		animation-delay: 0.6s;
	}

	@keyframes morse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.2; }
	}
</style>
