import { describe, it, expect } from 'vitest';
import {
	parseSSEStream,
	RefusalError,
	ContextExceededError,
	TruncatedError,
} from '../sse';

/**
 * RED-PHASE acceptance tests — spec #572 Task 5, R19/R20, Scenarios 8/9/15.
 *
 * Contract under test: the SSE parser MUST recognize three new server event
 * types — `event: refusal`, `event: context_exceeded`, `event: truncated` —
 * and route them to a typed error/notice path. They MUST NOT be emitted as
 * content tokens (the regression: today they fall through `parseSSEStream`
 * and are yielded as if the model said them).
 *
 * CONTRACT vs MECHANISM (read before changing anything — task 981 / Rune):
 *   The exported `consumeClassified` RETURN SHAPE is the locked contract.
 *   The classifier body below is a SHIM. Task 981 may need to update its
 *   inspection logic to match whatever output mechanism the parser ends up
 *   using (typed throw, discriminated-union yield, callback, second channel,
 *   ...). What does NOT change is what a consumer can observe:
 *     - content[]            : ordinary content tokens, in order
 *     - refusal?             : canned refusal message (Scenario 8)
 *     - contextExceeded?     : canned context-exceeded message (Scenario 9)
 *     - truncated?           : { message, contentBefore[] } (Scenario 15)
 *   Tests assert ONLY on this shape — never on parseSSEStream internals or a
 *   specific thrown class. Satisfying these tests = satisfying R19/R20.
 *
 * Canned strings are verbatim from the spec API Contract / R18 / Scenarios:
 *   refusal:          "The assistant declined to respond to that request."
 *   context_exceeded: "The request exceeded the model's context window. Try a shorter input."
 *   truncated:        "The response was cut off due to length."
 */

const REFUSAL_MSG = 'The assistant declined to respond to that request.';
const CONTEXT_MSG = "The request exceeded the model's context window. Try a shorter input.";
const TRUNCATED_MSG = 'The response was cut off due to length.';

/** Build a ReadableStreamDefaultReader from raw string chunks (one per read()). */
function makeReader(chunks: string[]): ReadableStreamDefaultReader<Uint8Array> {
	const encoder = new TextEncoder();
	let index = 0;
	return {
		async read() {
			if (index >= chunks.length) {
				return { done: true, value: undefined };
			}
			return { done: false, value: encoder.encode(chunks[index++]) };
		},
		releaseLock() {},
		cancel() {
			return Promise.resolve();
		},
	} as unknown as ReadableStreamDefaultReader<Uint8Array>;
}

interface Classified {
	content: string[];
	refusal?: string;
	contextExceeded?: string;
	truncated?: { message: string; contentBefore: string[] };
}

/**
 * Consumer-wrapper helper — THIS SIGNATURE IS THE CONTRACT.
 *
 * It drives `parseSSEStream` and classifies its observable output into the
 * four fields above. Today the parser yields the new events' payloads as
 * plain content tokens (the regression), so this classifier sees them as
 * content and leaves `refusal`/`contextExceeded`/`truncated` undefined — the
 * tests below then fail for the BEHAVIORAL reason (not classified), which is
 * the correct red.
 *
 * task 981 / Rune: when the parser's API lands, update the inspection logic
 * INSIDE this helper (try/catch a typed throw, switch on a discriminated
 * yield, register a callback — your choice) so the four fields are populated
 * from the real signal. Do NOT change the return shape or the assertions.
 */
async function consumeClassified(
	reader: ReadableStreamDefaultReader<Uint8Array>
): Promise<Classified> {
	const result: Classified = { content: [] };
	try {
		for await (const token of parseSSEStream(reader)) {
			// SHIM (task 981): the chosen mechanism is typed-throw — every
			// yield from parseSSEStream IS a content token (the plain-string
			// yield contract is preserved). Typed terminal events arrive via
			// the catch branch below, not as a discriminated yield. This
			// branch is therefore correct as-is for this mechanism.
			result.content.push(token);
		}
	} catch (e) {
		// SHIM (task 981): the parser signals typed terminal events by
		// THROWING a typed SSEError subclass AFTER yielding any preceding
		// content. Inspect the thrown value and populate the matching field;
		// for truncated, contentBefore is the content already accumulated
		// (the throw happens after those tokens were yielded above).
		if (e instanceof TruncatedError) {
			result.truncated = { message: e.message, contentBefore: [...result.content] };
		} else if (e instanceof ContextExceededError) {
			result.contextExceeded = e.message;
		} else if (e instanceof RefusalError) {
			result.refusal = e.message;
		}
		// A bare SSEError (unclassified `event: error`) is intentionally not
		// mapped to any of the four fields — it is the catch-all, not one of
		// the three classified terminal events under test.
	}
	return result;
}

