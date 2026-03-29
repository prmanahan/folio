/**
 * SSE (Server-Sent Events) stream parser.
 *
 * Parses the text/event-stream format:
 *   data: some content\n\n
 *   data: [DONE]\n\n
 *   event: error\ndata: error message\n\n
 *
 * Returns an async generator that yields token strings. Stops when [DONE]
 * sentinel is received. Throws SSEError if an error event is received.
 */

export class SSEError extends Error {
	constructor(message: string) {
		super(message);
		this.name = 'SSEError';
	}
}

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

			if (currentEvent === 'error') {
				throw new SSEError(data);
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
