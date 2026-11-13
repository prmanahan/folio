// Test double for the GitHub API at the `fetch` boundary. Loaded into the
// fetch script's child process with `node --import`, so the script runs
// unmodified through its CLI entry. Not a fixture and not product code.
//
// Reads a scenario file (PROJECTS_FETCH_SCENARIO) and appends one JSON line
// per event to a log file (PROJECTS_FETCH_LOG): a `boot` line when loaded,
// then one `request` line per fetch call.
//
// Scenario shape:
//   { routes: { "<origin><path>": { responses: [ResponseSpec, ...], repeatLast?: true } } }
//   ResponseSpec: { status?, headers?, body? (JSON value), rawBody? (string), networkError?, bodyError?,
//                   hang?, outAsDir? }
//   bodyError: the response's body stream errors when read (a body-read failure).
//   hang: the request never answers; it rejects with init.signal.reason when the signal aborts,
//         and holds the event loop open until then (so a missing signal is a real hang).
//   outAsDir: just before answering, the output path is created as a non-empty directory.
//   Scenario-level `failRename: true`: fs.renameSync throws EXDEV after logging.
// The boot line carries `outExists` (the preload runs after the seed is written, so a seeded run
// reads true). The preload also logs: `timeout` {ms} per AbortSignal.timeout call (the real timer
// is shortened to 50 ms), `write` {path} per fs.writeFileSync and `rename` {from,to} per
// fs.renameSync. Each `request` line carries `hasSignal` and `outExists`: whether <cwd>/frontend/static/projects.json
// existed when the request was made (the run's cwd is the temp tree root).
// The nth request to a route (query string ignored) gets responses[n]. A route
// with no response left, or an unscripted route, answers 404.
import fs, { existsSync, mkdirSync, openSync, readFileSync, writeSync } from 'node:fs';
import { syncBuiltinESMExports } from 'node:module';
import { join } from 'node:path';

const scenario = JSON.parse(readFileSync(process.env.PROJECTS_FETCH_SCENARIO, 'utf8'));
const logPath = process.env.PROJECTS_FETCH_LOG;
const counts = new Map();

const OUT = join(process.cwd(), 'frontend', 'static', 'projects.json');
// writeSync on an fd, not appendFileSync: appendFileSync calls fs.writeFileSync, which is wrapped below.
const logFd = openSync(logPath, 'a');
const log = (entry) => writeSync(logFd, `${JSON.stringify(entry)}\n`);
log({ kind: 'boot', outExists: existsSync(OUT) });

const realTimeout = AbortSignal.timeout.bind(AbortSignal);
AbortSignal.timeout = (ms) => {
	log({ kind: 'timeout', ms });
	return realTimeout(Math.min(ms, 50));
};

const realRename = fs.renameSync;
const realWrite = fs.writeFileSync;
fs.writeFileSync = (path, ...rest) => {
	log({ kind: 'write', path: String(path) });
	return realWrite(path, ...rest);
};
fs.renameSync = (from, to) => {
	log({ kind: 'rename', from: String(from), to: String(to) });
	if (scenario.failRename) throw Object.assign(new Error('SENTINEL-RENAME-FAILURE'), { code: 'EXDEV' });
	return realRename(from, to);
};
syncBuiltinESMExports();

function respond(spec) {
	const headers = new Headers(spec.headers ?? {});
	if (spec.bodyError) {
		const stream = new ReadableStream({
			start(c) {
				c.error(new Error('SENTINEL-BODY-READ-FAILURE'));
			},
		});
		return new Response(stream, { status: spec.status ?? 200, headers });
	}
	const text = spec.rawBody !== undefined ? spec.rawBody : JSON.stringify(spec.body ?? null);
	if (!headers.has('content-type')) headers.set('content-type', 'application/json; charset=utf-8');
	const status = spec.status ?? 200;
	const nullBody = status === 204 || status === 304;
	return new Response(nullBody ? null : text, { status, headers });
}

async function doubleFetch(input, init = {}) {
	const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
	const redirect = init.redirect ?? 'follow';
	const headers = new Headers(init.headers ?? {});
	log({ kind: 'request', url, redirect, authorization: headers.get('authorization'), method: init.method ?? 'GET', hasSignal: init.signal instanceof AbortSignal, outExists: existsSync(OUT) });

	const parsed = new URL(url);
	const key = parsed.origin + parsed.pathname;
	const route = scenario.routes[key];
	if (!route) return respond({ status: 404, body: { message: 'not scripted' } });
	const n = counts.get(key) ?? 0;
	counts.set(key, n + 1);
	const index = route.repeatLast ? Math.min(n, route.responses.length - 1) : n;
	const spec = route.responses[index];
	if (!spec) return respond({ status: 404, body: { message: 'double exhausted' } });
	if (spec.networkError) throw new TypeError('fetch failed');
	if (spec.hang) {
		return new Promise((_, reject) => {
			const keepAlive = setTimeout(() => {}, 60_000);
			init.signal?.addEventListener('abort', () => {
				clearTimeout(keepAlive);
				reject(init.signal.reason);
			});
		});
	}
	if (spec.outAsDir) {
		mkdirSync(OUT);
		writeSync(openSync(join(OUT, 'keep'), 'w'), 'x');
	}

	const status = spec.status ?? 200;
	if (status >= 300 && status < 400) {
		if (redirect === 'error') throw new TypeError('fetch failed');
		if (redirect === 'follow') {
			const location = new URL(new Headers(spec.headers ?? {}).get('location'), url).href;
			return doubleFetch(location, init);
		}
	}
	return respond(spec);
}

globalThis.fetch = doubleFetch;
