// @vitest-environment node
//
// Tests for frontend/scripts/check-freshness-post-deploy.mjs (folio #4853, plan T13;
// spec docs/specs/2026-11-18-live-lists-and-blog-move.md R-31, R-33 and the
// "Post-deploy check, judged on every read" scenarios).
//
// The script reads no environment, so the only double is `fetch`; time is vitest's fake
// clock (the 150 s window and the 10 s cadence). The script runs on import and exports
// `done`, its exit code.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	SKEW_MS,
	T0,
	URL_FRESHNESS,
	after,
	body,
	captureLogs,
	fetchDouble,
	hang,
	iso,
	load,
	ok,
} from './check-freshness-test-support.mjs';

const SCRIPT = 'check-freshness-post-deploy.mjs';
const WINDOW_MS = 150_000;
const CADENCE_MS = 10_000;
const READ_TIMEOUT_MS = 30_000;
const STALE = () => ok(body(T0 - 5000, T0 - 5000, T0 - 1000)); // times not later than started_at
const FRESH = () => {
	const now = Date.now();
	return ok(body(now - 100, now - 100, now - 5000));
};

let logs;
beforeEach(() => {
	vi.useFakeTimers({ now: T0 });
	logs = captureLogs();
});
afterEach(() => {
	logs.restore();
	vi.useRealTimers();
	vi.unstubAllGlobals();
	process.exitCode = undefined;
});

/** Start the script; the caller advances time. */
async function start(handler) {
	const dbl = fetchDouble(handler);
	const mod = await load(SCRIPT, dbl.fn);
	return { ...dbl, mod };
}

/** Run to the end (long enough for the last read's timeout) and report. */
async function run(handler) {
	const s = await start(handler);
	await vi.advanceTimersByTimeAsync(WINDOW_MS + 2 * READ_TIMEOUT_MS);
	const code = await s.mod.done;
	return { code, calls: s.calls, exitCode: process.exitCode };
}

describe('R-31: what the script reads, and how', () => {
	it('reads exactly https://peter.manahan.io/api/lists/freshness', async () => {
		const { calls } = await run(STALE);
		expect(calls.length).toBeGreaterThan(0);
		for (const c of calls) expect(c.url).toBe(URL_FRESHNESS);
	});

	it('passes fetch an init object with exactly the keys redirect and signal, redirect "manual"', async () => {
		const { calls } = await run(STALE);
		for (const c of calls) {
			expect(Object.keys(c.init).sort()).toEqual(['redirect', 'signal']);
			expect(c.init.redirect).toBe('manual');
			expect(c.init.signal).toBeInstanceOf(AbortSignal);
		}
	});

	it('gives each read a 30 s timeout: live at 29.999 s, aborted at 30 s', async () => {
		const s = await start((_i, _u, init) => hang(init));
		await vi.advanceTimersByTimeAsync(READ_TIMEOUT_MS - 1);
		expect(s.calls.length, 'the script made a read').toBeGreaterThan(0);
		expect(s.calls[0].init.signal.aborted).toBe(false);
		await vi.advanceTimersByTimeAsync(1);
		expect(s.calls[0].init.signal.aborted).toBe(true);
		await vi.advanceTimersByTimeAsync(WINDOW_MS + 2 * READ_TIMEOUT_MS);
		await s.mod.done;
	});

	it('a 301 on every read reads red', async () => {
		const r = await run(() => new Response(null, { status: 301, headers: { location: 'https://x.test/' } }));
		expect(r.code).toBe(1);
		expect(r.exitCode).toBe(1);
	});

	it('a rejected fetch on every read exits non-zero, and the error is printed', async () => {
		const r = await run(() => {
			throw new Error('boom-connect-refused');
		});
		expect(r.exitCode).toBe(1);
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('boom-connect-refused');
	});
});

