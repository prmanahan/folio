// Post-deploy freshness check (spec docs/specs/2026-11-18-live-lists-and-blog-move.md, R-31, R-33).
//
// Polls https://peter.manahan.io/api/lists/freshness. A read starts every 10 s from the first read's
// start, and none starts at or after 150 s. Reads may overlap. Each read is judged against its own
// started_at: green when it answers 200 with exactly the freshness shape, and both list times are
// later than that started_at and no later than now + skew (inclusive). The check is green on the
// first read that settles green and makes no further read. It is red once 150 s have passed, no read
// is in flight, and none settled green: the last settled body or error is printed, exit code 1.
//
// A control, so it is held small: no imports, no environment, no dependency. R-35's tests scan
// this file's source for what it may import and what it may mention.
//
// The twin of check-freshness-daily.mjs. The two share no module on purpose, since the import
// allowlist would forbid one; the shared pieces below are kept identical by hand.

const URL_FRESHNESS = 'https://peter.manahan.io/api/lists/freshness';
const READ_TIMEOUT_MS = 30_000;
const CADENCE_MS = 10_000;
const WINDOW_MS = 150_000;
// One value with the server's ahead-of-now bound (MAX_AGE_AHEAD in libs/site-core/projects_list.rs);
// a test fails when they differ (R-34).
const SKEW_MINUTES = 5;
const SKEW_MS = SKEW_MINUTES * 60_000;

const TIME_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/;
const KEYS = ['articles', 'projects', 'started_at'];

/** A time in the fetched_at grammar that is a real instant: its epoch ms, else null. */
function instant(value) {
	if (typeof value !== 'string' || !TIME_RE.test(value)) return null;
	const ms = Date.parse(value);
	if (Number.isNaN(ms) || new Date(ms).toISOString() !== value) return null;
	return ms;
}

/** One read: { text } when a 200 came back, { error } for anything else. Never throws. */
async function read(controller) {
	const timer = setTimeout(() => controller.abort(), READ_TIMEOUT_MS);
	try {
		const response = await fetch(URL_FRESHNESS, { redirect: 'manual', signal: controller.signal });
		const text = await response.text();
		if (response.status !== 200) return { error: `status ${response.status}`, text };
		return { text };
	} catch (err) {
		return { error: String(err) };
	} finally {
		clearTimeout(timer);
	}
}

/** The reason a read is red, or null when it is green. `now` is read when the read settles. */
function judge(result, now) {
	if (result.error !== undefined) return result.error;
	let parsed;
	try {
		parsed = JSON.parse(result.text);
	} catch {
		return 'body is not JSON';
	}
	if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) return 'body is not an object';
	if (Object.keys(parsed).sort().join() !== KEYS.join()) return 'body keys are not exactly projects, articles, started_at';
	const startedAt = instant(parsed.started_at);
	if (startedAt === null) return 'started_at is not a time in the fetched_at grammar';
	for (const name of ['projects', 'articles']) {
		const value = parsed[name];
		if (value === null) return `${name} has no time`;
		const ms = instant(value);
		if (ms === null) return `${name} is not a time in the fetched_at grammar`;
		if (ms <= startedAt) return `${name} is not later than started_at`;
		if (ms > now + SKEW_MS) return `${name} is more than ${SKEW_MINUTES} minutes ahead of now`;
	}
	return null;
}

function poll() {
	return new Promise((resolve) => {
		const t0 = Date.now();
		const controllers = new Set();
		let started = 0;
		let inFlight = 0;
		let last = null;
		let finished = false;
		let timer;

		const finish = (code) => {
			if (finished) return;
			finished = true;
			clearTimeout(timer);
			for (const c of controllers) c.abort();
			if (code !== 0) {
				console.error('RED: no read settled green within the window');
				if (last !== null) console.error(last.text ?? last.error);
			} else {
				console.log('green: both lists refreshed under this release');
			}
			resolve(code);
		};

		const redIfSpent = () => {
			if (!finished && Date.now() - t0 >= WINDOW_MS && inFlight === 0) finish(1);
		};

		const startRead = () => {
			const controller = new AbortController();
			controllers.add(controller);
			inFlight += 1;
			read(controller)
				.then((result) => {
					controllers.delete(controller);
					inFlight -= 1;
					if (finished) return;
					last = result;
					const reason = judge(result, Date.now());
					if (reason === null) finish(0);
					else redIfSpent();
				})
				// Fail closed: a fault in the judging itself is red, never a silent green or a hang.
				.catch((err) => {
					last = { error: `unexpected ${String(err)}` };
					finish(1);
				});
		};

		const tick = () => {
			if (finished) return;
			if (Date.now() - t0 >= WINDOW_MS) {
				redIfSpent();
				return;
			}
			startRead();
			started += 1;
			timer = setTimeout(tick, Math.max(0, t0 + started * CADENCE_MS - Date.now()));
		};
		tick();
	});
}

export const done = poll()
	.catch((err) => {
		console.error(`RED: unexpected ${String(err)}`);
		return 1;
	})
	.then((code) => {
		process.exitCode = code;
		return code;
	});
