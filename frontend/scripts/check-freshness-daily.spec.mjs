// @vitest-environment node
//
// Tests for frontend/scripts/check-freshness-daily.mjs (folio #4853, plan T13;
// spec docs/specs/2026-11-18-live-lists-and-blog-move.md R-31, R-32 and the
// "Daily check, judged on its second read" scenarios).
//
// The script reads no environment, so the only double is `fetch`; time is vitest's fake
// clock (the 150 s wait). The script runs on import and exports `done`, its exit code.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	H48_MS,
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

const SCRIPT = 'check-freshness-daily.mjs';
const WAIT_MS = 150_000;
const READ_TIMEOUT_MS = 30_000;

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

/** Run the script to its end against `handler`; returns the exit code and the calls. */
async function run(handler) {
	const dbl = fetchDouble(handler);
	const mod = await load(SCRIPT, dbl.fn);
	await vi.advanceTimersByTimeAsync(WAIT_MS + 3 * READ_TIMEOUT_MS);
	const code = await mod.done;
	return { code, calls: dbl.calls, exitCode: process.exitCode };
}

/** A second read whose times are `offsetMs` from the instant the read is judged. */
const second = (handler2) => (i, _url, init) => (i === 0 ? ok(body(T0, T0, T0)) : handler2(init));
/** Both list times at `now + offset`, where `now` is the fake clock at the second read. */
const timesAt = (offsetMs) => () => {
	const now = Date.now();
	return ok(body(now + offsetMs, now + offsetMs, now - 1000));
};

describe('R-31: what the script reads, and how', () => {
	it('reads exactly https://peter.manahan.io/api/lists/freshness, as a string', async () => {
		const { calls } = await run(timesAt(0));
		expect(calls.length).toBeGreaterThan(0);
		for (const c of calls) expect(c.url).toBe(URL_FRESHNESS);
	});

	it('passes fetch an init object with exactly the keys redirect and signal, redirect "manual"', async () => {
		const { calls } = await run(timesAt(0));
		for (const c of calls) {
			expect(Object.keys(c.init).sort()).toEqual(['redirect', 'signal']);
			expect(c.init.redirect).toBe('manual');
			expect(c.init.signal).toBeInstanceOf(AbortSignal);
		}
	});

	it('gives each read a 30 s timeout: the signal is live at 29.999 s and aborted at 30 s', async () => {
		const dbl = fetchDouble((_i, _u, init) => hang(init));
		const mod = await load(SCRIPT, dbl.fn);
		await vi.advanceTimersByTimeAsync(READ_TIMEOUT_MS - 1);
		expect(dbl.calls.length, 'the script made a read').toBeGreaterThan(0);
		expect(dbl.calls[0].init.signal.aborted).toBe(false);
		await vi.advanceTimersByTimeAsync(1);
		expect(dbl.calls[0].init.signal.aborted).toBe(true);
		await vi.advanceTimersByTimeAsync(WAIT_MS + READ_TIMEOUT_MS);
		await mod.done;
	});

	it('a 301 reads red (the read is made with redirect: manual, so the 301 is the answer)', async () => {
		const r = await run(second(() => new Response(null, { status: 301, headers: { location: 'https://x.test/' } })));
		expect(r.code).not.toBe(0);
		expect(r.exitCode).not.toBe(0);
	});

	it('a rejected fetch makes the process exit non-zero, and the error is printed', async () => {
		const r = await run(() => {
			throw new Error('boom-connect-refused');
		});
		expect(r.exitCode).toBe(1);
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('boom-connect-refused');
	});
});

