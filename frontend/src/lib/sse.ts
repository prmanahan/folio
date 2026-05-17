/**
 * SSE (Server-Sent Events) stream parser.
 *
 * Parses the text/event-stream format:
 *   data: some content\n\n
 *   data: [DONE]\n\n
 *   event: error\ndata: error message\n\n
 *   event: refusal\ndata: <canned message>\n\n
 *   event: context_exceeded\ndata: <canned message>\n\n
 *   event: truncated\ndata: <canned message>\n\n
 *
 * Returns an async generator that yields token strings. Stops when [DONE]
 * sentinel is received.
 *
 * --- Public contract (spec #572 T5, R19/R20) ---
 *
 * The generator yields ONLY ordinary content tokens as `string`. Terminal,
 * non-content conditions are signalled by THROWING a typed error AFTER any
 * preceding content tokens have already been yielded (so a consumer that has
 * rendered partial content keeps it — see TruncatedError):
 *
 *   - `SSEError`            — base class. `event: error` (unclassified
 *                             catch-all) throws this directly.
 *   - `RefusalError`        — thrown on `event: refusal`. The model declined.
 *   - `ContextExceededError`— thrown on `event: context_exceeded`. The request
 *                             tripped the model's context window.
 *   - `TruncatedError`      — thrown on `event: truncated`. The response was
 *                             cut off due to length. INFORMATIONAL: any
 *                             content tokens yielded before the throw are
 *                             valid and should be retained; the truncated
 *                             notice is additional, not a replacement.
 *
 * `RefusalError`, `ContextExceededError`, and `TruncatedError` all extend
 * `SSEError`, so existing `catch (e instanceof SSEError)` consumers route them
 * through the same error/notice path with zero changes (R19 explicitly permits
 * reuse). The `.message` of each carries the canned, server-supplied text
 * verbatim — never raw model text (R29). Discriminate the three with
 * `instanceof` on the specific subclass when distinct treatment is desired
 * (R20, designer's latitude).
 *
 * The plain-`string` yield contract is unchanged from the original parser:
 * code that does `for await (const token of parseSSEStream(reader))` and
 * treats each token as content text continues to work unmodified.
 */

export class SSEError extends Error {
	constructor(message: string) {
		super(message);
		this.name = 'SSEError';
	}
}

/** Thrown on `event: refusal` — the model declined to respond (Scenario 8). */
export class RefusalError extends SSEError {
	constructor(message: string) {
		super(message);
		this.name = 'RefusalError';
	}
}

/**
 * Thrown on `event: context_exceeded` — the request tripped the model's
 * context window (Scenario 9).
 */
export class ContextExceededError extends SSEError {
	constructor(message: string) {
		super(message);
		this.name = 'ContextExceededError';
	}
}

/**
 * Thrown on `event: truncated` — the response was cut off due to length
 * (Scenario 15). Informational: content tokens yielded before this throw are
 * valid model output and MUST be retained by the consumer; the truncated
 * notice is additional, not a replacement.
 */
export class TruncatedError extends SSEError {
	constructor(message: string) {
		super(message);
		this.name = 'TruncatedError';
	}
}

/** Maps a recognized typed `event:` name to its error constructor. */
const TYPED_EVENT_ERRORS: Record<string, new (message: string) => SSEError> = {
	error: SSEError,
	refusal: RefusalError,
	context_exceeded: ContextExceededError,
	truncated: TruncatedError,
};

export async function* parseSSEStream(
	reader: ReadableStreamDefaultReader<Uint8Array>
): AsyncGenerator<string, void, unknown> {
	const decoder = new TextDecoder();
	let buffer = '';
	let currentEvent = '';

	while (true) {
		const { done, value } = await reader.read();
		if (done) break;

		buffer += decoder.decode(value, { stream: true });

		// Split on newlines but keep the last (possibly incomplete) line
		const lines = buffer.split('\n');
		buffer = lines.pop() ?? '';

		for (const line of lines) {
			if (line.startsWith('event: ')) {
				currentEvent = line.slice(7);
				continue;
			}
			if (!line.startsWith('data: ')) continue;
			const data = line.slice(6);

			// Typed terminal events (error / refusal / context_exceeded /
			// truncated): throw the matching typed error. Any preceding
			// content tokens have ALREADY been yielded above, so a consumer
			// that rendered partial content keeps it (truncated ordering,
			// Scenario 15 / R11a).
			const ErrCtor = TYPED_EVENT_ERRORS[currentEvent];
			if (ErrCtor) {
				throw new ErrCtor(data);
			}

			currentEvent = '';
			if (data === '[DONE]') return;
			yield data;
		}
	}

	// Handle any remaining buffer content after stream ends
	if (buffer.startsWith('data: ')) {
		const data = buffer.slice(6);
		if (data !== '[DONE]') yield data;
	}
}