describe('R-33: the cadence and the window', () => {
	it('reads at 0, 10, ... 140 s from the first read start, and never at or after 150 s', async () => {
		const r = await run(STALE);
		expect(r.calls.map((c) => c.at - T0)).toEqual(Array.from({ length: 15 }, (_, k) => k * CADENCE_MS));
		expect(r.code).toBe(1);
	});

	it('the cadence runs from the first start, so a slow read does not push the next one back', async () => {
		const r = await run((i) => (i === 0 ? after(25_000, STALE()) : STALE()));
		expect(r.calls.map((c) => c.at - T0).slice(0, 4)).toEqual([0, 10_000, 20_000, 30_000]);
	});

	it('reads may overlap: hung reads do not hold up the next start', async () => {
		const s = await start((_i, _u, init) => hang(init));
		await vi.advanceTimersByTimeAsync(25_000);
		expect(s.calls).toHaveLength(3);
		await vi.advanceTimersByTimeAsync(WINDOW_MS + 2 * READ_TIMEOUT_MS);
		await s.mod.done;
	});

	it('red fires only once 150 s have passed and no read is in flight: every read hanging, red at 170 s', async () => {
		const s = await start((_i, _u, init) => hang(init));
		let settled = false;
		s.mod.done.then(() => {
			settled = true;
		});
		await vi.advanceTimersByTimeAsync(169_999);
		expect(settled).toBe(false);
		await vi.advanceTimersByTimeAsync(1);
		expect(settled).toBe(true);
		expect(await s.mod.done).toBe(1);
		expect(s.calls).toHaveLength(15);
	});

	it('not red at 150 s while a read is still in flight', async () => {
		const s = await start((i, _u, init) => (i === 14 ? after(25_000, STALE(), init.signal) : STALE()));
		let settled = false;
		s.mod.done.then(() => {
			settled = true;
		});
		await vi.advanceTimersByTimeAsync(WINDOW_MS + 10_000);
		expect(settled).toBe(false); // read 14 started at 140 s and settles at 165 s
		await vi.advanceTimersByTimeAsync(20_000);
		expect(settled).toBe(true);
		expect(await s.mod.done).toBe(1);
	});

	it('a read started before 150 s that settles green after 150 s is green', async () => {
		const s = await start((i, _u, init) => (i === 14 ? after(20_000, FRESH(), init.signal) : STALE()));
		await vi.advanceTimersByTimeAsync(WINDOW_MS + READ_TIMEOUT_MS);
		expect(await s.mod.done).toBe(0);
		expect(s.calls).toHaveLength(15);
	});
});

describe('R-33: green on the first read that settles green, and no further read', () => {
	it('a green first read is green at once, with no further read', async () => {
		const s = await start(FRESH);
		await vi.advanceTimersByTimeAsync(WINDOW_MS + READ_TIMEOUT_MS);
		expect(await s.mod.done).toBe(0);
		expect(s.calls).toHaveLength(1);
		expect(process.exitCode ?? 0).toBe(0);
	});

	it('stale reads, then a green third read: green, and exactly three reads', async () => {
		const s = await start((i) => (i < 2 ? STALE() : FRESH()));
		await vi.advanceTimersByTimeAsync(WINDOW_MS + READ_TIMEOUT_MS);
		expect(await s.mod.done).toBe(0);
		expect(s.calls.map((c) => c.at - T0)).toEqual([0, 10_000, 20_000]);
	});

	it('goes green on the first read that settles green even if an earlier read is still hung', async () => {
		const s = await start((i, _u, init) => (i === 0 ? hang(init) : FRESH()));
		await vi.advanceTimersByTimeAsync(CADENCE_MS);
		expect(await s.mod.done).toBe(0);
		expect(s.calls).toHaveLength(2);
	});

	it('non-200 reads, then green: green (the earlier reds are not carried)', async () => {
		const s = await start((i) => (i < 5 ? new Response('starting', { status: 503 }) : FRESH()));
		await vi.advanceTimersByTimeAsync(WINDOW_MS + READ_TIMEOUT_MS);
		expect(await s.mod.done).toBe(0);
	});
});

