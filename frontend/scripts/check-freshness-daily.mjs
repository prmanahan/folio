// Daily freshness check (spec docs/specs/2026-11-18-live-lists-and-blog-move.md, R-31, R-32).
//
// Reads https://peter.manahan.io/api/lists/freshness, waits 150 s, reads again, and judges only the
// second read: green when it answers 200 with exactly the freshness shape and both list times lie
// within [now - 48 h, now + skew], both ends inclusive. Anything else is red: the body or the error
// is printed and the exit code is 1.
//
// A control, so it is held small: no imports, no environment, no dependency. R-35's tests scan
// this file's source for what it may import and what it may mention.
//
// The twin of check-freshness-post-deploy.mjs. The two share no module on purpose, since the import
// allowlist would forbid one; the shared pieces below are kept identical by hand.

const URL_FRESHNESS = 'https://peter.manahan.io/api/lists/freshness';
const READ_TIMEOUT_MS = 30_000;
const WAIT_MS = 150_000;
const MAX_AGE_MS = 48 * 60 * 60 * 1000;
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
async function read() {
	const controller = new AbortController();
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
	if (instant(parsed.started_at) === null) return 'started_at is not a time in the fetched_at grammar';
	for (const name of ['projects', 'articles']) {
		const value = parsed[name];
		if (value === null) return `${name} has no time`;
		const ms = instant(value);
		if (ms === null) return `${name} is not a time in the fetched_at grammar`;
		if (ms < now - MAX_AGE_MS) return `${name} is more than 48 h old`;
		if (ms > now + SKEW_MS) return `${name} is more than ${SKEW_MINUTES} minutes ahead of now`;
	}
	return null;
}

async function main() {
	try {
		const first = await read();
		console.log(`first read (not judged): ${first.error ?? 'answered 200'}`);
		await new Promise((resolve) => setTimeout(resolve, WAIT_MS));
		const second = await read();
		const reason = judge(second, Date.now());
		if (reason === null) {
			console.log('green: both lists are fresh');
			return 0;
		}
		console.error(`RED: ${reason}`);
		console.error(second.text ?? second.error);
		return 1;
	} catch (err) {
		console.error(`RED: unexpected ${String(err)}`);
		return 1;
	}
}

export const done = main().then((code) => {
	process.exitCode = code;
	return code;
});
