import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/svelte';

/**
 * RED-PHASE component acceptance tests — spec #572 Task 5, R19/R20,
 * Scenarios 8/9/15.
 *
 * Surface: the chat UI component that consumes the parsed SSE stream and
 * renders the transcript (`ChatTab.svelte` → `ChatMessage.svelte`).
 *
 * Mock seam (documented for task 981 / Rune — implement against the SAME seam):
 *   We mock `$lib/api` so `api.chat()` returns a synthetic `Response` whose
 *   body is a `ReadableStream` of raw SSE bytes. The REAL `parseSSEStream`
 *   and the REAL components run end-to-end; the regression manifests at the
 *   rendering layer, so red here is behavioral, not a stub artifact.
 *
 * Contract asserted (NOT the visual mechanism — designer's latitude per R20):
 *   - NEGATIVE (load-bearing): the canned message MUST NOT render inside the
 *     assistant content bubble (`.assistant-bubble` — AI-bubble styling).
 *   - POSITIVE: the canned message IS surfaced via the component's
 *     error/notice treatment (the existing `role="alert"` / `.error-message`
 *     path the component already uses for `event: error`). R20 permits a
 *     distinct visual treatment; we assert "not content treatment" + "is some
 *     error/notice treatment", not a specific new class name.
 *
 * Stable signals used (no implementation reading required):
 *   notice  = element with role="alert" (ChatTab renders error this way today)
 *   content = `.assistant-bubble` (ChatMessage's AI-bubble class)
 */

const REFUSAL_MSG = 'The assistant declined to respond to that request.';
const CONTEXT_MSG = "The request exceeded the model's context window. Try a shorter input.";
const TRUNCATED_MSG = 'The response was cut off due to length.';

/** A Response whose body streams the given raw SSE text as one chunk. */
function sseResponse(raw: string): Response {
	const encoder = new TextEncoder();
	const stream = new ReadableStream<Uint8Array>({
		start(controller) {
			controller.enqueue(encoder.encode(raw));
			controller.close();
		},
	});
	return new Response(stream, {
		status: 200,
		headers: { 'Content-Type': 'text/event-stream' },
	});
}

const chatMock = vi.fn();

// Mock the FULL $lib/api seam touched by ChatTab's render subtree, not just
// chat(). ChatTab renders SuggestedQuestions.svelte, which calls
// api.getFaqSuggestions() in a mount $effect — an incomplete mock makes that
// `undefined`, throwing `TypeError: api.getFaqSuggestions is not a function`
// during render BEFORE any behavioral assertion runs (a WRONG-reason red).
// getFaqSuggestions resolves [] (FaqSuggestion[] — SuggestedQuestions only
// reads `.length`/iterates), so the component renders cleanly and the
// behavioral assertions are actually reached. Only chat() is the
// behavior-under-test seam; the rest are inert no-ops sized to the contract.
vi.mock('$lib/api', () => ({
	api: {
		chat: (...args: unknown[]) => chatMock(...args),
		getFaqSuggestions: () => Promise.resolve([]),
	},
}));

import ChatTab from '$lib/components/ai/ChatTab.svelte';

async function sendAndSettle(): Promise<void> {
	const textarea = screen.getByPlaceholderText(/type your transmission/i) as HTMLTextAreaElement;
	const button = screen.getByRole('button', { name: /transmit message/i });
	// `fireEvent.input` updates the bound value; `fireEvent.click` transmits.
	// (@testing-library/svelte re-exports fireEvent; project has no user-event.)
	await fireEvent.input(textarea, { target: { value: 'tell me about Peter' } });
	await fireEvent.click(button);
}

describe('ChatTab — new typed terminal events render as notice, never as content (spec #572 R19/R20)', () => {
	beforeEach(() => {
		chatMock.mockReset();
	});
	afterEach(() => {
		document.body.innerHTML = '';
	});

	it('refusal: canned text is NOT in the assistant content bubble (R20 / Scenario 8)', async () => {
		chatMock.mockResolvedValue(
			sseResponse(`event: refusal\ndata: ${REFUSAL_MSG}\ndata: [DONE]\n`)
		);
		const { container } = render(ChatTab);
		await sendAndSettle();

		await waitFor(() => {
			// Negative (load-bearing): no AI bubble carries the canned refusal text
			const bubbles = Array.from(container.querySelectorAll('.assistant-bubble'));
			for (const b of bubbles) {
				expect(b.textContent ?? '').not.toContain(REFUSAL_MSG);
			}
		});
	});

	it('refusal: canned text IS surfaced via the error/notice treatment (R20 / Scenario 8)', async () => {
		chatMock.mockResolvedValue(
			sseResponse(`event: refusal\ndata: ${REFUSAL_MSG}\ndata: [DONE]\n`)
		);
		render(ChatTab);
		await sendAndSettle();

		const alert = await screen.findByRole('alert');
		expect(alert).toHaveTextContent(REFUSAL_MSG);
	});

	it('context_exceeded: canned text is NOT in the assistant content bubble (R20 / Scenario 9)', async () => {
		chatMock.mockResolvedValue(
			sseResponse(`event: context_exceeded\ndata: ${CONTEXT_MSG}\ndata: [DONE]\n`)
		);
		const { container } = render(ChatTab);
		await sendAndSettle();

		await waitFor(() => {
			const bubbles = Array.from(container.querySelectorAll('.assistant-bubble'));
			for (const b of bubbles) {
				expect(b.textContent ?? '').not.toContain(CONTEXT_MSG);
			}
		});
	});

	it('context_exceeded: canned text IS surfaced via the error/notice treatment (R20 / Scenario 9)', async () => {
		chatMock.mockResolvedValue(
			sseResponse(`event: context_exceeded\ndata: ${CONTEXT_MSG}\ndata: [DONE]\n`)
		);
		render(ChatTab);
		await sendAndSettle();

		const alert = await screen.findByRole('alert');
		expect(alert).toHaveTextContent(CONTEXT_MSG);
	});

	it('truncated: pre-truncation content renders, canned notice text does NOT pollute the bubble (R20 / Scenario 15)', async () => {
		chatMock.mockResolvedValue(
			sseResponse(
				`data: Peter ships reliable systems.\nevent: truncated\ndata: ${TRUNCATED_MSG}\ndata: [DONE]\n`
			)
		);
		const { container } = render(ChatTab);
		await sendAndSettle();

		// The real content that arrived before truncation must still render...
		const bubble = await waitFor(() => {
			const b = container.querySelector('.assistant-bubble');
			expect(b).not.toBeNull();
			return b as Element;
		});
		expect(bubble.textContent ?? '').toContain('Peter ships reliable systems.');
		// ...but the truncated canned notice must NOT be inside the AI bubble.
		expect(bubble.textContent ?? '').not.toContain(TRUNCATED_MSG);
	});

	it('truncated: canned notice text IS surfaced via the notice treatment (R19 / Scenario 15)', async () => {
		chatMock.mockResolvedValue(
			sseResponse(
				`data: partial.\nevent: truncated\ndata: ${TRUNCATED_MSG}\ndata: [DONE]\n`
			)
		);
		render(ChatTab);
		await sendAndSettle();

		const alert = await screen.findByRole('alert');
		expect(alert).toHaveTextContent(TRUNCATED_MSG);
	});
});
