<script lang="ts">
	import { api } from '$lib/api';
	import { parseSSEStream, SSEError } from '$lib/sse';
	import type { ChatMessage } from '$lib/types';
	import ChatMessageComponent from './ChatMessage.svelte';
	import SuggestedQuestions from './SuggestedQuestions.svelte';

	let messages = $state<ChatMessage[]>([]);
	let input = $state('');
	let isStreaming = $state(false);
	let error = $state<string | null>(null);
	let messagesEl = $state<HTMLDivElement | null>(null);

	$effect(() => {
		// Scroll to bottom whenever messages change
		if (messagesEl && messages.length > 0) {
			// Access messages to track the reactive dependency
			const _len = messages.length;
			const _last = messages[messages.length - 1]?.content;
			messagesEl.scrollTop = messagesEl.scrollHeight;
		}
	});

	async function sendMessage(text: string) {
		if (!text.trim() || isStreaming) return;
		input = '';
		messages = [...messages, { role: 'user', content: text }];
		messages = [...messages, { role: 'assistant', content: '' }];
		isStreaming = true;
		error = null;

		try {
			const response = await api.chat(text);

			if (response.status === 429) {
				error = 'Rate limit reached. Please try again later.';
				messages = messages.slice(0, -1);
				isStreaming = false;
				return;
			}
			if (response.status === 500) {
				error = 'AI features are currently unavailable.';
				messages = messages.slice(0, -1);
				isStreaming = false;
				return;
			}
			if (!response.ok) {
				error = 'Something went wrong. Please try again.';
				messages = messages.slice(0, -1);
				isStreaming = false;
				return;
			}

			const reader = response.body!.getReader();

			for await (const token of parseSSEStream(reader)) {
				const lastMsg = messages[messages.length - 1];
				messages = [...messages.slice(0, -1), { ...lastMsg, content: lastMsg.content + token }];
			}

			// If stream completed but no content was received, show error
			if (messages[messages.length - 1]?.content === '') {
				error = 'AI response was empty. Please try again.';
				messages = messages.slice(0, -1);
			}
		} catch (e) {
			if (e instanceof SSEError) {
				error = e.message;
			} else {
				error = 'Network error. Please check your connection.';
			}
			if (messages[messages.length - 1]?.content === '') {
				messages = messages.slice(0, -1);
			}
		} finally {
			isStreaming = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			sendMessage(input);
		}
	}

	function handleSuggestedQuestion(question: string) {
		sendMessage(question);
	}
</script>

<div class="chat-tab">
	<div class="messages" bind:this={messagesEl}>
		{#if messages.length === 0}
			<div class="empty-state">
				<p class="empty-heading">Ask me anything</p>
				<p class="empty-sub">I can answer questions about Peter's background, experience, and skills.</p>
				<SuggestedQuestions onSelect={handleSuggestedQuestion} />
			</div>
		{:else}
			{#each messages as message, i (i)}
				<ChatMessageComponent {message} />
			{/each}
		{/if}
	</div>

	{#if error}
		<div class="error-message" role="alert">
			{error}
		</div>
	{/if}

	<div class="input-area">
		<textarea
			class="input"
			placeholder="Type your transmission..."
			bind:value={input}
			onkeydown={handleKeydown}
			disabled={isStreaming}
			rows={1}
		></textarea>
		<button
			class="transmit-btn"
			onclick={() => sendMessage(input)}
			disabled={isStreaming || !input.trim()}
			aria-label="Transmit message"
		>
			{#if isStreaming}
				<span class="spinner"></span>
			{:else}
				Transmit
			{/if}
		</button>
	</div>
</div>

<style>
	.chat-tab {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow: hidden;
		font-family: var(--font-mono);
		font-size: 0.875rem;
	}

	.messages {
		flex: 1;
		overflow-y: auto;
		padding: 1rem;
		scroll-behavior: smooth;
	}

	.empty-state {
		padding: 1rem 0;
	}

	.empty-heading {
		font-family: var(--font-heading);
		font-size: 0.875rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.10em;
		color: var(--color-text-muted);
		margin-bottom: 0.25rem;
	}

	.empty-sub {
		font-family: var(--font-mono);
		font-size: 0.8125rem;
		color: var(--color-text-ghost);
		margin-bottom: 1rem;
	}

	.error-message {
		margin: 0 1rem 0.5rem;
		padding: 0.625rem 0.875rem;
		background: rgba(192, 84, 74, 0.15);
		border: 1px solid rgba(192, 84, 74, 0.4);
		border-radius: var(--radius-sm);
		color: var(--raw-copper-light);
		font-size: 0.8125rem;
		flex-shrink: 0;
	}

	.input-area {
		display: flex;
		align-items: flex-end;
		gap: 0.5rem;
		padding: 0.75rem;
		border-top: 1px solid rgba(176, 141, 87, 0.30);
		background: var(--color-bg);
		flex-shrink: 0;
	}

	.input {
		flex: 1;
		padding: 0.5rem 0.75rem;
		background: var(--color-bg-deep);
		border: 1px solid rgba(176, 141, 87, 0.30);
		border-radius: var(--radius-sm);
		color: var(--color-text);
		font-family: var(--font-mono);
		font-size: 0.875rem;
		line-height: 1.5;
		resize: none;
		max-height: 120px;
		overflow-y: auto;
		box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.4);
		transition: border-color 0.2s;
	}

	.input:focus {
		outline: none;
		border-color: var(--raw-brass-light);
		box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.4), 0 0 0 2px rgba(176, 141, 87, 0.15);
	}

	.input::placeholder {
		color: #6a5a4a;
	}

	.input:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.transmit-btn {
		font-family: var(--font-heading);
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.10em;
		padding: 0.5rem 1rem;
		background: linear-gradient(180deg, #c9a45e 0%, #9a7040 100%);
		border: 1px solid var(--raw-brass-light);
		border-radius: var(--radius-btn);
		color: var(--raw-bg-base);
		cursor: pointer;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.15) inset,
			0 3px 8px rgba(0, 0, 0, 0.5);
		transition: filter 0.2s ease;
		white-space: nowrap;
		min-height: 36px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.transmit-btn:hover:not(:disabled) {
		filter: brightness(1.1);
	}

	.transmit-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.spinner {
		display: inline-block;
		width: 14px;
		height: 14px;
		border: 2px solid rgba(26, 22, 18, 0.4);
		border-top-color: var(--raw-bg-base);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}
</style>