describe('R-32: two reads, the second judged', () => {
	it('reads exactly twice, the second 150 s after the first settled', async () => {
		const r = await run(timesAt(0));
		expect(r.calls).toHaveLength(2);
		expect(r.calls[1].at - r.calls[0].at).toBe(WAIT_MS);
	});

	it('starts the wait when the first read settles, not when it started', async () => {
		const dbl = fetchDouble((i, _u, init) => (i === 0 ? after(20_000, ok(body(T0, T0, T0)), init.signal) : timesAt(0)()));
		const mod = await load(SCRIPT, dbl.fn);
		await vi.advanceTimersByTimeAsync(20_000 + WAIT_MS - 1);
		expect(dbl.calls).toHaveLength(1);
		await vi.advanceTimersByTimeAsync(1);
		expect(dbl.calls).toHaveLength(2);
		await vi.advanceTimersByTimeAsync(READ_TIMEOUT_MS);
		await mod.done;
	});

	it('green: 200, exactly the shape, both times inside the window', async () => {
		const r = await run(timesAt(-1000));
		expect(r.code).toBe(0);
		expect(r.exitCode ?? 0).toBe(0);
	});

	it('the first read is not judged: a first read that is red does not make the check red', async () => {
		const r = await run((i) => (i === 0 ? new Response('down', { status: 503 }) : timesAt(0)()));
		expect(r.code).toBe(0);
	});

	it('the first read is not judged: a first read that is green does not save a red second', async () => {
		const r = await run((i) => (i === 0 ? ok(body(T0, T0, T0)) : new Response('down', { status: 503 })));
		expect(r.code).toBe(1);
	});

	it('the first read times out and the second is green: green', async () => {
		const r = await run((i, _u, init) => (i === 0 ? hang(init) : timesAt(0)()));
		expect(r.code).toBe(0);
		expect(r.calls).toHaveLength(2);
		expect(r.calls[1].at - r.calls[0].at).toBe(READ_TIMEOUT_MS + WAIT_MS);
	});
});

describe('the second read, red cases', () => {
	const red = (name, handler2) =>
		it(`red: ${name}`, async () => {
			const r = await run(second(handler2));
			expect(r.code).toBe(1);
			expect(r.exitCode).toBe(1);
		});

	red('the read throws', () => {
		throw new Error('socket hang up');
	});
	red('the read times out', (init) => hang(init));
	red('non-200 (500)', () => new Response('{}', { status: 500 }));
	red('a redirect (302)', () => new Response(null, { status: 302 }));
	red('a 204 with no body', () => new Response(null, { status: 204 }));
	red('a body that is not JSON', () => ok('<html>not json</html>'));
	red('JSON null', () => ok('null'));
	red('a JSON array', () => ok('[]'));
	red('an empty object', () => ok('{}'));
	red('a missing key (no articles)', () => {
		const now = Date.now();
		return ok(JSON.stringify({ projects: iso(now), started_at: iso(now - 1000) }));
	});
	red('an extra key', () => {
		const now = Date.now();
		return ok(JSON.stringify({ projects: iso(now), articles: iso(now), started_at: iso(now - 1), extra: 1 }));
	});
	red('a non-string time (a number)', () => {
		const now = Date.now();
		return ok(JSON.stringify({ projects: now, articles: iso(now), started_at: iso(now - 1) }));
	});
	red('a time outside the grammar ("2026")', () => {
		const now = Date.now();
		return ok(JSON.stringify({ projects: '2026', articles: iso(now), started_at: iso(now - 1) }));
	});
	red('a time with no milliseconds', () => {
		const now = Date.now();
		return ok(JSON.stringify({ projects: '2026-12-01T12:02:30Z', articles: iso(now), started_at: iso(now - 1) }));
	});
	red('a time with an offset instead of Z', () => {
		const now = Date.now();
		return ok(JSON.stringify({ projects: '2026-12-01T12:02:30.000+00:00', articles: iso(now), started_at: iso(now - 1) }));
	});
	red('a time that is not a valid instant (Feb 30)', () => {
		const now = Date.now();
		return ok(JSON.stringify({ projects: '2026-02-30T00:00:00.000Z', articles: iso(now), started_at: iso(now - 1) }));
	});
	red('started_at missing its time (a non-string)', () => {
		const now = Date.now();
		return ok(JSON.stringify({ projects: iso(now), articles: iso(now), started_at: null }));
	});
	red('projects null', () => {
		const now = Date.now();
		return ok(body(null, now, now - 1));
	});
	red('articles null', () => {
		const now = Date.now();
		return ok(body(now, null, now - 1));
	});
	red('projects more than 48 h old', () => {
		const now = Date.now();
		return ok(body(now - H48_MS - 1, now, now - 1));
	});
	red('articles more than 48 h old', () => {
		const now = Date.now();
		return ok(body(now, now - H48_MS - 1, now - 1));
	});
	red('projects more than the skew allowance ahead', () => {
		const now = Date.now();
		return ok(body(now + SKEW_MS + 1, now, now - 1));
	});
	red('articles more than the skew allowance ahead', () => {
		const now = Date.now();
		return ok(body(now, now + SKEW_MS + 1, now - 1));
	});
});

