// Shared test support for the two freshness-check specs (R-31 to R-34).
// Not a spec file: vitest collects `scripts/*.spec.mjs` only. The scripts read no
// environment, so the one thing a test doubles is `fetch`; time is the fake clock.
import { vi } from 'vitest';

export const URL_FRESHNESS = 'https://peter.manahan.io/api/lists/freshness';
export const SKEW_MS = 5 * 60 * 1000;
export const H48_MS = 48 * 60 * 60 * 1000;
/** The fake clock's start. Every instant a test builds is an offset from the clock. */
export const T0 = Date.parse('2026-12-01T12:00:00.000Z');

export const iso = (ms) => new Date(ms).toISOString();

/** The freshness body. Times are epoch ms (or null); started_at likewise. */
export const body = (projects, articles, startedAt) =>
	JSON.stringify({
		projects: projects === null ? null : iso(projects),
		articles: articles === null ? null : iso(articles),
		started_at: iso(startedAt),
	});

export const ok = (text) => new Response(text, { status: 200 });

/**
 * A fetch double. `handler(callIndex, url, init)` returns a Response, a promise of one, or throws.
 * Every call is recorded with the fake-clock time it started at.
 */
export function fetchDouble(handler) {
	const calls = [];
	const fn = (url, init) => {
		const call = { url, init, at: Date.now(), index: calls.length };
		calls.push(call);
		try {
			return Promise.resolve(handler(call.index, url, init));
		} catch (err) {
			return Promise.reject(err);
		}
	};
	return { fn, calls };
}

/** Resolves after `ms` of fake time with `value`; rejects when `signal` aborts first. */
export function after(ms, value, signal) {
	return new Promise((resolve, reject) => {
		const id = setTimeout(() => resolve(value), ms);
		signal?.addEventListener('abort', () => {
			clearTimeout(id);
			reject(new DOMException('aborted', 'AbortError'));
		});
	});
}

/** A response that never arrives; rejects when the signal aborts, as a real fetch does. */
export const hang = (init) =>
	new Promise((_resolve, reject) => {
		// A real fetch rejects with the signal's reason.
		init?.signal?.addEventListener('abort', () => reject(init.signal.reason));
	});

/**
 * Load a check script fresh, at the fake clock, with `fetch` doubled. The script runs on
 * import and exports `done`, a promise of its exit code that never rejects.
 */
export async function load(scriptFile, fn) {
	vi.resetModules();
	vi.stubGlobal('fetch', fn);
	process.exitCode = undefined;
	return import(/* @vite-ignore */ `./${scriptFile}`);
}

/** Capture what the script prints. */
export function captureLogs() {
	const lines = [];
	const push = (...a) => lines.push(a.map(String).join(' '));
	const spies = ['log', 'error', 'warn', 'info'].map((k) => vi.spyOn(console, k).mockImplementation(push));
	return { text: () => lines.join('\n'), restore: () => spies.forEach((s) => s.mockRestore()) };
}
