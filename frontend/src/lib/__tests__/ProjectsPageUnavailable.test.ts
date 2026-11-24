/**
 * R-24 (docs/specs/2026-11-18-live-lists-and-blog-move.md): /projects loads GET /api/lists/projects,
 * shows only its heading while pending, aborts at 15 s, and shows one exact alert line plus the two
 * GitHub links on every failed outcome. Plan T12 (#4853).
 *
 * Only global `fetch` is substituted (the network boundary). A never-settling fetch rejects with an
 * AbortError when the signal the page passes aborts, like the real one. Fake timers drive the 15 s.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, waitFor } from '@testing-library/svelte';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import ProjectsPage from '../../routes/(public)/projects/+page.svelte';

const LINE = 'The project list is temporarily unavailable; try again shortly.';
const URL_PATH = '/api/lists/projects';

const EXPECTED = JSON.parse(
	readFileSync(resolve(import.meta.dirname, '../../../scripts/projects-fixtures/list-expected.json'), 'utf8'),
);

let fetchMock: ReturnType<typeof vi.fn>;
let signals: AbortSignal[];

function json(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
}

function stubFetch(impl: (signal: AbortSignal | undefined) => Promise<Response>) {
	signals = [];
	fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
		if (String(input) !== URL_PATH) return Promise.resolve(new Response('nope', { status: 500 }));
		if (init?.signal) signals.push(init.signal);
		return impl(init?.signal ?? undefined);
	});
	vi.stubGlobal('fetch', fetchMock);
}

/** Never settles on its own; rejects like fetch when aborted. */
function hang(signal: AbortSignal | undefined): Promise<Response> {
	return new Promise((_, reject) => {
		signal?.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')));
	});
}

function line(c: HTMLElement): HTMLElement | null {
	return c.querySelector<HTMLElement>('[role="alert"]');
}
function repoLinks(c: HTMLElement): HTMLAnchorElement[] {
	return [...c.querySelectorAll<HTMLAnchorElement>('a')].filter((a) =>
		(a.getAttribute('href') ?? '').startsWith('https://github.com/'),
	);
}

function expectUnavailable(c: HTMLElement) {
	const el = line(c);
	expect(el).not.toBeNull();
	expect(el?.textContent?.trim()).toBe(LINE);
	expect(c.querySelectorAll('h2').length).toBe(0);
	expect(c.textContent).not.toContain('No projects yet.');
	const links = repoLinks(c);
	expect(links.map((a) => a.getAttribute('href'))).toEqual([
		'https://github.com/mnemra',
		'https://github.com/prmanahan',
	]);
	const texts = links.map((a) => (a.textContent ?? '').trim());
	expect(texts[0]).toContain('mnemra');
	expect(texts[1]).toContain('prmanahan');
	expect(new Set(texts).size).toBe(2);
	for (const a of links) {
		expect(a.getAttribute('target')).toBe('_blank');
		expect(a.getAttribute('rel')).toBe('noopener noreferrer');
	}
}

beforeEach(() => {
	vi.unstubAllGlobals();
});
afterEach(() => {
	vi.useRealTimers();
	vi.unstubAllGlobals();
});

describe('R-24: pending and the 15 s abort', () => {
	it('while pending shows only the heading: no unavailable line, no "No projects yet.", before 15 s', async () => {
		vi.useFakeTimers();
		stubFetch(hang);
		const { container } = render(ProjectsPage);
		await vi.advanceTimersByTimeAsync(14_999);

		expect(fetchMock).toHaveBeenCalledTimes(1);
		expect(container.querySelector('h1')?.textContent).toBe('Projects');
		expect(line(container)).toBeNull();
		expect(container.textContent).not.toContain('No projects yet.');
		expect(container.textContent).not.toContain('temporarily unavailable');
		expect(container.querySelectorAll('a').length).toBe(0);
		expect(signals[0].aborted).toBe(false);
	});

	it('a never-resolving fetch shows the unavailable line after 15 s, and the request was aborted', async () => {
		vi.useFakeTimers();
		stubFetch(hang);
		const { container } = render(ProjectsPage);
		await vi.advanceTimersByTimeAsync(15_000);

		expect(signals[0].aborted).toBe(true);
		expectUnavailable(container);
	});

	it('a valid 200 that would arrive after 15 s is never applied', async () => {
		vi.useFakeTimers();
		// Ignores the signal on purpose: resolves at 16 s with a valid document.
		stubFetch(() => new Promise((resolveFn) => setTimeout(() => resolveFn(json(EXPECTED)), 16_000)));
		const { container } = render(ProjectsPage);
		await vi.advanceTimersByTimeAsync(15_000);
		expect(signals[0].aborted).toBe(true);
		await vi.advanceTimersByTimeAsync(2_000);

		expectUnavailable(container);
		expect(repoLinks(container).length).toBe(2);
	});

	it('a response that settles before 15 s is applied and the timer does not later replace it', async () => {
		vi.useFakeTimers();
		stubFetch(async () => json(EXPECTED));
		const { container } = render(ProjectsPage);
		await vi.advanceTimersByTimeAsync(100);
		expect(container.querySelectorAll('h2').length).toBe(EXPECTED.groups.length);
		await vi.advanceTimersByTimeAsync(20_000);

		expect(line(container)).toBeNull();
		expect(container.querySelectorAll('h2').length).toBe(EXPECTED.groups.length);
	});
});

describe('R-24: every other outcome shows the exact line and the two links', () => {
	const cases: [string, () => Promise<Response>][] = [
		['a 503 with the route error body', async () => json({ error: 'Projects list unavailable' }, 503)],
		['another status (500)', async () => new Response('x', { status: 500 })],
		['a 404 whose body is a valid list', async () => json(EXPECTED, 404)],
		['a network error', () => Promise.reject(new TypeError('Failed to fetch'))],
		['a non-JSON 200 body', async () => new Response('<html></html>', { status: 200 })],
		['a wrong shape (groups as an object)', async () => json({ fetched_at: EXPECTED.fetched_at, groups: {} })],
	];
	for (const [name, impl] of cases) {
		it(name, async () => {
			stubFetch(impl);
			const { container } = render(ProjectsPage);
			await waitFor(() => expect(line(container)).not.toBeNull());
			expectUnavailable(container);
		});
	}

	it('an empty list is not the unavailable line: it keeps "No projects yet."', async () => {
		stubFetch(async () => json({ fetched_at: EXPECTED.fetched_at, groups: [] }));
		const { container } = render(ProjectsPage);
		await waitFor(() => expect(container.textContent).toContain('No projects yet.'));
		expect(line(container)).toBeNull();
	});
});

describe('R-24 / D21: text only', () => {
	it('the description <img src=x onerror=alert(1)> **b** creates no img or strong element', async () => {
		stubFetch(async () =>
			json({
				fetched_at: EXPECTED.fetched_at,
				groups: [
					{ owner: 'mnemra', repos: [{ name: 'fixture-solo', description: '<img src=x onerror=alert(1)> **b**' }] },
				],
			}),
		);
		const { container } = render(ProjectsPage);
		await waitFor(() => expect(repoLinks(container).length).toBe(1));

		expect(container.querySelector('img')).toBeNull();
		expect(container.querySelector('strong')).toBeNull();
		expect(container.querySelector('.description')?.textContent).toBe('<img src=x onerror=alert(1)> **b**');
	});
});