describe('the window is inclusive at both ends, judged at the second read', () => {
	it('a list time exactly 48 h old is green (both lists)', async () => {
		const r = await run(second(() => {
			const now = Date.now();
			return ok(body(now - H48_MS, now - H48_MS, now - 1));
		}));
		expect(r.code).toBe(0);
	});

	it('a list time exactly the skew allowance ahead is green (both lists)', async () => {
		const r = await run(second(() => {
			const now = Date.now();
			return ok(body(now + SKEW_MS, now + SKEW_MS, now - 1));
		}));
		expect(r.code).toBe(0);
	});

	it('one list at the lower bound and the other at the upper bound is green', async () => {
		const r = await run(second(() => {
			const now = Date.now();
			return ok(body(now - H48_MS, now + SKEW_MS, now - 1));
		}));
		expect(r.code).toBe(0);
	});

	it('is judged against the clock when the second read settles, not when the script started', async () => {
		// 48 h before the START is already 150 s outside the window by the second read.
		const r = await run(second(() => ok(body(T0 - H48_MS, T0, T0 - 1))));
		expect(r.code).toBe(1);
	});

	it('the skew allowance is 5 minutes, measured against the same clock', async () => {
		expect(SKEW_MS).toBe(300_000);
		const r = await run(second(() => ok(body(Date.now() + 300_000, Date.now(), Date.now() - 1))));
		expect(r.code).toBe(0);
	});
});

describe('the log on red', () => {
	it('holds the response body for a non-200', async () => {
		const r = await run(second(() => new Response('upstream said NO-SUCH-LIST', { status: 502 })));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('upstream said NO-SUCH-LIST');
	});

	it('holds the response body when the body breaks the shape', async () => {
		const r = await run(second(() => ok('{"projects":"2026"}')));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain(JSON.stringify('{"projects":"2026"}'));
	});

	it('holds the response body when a time is outside the window', async () => {
		const b = body(T0 - H48_MS - 10_000_000, T0, T0 - 1);
		const r = await run(second(() => ok(b)));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain(JSON.stringify(b));
	});

	it('holds the error for a thrown fetch', async () => {
		const r = await run(second(() => {
			throw new Error('ECONNRESET-marker');
		}));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('ECONNRESET-marker');
	});
});

describe('review fixes: what a red prints', () => {
	const lines = () => logs.text().split('\n');

	it('a failed fetch prints the cause chain, not only "fetch failed"', async () => {
		const r = await run(second(() => {
			throw new TypeError('fetch failed', { cause: new Error('connect ECONNREFUSED') });
		}));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('connect ECONNREFUSED');
	});

	it('a timeout is aborted with a named reason, and the log carries it', async () => {
		const dbl = fetchDouble((i, _u, init) => (i === 0 ? ok(body(T0, T0, T0)) : hang(init)));
		const mod = await load(SCRIPT, dbl.fn);
		await vi.advanceTimersByTimeAsync(WAIT_MS + READ_TIMEOUT_MS);
		await mod.done;
		expect(dbl.calls[1].init.signal.reason).toBeInstanceOf(Error);
		expect(dbl.calls[1].init.signal.reason.message).toBe('read timed out after 30 s');
		expect(logs.text()).toContain('read timed out after 30 s');
	});

	it('an empty-body 301 prints the reason, the status and the Location', async () => {
		const r = await run(second(() => new Response(null, { status: 301, headers: { location: 'https://elsewhere.test/x' } })));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('RED: status 301');
		expect(logs.text()).toContain('status: 301');
		expect(logs.text()).toContain(`location: ${JSON.stringify('https://elsewhere.test/x')}`);
	});

	it('a body carrying a workflow command produces no output line starting with ::', async () => {
		const r = await run(second(() => new Response('x\n::error::injected\n::set-output name=a::b', { status: 502 })));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('error::injected');
		for (const l of lines()) expect(l.startsWith('::'), l).toBe(false);
	});

	it('an error text and a Location carrying a workflow command produce no :: line', async () => {
		const r1 = await run(second(() => {
			throw new Error('bad\n::error::from-error');
		}));
		expect(r1.code).toBe(1);
		expect(logs.text()).toContain('from-error');
		for (const l of lines()) expect(l.startsWith('::'), l).toBe(false);
	});

	it('a Location carrying a workflow command produces no :: line', async () => {
		const r = await run(second(() => new Response(null, { status: 302, headers: { location: '/x%0A::error::loc' } })));
		expect(r.code).toBe(1);
		for (const l of lines()) expect(l.startsWith('::'), l).toBe(false);
	});

	it('a printed body is cut to 2048 characters', async () => {
		const r = await run(second(() => new Response('B'.repeat(5000), { status: 502 })));
		expect(r.code).toBe(1);
		expect(logs.text()).toContain('B'.repeat(2048));
		expect(logs.text()).not.toContain('B'.repeat(2049));
	});
});