describe('R-33: each read is judged against its own started_at', () => {
	it('red when every read for 150 s shows a list time not later than its own started_at', async () => {
		const r = await run(STALE);
		expect(r.code).toBe(1);
	});

	it('a list time equal to started_at is not later: red', async () => {
		const r = await run(() => ok(body(T0, T0, T0)));
		expect(r.code).toBe(1);
	});

	it('only one list later than started_at is red (articles stale)', async () => {
		const r = await run(() => ok(body(T0 + 1000, T0 - 1, T0)));
		expect(r.code).toBe(1);
	});

	it('only one list later than started_at is red (projects stale)', async () => {
		const r = await run(() => ok(body(T0 - 1, T0 + 1000, T0)));
		expect(r.code).toBe(1);
	});

	it('uses the started_at of the same read, not the first read: a restart between reads must not turn a stale read green', async () => {
		// Each read reports a newer process start. Judged against the FIRST read's started_at
		// (T0 - 5000) the times from read 2 on would pass; against their own they are stale.
		const r = await run((i) => {
			const startedAt = T0 - 5000 + i * 1000;
			return ok(body(startedAt - 1, startedAt - 1, startedAt));
		});
		expect(r.code).toBe(1);
	});

	it('a list time exactly the skew allowance ahead of now, and later than started_at, is green', async () => {
		const s = await start(() => {
			const now = Date.now();
			return ok(body(now + SKEW_MS, now + SKEW_MS, now - 5000));
		});
		await vi.advanceTimersByTimeAsync(READ_TIMEOUT_MS);
		expect(await s.mod.done).toBe(0);
	});

	it('a projects time one ms past the skew allowance ahead of now is red', async () => {
		const r = await run(() => {
			const now = Date.now();
			return ok(body(now + SKEW_MS + 1, now, now - 5000));
		});
		expect(r.code).toBe(1);
	});

	it('an articles time one ms past the skew allowance ahead of now is red', async () => {
		const r = await run(() => {
			const now = Date.now();
			return ok(body(now, now + SKEW_MS + 1, now - 5000));
		});
		expect(r.code).toBe(1);
	});

	it('a read that settles with its times 1 s past the bound (at settle time) is red', async () => {
		const r = await run((_i, _u, init) => {
			const startedNow = Date.now();
			const ahead = SKEW_MS + 21_000;
			return after(20_000, ok(body(startedNow + ahead, startedNow + ahead, startedNow - 5000)), init.signal);
		});
		expect(r.code).toBe(1);
	});

	it('the ahead-of-now bound is judged at the moment the read settles', async () => {
		// A read that takes 20 s: times 5 min + 10 s ahead of the start of the read are within
		// the bound by the time it settles (now has moved on 20 s), and 5 min + 21 s are not.
		const mk = (aheadMs) => (_i, _u, init) => {
			const startedNow = Date.now();
			return after(20_000, ok(body(startedNow + aheadMs, startedNow + aheadMs, startedNow - 5000)), init.signal);
		};
		const s = await start(mk(SKEW_MS + 10_000));
		await vi.advanceTimersByTimeAsync(20_000);
		expect(await s.mod.done).toBe(0);
	});
});

describe('R-33: every kind of bad read is red after 150 s', () => {
	const red = (name, handler) =>
		it(`red: ${name}`, async () => {
			const r = await run(handler);
			expect(r.code).toBe(1);
			expect(r.exitCode).toBe(1);
		});

	red('every read throws', () => {
		throw new Error('socket hang up');
	});
	red('every read times out', (_i, _u, init) => hang(init));
	red('every read is a 500', () => new Response('{}', { status: 500 }));
	red('every read is a 302', () => new Response(null, { status: 302 }));
	red('every read is not JSON', () => ok('<html>maintenance</html>'));
	red('every read has a missing key', () => ok(JSON.stringify({ projects: iso(T0), started_at: iso(T0) })));
	red('every read has an extra key', () => {
		const now = Date.now();
		return ok(JSON.stringify({ projects: iso(now), articles: iso(now), started_at: iso(now - 5000), extra: 1 }));
	});
	red('every read has a non-string time', () => {
		const now = Date.now();
		return ok(JSON.stringify({ projects: now, articles: iso(now), started_at: iso(now - 5000) }));
	});
	red('every read has a time outside the grammar', () => {
		const now = Date.now();
		return ok(JSON.stringify({ projects: '2026', articles: iso(now), started_at: iso(now - 5000) }));
	});
	red('every read has a null list time', () => {
		const now = Date.now();
		return ok(body(null, now, now - 5000));
	});

	it('red also after a mix: a thrown read, a timeout, a 500 and a stale read, with no green', async () => {
		const r = await run((i, _u, init) => {
			if (i % 4 === 0) throw new Error('boom');
			if (i % 4 === 1) return hang(init);
			if (i % 4 === 2) return new Response('x', { status: 500 });
			return STALE();
		});
		expect(r.code).toBe(1);
	});
});

