import { describe, it, expect } from 'vitest';
import { parseSSEStream } from '../sse';

// #4712: the backend frames with axum's Event::data, which splits a delta
// containing newlines (\n, \r or \r\n) into several `data:` lines of ONE event.
// Per the SSE spec the client rejoins them with "\n" at the blank-line boundary.

function makeReader(chunks: string[]): ReadableStreamDefaultReader<Uint8Array> {
	const encoder = new TextEncoder();
	let index = 0;
	return {
		async read() {
			if (index >= chunks.length) return { done: true, value: undefined };
			return { done: false, value: encoder.encode(chunks[index++]) };
		},
		releaseLock() {},
		cancel() {
			return Promise.resolve();
		}
	} as unknown as ReadableStreamDefaultReader<Uint8Array>;
}

async function collect(chunks: string[]): Promise<string[]> {
	const tokens: string[] = [];
	for await (const t of parseSSEStream(makeReader(chunks))) tokens.push(t);
	return tokens;
}

describe('parseSSEStream: multi-line events (#4712)', () => {
	it('rejoins the data lines of one event with a newline', async () => {
		expect(await collect(['data: line one\ndata: line two\n\n'])).toEqual(['line one\nline two']);
	});

	it('keeps a blank line inside a delta (paragraph break)', async () => {
		expect(await collect(['data: a\ndata: \ndata: b\n\n'])).toEqual(['a\n\nb']);
	});

	it('rejoins when the event is split across chunks', async () => {
		expect(await collect(['data: a\nda', 'ta: b\n', '\n', 'data: c\n\n'])).toEqual(['a\nb', 'c']);
	});

	it('keeps separate events separate', async () => {
		expect(await collect(['data: one\n\ndata: two\n\n'])).toEqual(['one', 'two']);
	});

	it('does not end the render on a model line that is exactly [DONE] inside a multi-line delta', async () => {
		expect(await collect(['data: x\ndata: [DONE]\ndata: y\n\n', 'data: after\n\n'])).toEqual([
			'x\n[DONE]\ny',
			'after'
		]);
	});

	it('still ends on the [DONE] sentinel event', async () => {
		expect(await collect(['data: hi\n\ndata: [DONE]\n\ndata: ignored\n\n'])).toEqual(['hi']);
	});

	it('handles CRLF line endings without a trailing \\r in the text', async () => {
		expect(await collect(['data: hello\r\ndata: world\r\n\r\n'])).toEqual(['hello\nworld']);
	});

	it('handles CRLF split between chunks', async () => {
		expect(await collect(['data: hello\r', '\n\r', '\ndata: next\r\n\r\n'])).toEqual([
			'hello',
			'next'
		]);
	});

	it('handles bare CR line endings without leaking a literal "data: " into the text', async () => {
		expect(await collect(['data: a\rdata: b\r\rdata: c\r\r'])).toEqual(['a\nb', 'c']);
	});

	it('flushes a final event that has no terminating blank line', async () => {
		expect(await collect(['data: tail'])).toEqual(['tail']);
	});

	it('does not double-count a CRLF whose CR ends one chunk and LF starts the next', async () => {
		// With a double count the "\r" + "\n" would read as a blank line and
		// split the event into 'a' and 'b'.
		expect(await collect(['data: a\r', '\ndata: b\r\n\r\n'])).toEqual(['a\nb']);
	});

	it("matches axum 0.8.8's own Event::data framing bytes (golden)", async () => {
		// From axum's unit tests: a multi-line delta split on \n and \r, ending
		// in an empty line.
		expect(await collect(['data: moon star\ndata: sunset bye\rdata: \n\n'])).toEqual([
			'moon star\nsunset bye\n'
		]);
	});

	it('accepts a data field with no space after the colon', async () => {
		expect(await collect(['data:x\n\n'])).toEqual(['x']);
	});

	it('flushes the decoder at EOF (a cut multi-byte character is not silently dropped)', async () => {
		const bytes = new Uint8Array([...new TextEncoder().encode('data: caf'), 0xc3]);
		let sent = false;
		const reader = {
			async read() {
				if (sent) return { done: true, value: undefined };
				sent = true;
				return { done: false, value: bytes };
			},
			releaseLock() {},
			cancel: () => Promise.resolve()
		} as unknown as ReadableStreamDefaultReader<Uint8Array>;
		const tokens: string[] = [];
		for await (const t of parseSSEStream(reader)) tokens.push(t);
		expect(tokens).toEqual(['caf' + String.fromCharCode(0xfffd)]);
	});

	it.each(['constructor', 'toString', '__proto__', 'hasOwnProperty'])(
		'treats an unknown event name %j as ordinary content, not a typed error',
		async (name) => {
			expect(await collect([`event: ${name}\ndata: x\n\n`])).toEqual(['x']);
		}
	);
});
