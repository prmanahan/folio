import { describe, it, expect } from 'vitest';
import { parseSSEStream, SSEError } from '../sse';

/**
 * Helper: build a ReadableStreamDefaultReader from an array of string chunks.
 * Each string chunk simulates a raw network read() call.
 */
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

/** Collect all tokens from the async generator into an array. */
async function collect(reader: ReadableStreamDefaultReader<Uint8Array>): Promise<string[]> {
	const tokens: string[] = [];
	for await (const token of parseSSEStream(reader)) {
		tokens.push(token);
	}
	return tokens;
}

describe('parseSSEStream', () => {
	it('parses a single data line', async () => {
		const reader = makeReader(['data: hello\n\n']);
		expect(await collect(reader)).toEqual(['hello']);
	});

	it('parses multiple tokens in one chunk', async () => {
		const reader = makeReader(['data: foo\ndata: bar\ndata: baz\n']);
		expect(await collect(reader)).toEqual(['foo', 'bar', 'baz']);
	});

	it('stops at [DONE] sentinel', async () => {
		const reader = makeReader(['data: hello\ndata: [DONE]\ndata: should-not-appear\n']);
		expect(await collect(reader)).toEqual(['hello']);
	});

	it('handles [DONE] in its own chunk', async () => {
		const reader = makeReader(['data: part1\n', 'data: [DONE]\n']);
		expect(await collect(reader)).toEqual(['part1']);
	});

	it('handles data split across multiple read() calls', async () => {
		// "data: hel" then "lo\n" -- the parser must buffer the incomplete line
		const reader = makeReader(['data: hel', 'lo\n', 'data: world\n']);
		expect(await collect(reader)).toEqual(['hello', 'world']);
	});

	it('ignores empty lines', async () => {
		const reader = makeReader(['\n\ndata: token\n\n\n']);
		expect(await collect(reader)).toEqual(['token']);
	});

	it('ignores lines that do not start with "data: "', async () => {
		const reader = makeReader(['event: ping\ndata: payload\nid: 42\n']);
		expect(await collect(reader)).toEqual(['payload']);
	});

	it('returns empty array when stream is immediately done', async () => {
		const reader = makeReader([]);
		expect(await collect(reader)).toEqual([]);
	});

	it('returns empty array when only [DONE] is sent', async () => {
		const reader = makeReader(['data: [DONE]\n']);
		expect(await collect(reader)).toEqual([]);
	});

	it('handles a realistic multi-chunk SSE stream', async () => {
		// Simulates a token-by-token stream followed by [DONE]
		const chunks = [
			'data: Hello',
			',\ndata:  world',
			'!\ndata: [DONE]\n',
		];
		const reader = makeReader(chunks);
		const tokens = await collect(reader);
		expect(tokens.join('')).toBe('Hello, world!');
	});

	it('handles data with spaces preserved', async () => {
		const reader = makeReader(['data:  leading space\n']);
		// slice(6) on "data:  leading space" gives " leading space"
		expect(await collect(reader)).toEqual([' leading space']);
	});

	it('throws SSEError when server sends an error event', async () => {
		const reader = makeReader(['event: error\ndata: AI response failed. Please try again.\n']);
		await expect(collect(reader)).rejects.toThrow(SSEError);
		await expect(collect(makeReader(['event: error\ndata: AI response failed. Please try again.\n'])))
			.rejects.toThrow('AI response failed. Please try again.');
	});

	it('throws SSEError mid-stream when error event arrives after content', async () => {
		const reader = makeReader(['data: partial\nevent: error\ndata: stream interrupted\n']);
		const tokens: string[] = [];
		await expect(async () => {
			for await (const token of parseSSEStream(reader)) {
				tokens.push(token);
			}
		}).rejects.toThrow(SSEError);
		expect(tokens).toEqual(['partial']);
	});

	it('handles error event split across chunks', async () => {
		const reader = makeReader(['event: err', 'or\ndata: broken\n']);
		await expect(collect(reader)).rejects.toThrow('broken');
	});
});