describe('the log on red', () => {
	it('holds the last response body', async () => {
		const r = await run((i) => new Response(`attempt-${i}-body`, { status: 503 }));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('attempt-14-body');
	});

	it('holds the last error', async () => {
		const r = await run((i) => {
			throw new Error(`error-${i}-marker`);
		});
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('error-14-marker');
	});

	it('"last" means the most recently settled read, not the most recently started', async () => {
		// Read 13 starts at 130 s and settles at 155 s; read 14 starts at 140 s and settles at 140 s.
		const r = await run((i, _u, init) => {
			if (i === 13) return after(25_000, new Response('SLOW-13-last-settled', { status: 503 }), init.signal);
			return new Response(`fast-${i}`, { status: 503 });
		});
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('SLOW-13-last-settled');
	});

	it('holds the body when the body is a stale freshness shape', async () => {
		const b = body(T0 - 5000, T0 - 5000, T0 - 1000);
		const r = await run(() => ok(b));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain(JSON.stringify(b));
	});
});

describe('review fixes: what a red prints', () => {
	const lines = () => logs.text().split('\n');

	it('a failed fetch prints the cause chain, not only "fetch failed"', async () => {
		const r = await run(() => {
			throw new TypeError('fetch failed', { cause: new Error('connect ECONNREFUSED') });
		});
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('connect ECONNREFUSED');
	});

	it('a timeout is aborted with a named reason, and the log carries it', async () => {
		const s = await start((_i, _u, init) => hang(init));
		await vi.advanceTimersByTimeAsync(WINDOW_MS + 2 * READ_TIMEOUT_MS);
		await s.mod.done;
		expect(s.calls[0].init.signal.reason).toBeInstanceOf(Error);
		expect(s.calls[0].init.signal.reason.message).toBe('read timed out after 30 s');
		expect(logs.text()).toContain('read timed out after 30 s');
	});

	it('an empty-body 301 prints the reason, the status and the Location, not an empty line', async () => {
		const r = await run(() => new Response(null, { status: 301, headers: { location: 'https://elsewhere.test/x' } }));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('RED: status 301');
		expect(logs.text()).toContain('status: 301');
		expect(logs.text()).toContain(`location: ${JSON.stringify('https://elsewhere.test/x')}`);
	});

	it('an empty-body 503 prints its reason and status', async () => {
		const r = await run(() => new Response(null, { status: 503 }));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('RED: status 503');
		expect(logs.text()).toContain('status: 503');
	});

	it('a body carrying a workflow command produces no output line starting with ::', async () => {
		const r = await run(() => new Response('x\n::error::injected\n::set-output name=a::b', { status: 502 }));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('error::injected');
		for (const l of lines()) expect(l.startsWith('::'), l).toBe(false);
	});

	it('an error text carrying a workflow command produces no :: line', async () => {
		const r = await run(() => {
			throw new Error('bad\n::error::from-error');
		});
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('from-error');
		for (const l of lines()) expect(l.startsWith('::'), l).toBe(false);
	});

	it('a Location carrying a workflow command produces no :: line', async () => {
		const r = await run(() => new Response(null, { status: 302, headers: { location: '/x%0A::error::loc' } }));
		expect(r.code).toBe(1);
		for (const l of lines()) expect(l.startsWith('::'), l).toBe(false);
	});

	it('a printed body is cut to 2048 characters', async () => {
		const r = await run(() => new Response('B'.repeat(5000), { status: 502 }));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('B'.repeat(2048));
		expect(logs.text()).not.toContain('B'.repeat(2049));
	});
});