describe('SSE parser — new typed terminal events (spec #572 R19/R20)', () => {
	describe('Feature: refusal event (Scenario 8)', () => {
		it('regression: refusal payload does NOT surface as a content token (R19 / Scenario 8)', async () => {
			// Given the server emits `event: refusal` then `[DONE]`
			const reader = makeReader([
				`event: refusal\ndata: ${REFUSAL_MSG}\ndata: [DONE]\n`,
			]);
			// When the stream is consumed
			const r = await consumeClassified(reader);
			// Then the canned refusal text is NEVER in the content stream
			expect(r.content.join('')).toBe('');
			expect(r.content).toEqual([]);
		});

		it('classifies refusal distinctly and carries its canned message verbatim (Scenario 8)', async () => {
			const reader = makeReader([
				`event: refusal\ndata: ${REFUSAL_MSG}\ndata: [DONE]\n`,
			]);
			const r = await consumeClassified(reader);
			// Routed to the refusal classification with the EXACT spec string
			expect(r.refusal).toBe(REFUSAL_MSG);
			// Not misrouted to a sibling classification
			expect(r.contextExceeded).toBeUndefined();
			expect(r.truncated).toBeUndefined();
		});

		it('terminates cleanly relative to [DONE] on refusal — no hang (Scenario 8)', async () => {
			const reader = makeReader([
				`event: refusal\ndata: ${REFUSAL_MSG}\ndata: [DONE]\n`,
				'data: should-never-be-read\n',
			]);
			const r = await consumeClassified(reader);
			// Consumer reached a terminal state; nothing past [DONE] leaked in
			expect(r.content).toEqual([]);
			expect(r.refusal).toBe(REFUSAL_MSG);
		});
	});

	describe('Feature: context_exceeded event (Scenario 9)', () => {
		it('regression: context_exceeded payload does NOT surface as a content token (R19 / Scenario 9)', async () => {
			const reader = makeReader([
				`event: context_exceeded\ndata: ${CONTEXT_MSG}\ndata: [DONE]\n`,
			]);
			const r = await consumeClassified(reader);
			expect(r.content.join('')).toBe('');
			expect(r.content).toEqual([]);
		});

		it('classifies context_exceeded distinctly and carries its canned message verbatim (Scenario 9)', async () => {
			const reader = makeReader([
				`event: context_exceeded\ndata: ${CONTEXT_MSG}\ndata: [DONE]\n`,
			]);
			const r = await consumeClassified(reader);
			expect(r.contextExceeded).toBe(CONTEXT_MSG);
			expect(r.refusal).toBeUndefined();
			expect(r.truncated).toBeUndefined();
		});

		it('terminates cleanly relative to [DONE] on context_exceeded — no hang (Scenario 9)', async () => {
			const reader = makeReader([
				`event: context_exceeded\ndata: ${CONTEXT_MSG}\ndata: [DONE]\n`,
				'data: should-never-be-read\n',
			]);
			const r = await consumeClassified(reader);
			expect(r.content).toEqual([]);
			expect(r.contextExceeded).toBe(CONTEXT_MSG);
		});
	});

	describe('Feature: truncated event + ordering (Scenario 15)', () => {
		it('regression: truncated payload does NOT surface as a content token (R19 / Scenario 15)', async () => {
			const reader = makeReader([
				'data: Peter has',
				' deep experience.',
				`\nevent: truncated\ndata: ${TRUNCATED_MSG}\ndata: [DONE]\n`,
			]);
			const r = await consumeClassified(reader);
			// The truncated canned message must NOT appear among content tokens
			expect(r.content).not.toContain(TRUNCATED_MSG);
			expect(r.content.join('')).toBe('Peter has deep experience.');
		});

		it('preserves content emitted BEFORE truncation; truncated is additional, not a replacement (Scenario 15)', async () => {
			// Given real content frames arrive, THEN `event: truncated`, THEN [DONE]
			const reader = makeReader([
				'data: alpha',
				'\ndata: beta',
				`\nevent: truncated\ndata: ${TRUNCATED_MSG}\ndata: [DONE]\n`,
			]);
			const r = await consumeClassified(reader);
			// Then all pre-truncation content is yielded normally, in order
			expect(r.content).toEqual(['alpha', 'beta']);
			// And the truncated signal is surfaced separately with its canned text
			expect(r.truncated).toBeDefined();
			expect(r.truncated?.message).toBe(TRUNCATED_MSG);
			// And it reports the same already-emitted content (not a replacement)
			expect(r.truncated?.contentBefore).toEqual(['alpha', 'beta']);
		});

		it('classifies truncated distinctly from refusal and context_exceeded (Scenario 15)', async () => {
			const reader = makeReader([
				'data: partial answer',
				`\nevent: truncated\ndata: ${TRUNCATED_MSG}\ndata: [DONE]\n`,
			]);
			const r = await consumeClassified(reader);
			expect(r.truncated?.message).toBe(TRUNCATED_MSG);
			expect(r.refusal).toBeUndefined();
			expect(r.contextExceeded).toBeUndefined();
		});

		it('terminates cleanly relative to [DONE] on truncated — no hang (Scenario 15)', async () => {
			const reader = makeReader([
				'data: kept',
				`\nevent: truncated\ndata: ${TRUNCATED_MSG}\ndata: [DONE]\n`,
				'data: should-never-be-read\n',
			]);
			const r = await consumeClassified(reader);
			expect(r.content).toEqual(['kept']);
			expect(r.truncated?.message).toBe(TRUNCATED_MSG);
		});
	});

	describe('Feature: the three are mutually distinguishable (R19)', () => {
		it('a consumer can tell refusal vs context_exceeded vs truncated apart', async () => {
			const refusal = await consumeClassified(
				makeReader([`event: refusal\ndata: ${REFUSAL_MSG}\ndata: [DONE]\n`])
			);
			const context = await consumeClassified(
				makeReader([`event: context_exceeded\ndata: ${CONTEXT_MSG}\ndata: [DONE]\n`])
			);
			const truncated = await consumeClassified(
				makeReader([`data: x\nevent: truncated\ndata: ${TRUNCATED_MSG}\ndata: [DONE]\n`])
			);

			// Each populates exactly its own classification, none of the others
			expect([refusal.refusal, refusal.contextExceeded, refusal.truncated]).toEqual([
				REFUSAL_MSG,
				undefined,
				undefined,
			]);
			expect([context.refusal, context.contextExceeded, context.truncated]).toEqual([
				undefined,
				CONTEXT_MSG,
				undefined,
			]);
			expect(truncated.refusal).toBeUndefined();
			expect(truncated.contextExceeded).toBeUndefined();
			expect(truncated.truncated?.message).toBe(TRUNCATED_MSG);
		});
	});
});
