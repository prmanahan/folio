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
 * Returns an async generator that yields one string per EVENT (events end at a
 * blank line, per the SSE spec). The `data:` lines of one event are rejoined
 * with "\n": the backend's axum `Event::data` splits a delta containing
 * newlines into several `data:` lines, so a delta "a\nb" arrives as
 * `data: a\ndata: b\n\n` and yields "a\nb". Line endings may be \n, \r\n or
 * \r. Stops when an event whose whole data is [DONE] is received.
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

const DONE = Symbol('sse-done');

/** SSE field values drop exactly one leading space. */
function stripFieldSpace(value: string): string {
	return value.startsWith(' ') ? value.slice(1) : value;
}

/**
 * Events are dispatched at the blank-line boundary (SSE spec). The data lines
 * of one event are rejoined with "\n": the backend's axum `Event::data` splits
 * a delta containing newlines into several `data:` lines (#4712). Line
 * endings may be \n, \r\n or \r; a trailing \r is held back until the next
 * chunk so a CRLF split across reads is not counted twice.
 */
export async function* parseSSEStream(
	reader: ReadableStreamDefaultReader<Uint8Array>
): AsyncGenerator<string, void, unknown> {
	const decoder = new TextDecoder();
	let buffer = '';
	let pendingCR = false;
	let currentEvent = '';
	let dataLines: string[] = [];

	// Dispatches the accumulated event: the token to yield, null for nothing,
	// or DONE for the sentinel.
	const dispatch = (): string | null | typeof DONE => {
		const event = currentEvent;
		const lines = dataLines;
		currentEvent = '';
		dataLines = [];
		if (lines.length === 0) return null;
		const data = lines.join('\n');

		// Typed terminal events (error / refusal / context_exceeded /
		// truncated): throw the matching typed error. Any preceding content
		// tokens have ALREADY been yielded, so a consumer that rendered partial
		// content keeps it (truncated ordering, Scenario 15 / R11a).
		if (Object.hasOwn(TYPED_EVENT_ERRORS, event)) throw new TYPED_EVENT_ERRORS[event](data);

		return data === '[DONE]' ? DONE : data;
	};

	while (true) {
		const { done, value } = await reader.read();
		if (done) break;

		let chunk = decoder.decode(value, { stream: true });
		if (pendingCR) {
			chunk = '\r' + chunk;
			pendingCR = false;
		}
		if (chunk.endsWith('\r')) {
			pendingCR = true;
			chunk = chunk.slice(0, -1);
		}
		buffer += chunk.replace(/\r\n|\r/g, '\n');

		// Split on newlines but keep the last (possibly incomplete) line
		const lines = buffer.split('\n');
		buffer = lines.pop() ?? '';

		for (const line of lines) {
			if (line === '') {
				const out = dispatch();
				if (out === DONE) return;
				if (out !== null) yield out;
			} else if (line.startsWith('event:')) {
				currentEvent = stripFieldSpace(line.slice(6));
			} else if (line.startsWith('data:')) {
				dataLines.push(stripFieldSpace(line.slice(5)));
			}
		}
	}

	// Flush the decoder: a multi-byte character split across the last chunks
	// would otherwise be dropped.
	// A held-back trailing \r only ended a line already in `buffer`, so it adds
	// nothing at EOF.
	buffer += decoder.decode();

	// A stream that ends without a terminating blank line (dropped connection,
	// proxy cut) still carries content the consumer should keep: the pre-#4712
	// parser yielded such a tail, and discarding it would lose the end of an
	// answer. The last, unterminated line is flushed with the pending event.
	if (buffer.startsWith('data:')) dataLines.push(stripFieldSpace(buffer.slice(5)));
	else if (buffer.startsWith('event:')) currentEvent = stripFieldSpace(buffer.slice(6));
	const out = dispatch();
	if (out !== DONE && out !== null) yield out;
}

