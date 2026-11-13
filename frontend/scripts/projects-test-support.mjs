// Shared harness for the fetch-script acceptance tests (black-box).
//
// The script under test is copied into a temp tree laid out like the repo
// (`<tmp>/frontend/scripts/*.mjs`, `<tmp>/frontend/static/`), then run as a
// child process through its CLI entry with cwd `<tmp>`, the way CI runs it
// (`node frontend/scripts/fetch-projects.mjs`). The GitHub API is replaced at
// the `fetch` boundary by projects-fetch-double.mjs. The output path is
// `<tmp>/frontend/static/projects.json`.
import { spawnSync } from 'node:child_process';
import {
	chmodSync,
	copyFileSync,
	existsSync,
	mkdirSync,
	mkdtempSync,
	readdirSync,
	realpathSync,
	statSync,
	readFileSync,
	rmSync,
	writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const here = dirname(fileURLToPath(import.meta.url));
export const repoRoot = join(here, '..', '..');
export const fixtureRoot = join(here, 'projects-fixtures');
export const SCRIPT_NAME = 'fetch-projects.mjs';
export const TOKEN = 'SENTINEL-TOKEN-DO-NOT-LEAK-4726';

const DOUBLE = join(here, 'projects-fetch-double.mjs');
const NOT_SCRIPT = (f) =>
	!f.endsWith('.mjs') ||
	f.startsWith('projects-') ||
	f.startsWith('check-csp') ||
	f === 'csp-lib.mjs' ||
	/\.(spec|test)\.mjs$/.test(f);

const temps = [];

/** Remove every temp tree runFetch created (restoring write access first). Call from afterAll. */
export function cleanupTemps() {
	while (temps.length > 0) {
		const tmp = temps.pop();
		try {
			chmodSync(join(tmp, 'frontend', 'static'), 0o755);
		} catch {
			// already gone or never made
		}
		rmSync(tmp, { recursive: true, force: true });
	}
}

export const MNEMRA = 'https://api.github.com/orgs/mnemra/repos';
export const PRMANAHAN = 'https://api.github.com/users/prmanahan/repos';

/** A GitHub-shaped repo element that passes every R-4 clause. */
export function repo(owner, name, over = {}) {
	return {
		id: 1,
		name,
		full_name: `${owner}/${name}`,
		private: false,
		html_url: `https://github.com/${owner}/${name}`,
		fork: false,
		archived: false,
		description: null,
		owner: { login: owner, type: 'Organization' },
		...over,
	};
}

/** One scripted page; `next` becomes a Link header. */
export function page(body, next = null, extra = {}) {
	const headers = next ? { link: `<${next}>; rel="next", <${next}>; rel="last"` } : {};
	return { status: 200, headers, body, ...extra };
}

/** Scenario with one single-page route per owner. */
export function simple(mnemraRepos, prmanahanRepos) {
	return {
		routes: {
			[MNEMRA]: { responses: [page(mnemraRepos)] },
			[PRMANAHAN]: { responses: [page(prmanahanRepos)] },
		},
	};
}

function extractNext(link) {
	return /<([^>]+)>;\s*rel="next"/.exec(link)[1];
}

/** The committed shared API fixture, as a scenario. */
export function sharedScenario() {
	const api = JSON.parse(readFileSync(join(fixtureRoot, 'api-responses.json'), 'utf8'));
	const pages = (owner) => api[owner].pages.map((p) => page(p.body, p.link ? extractNext(p.link) : null));
	return { routes: { [MNEMRA]: { responses: pages('mnemra') }, [PRMANAHAN]: { responses: pages('prmanahan') } } };
}

export function readExpected() {
	return JSON.parse(readFileSync(join(fixtureRoot, 'list-expected.json'), 'utf8'));
}

function safeJson(t) {
	try {
		return JSON.parse(t);
	} catch {
		return undefined;
	}
}

/**
 * Run the fetch script as a child process. Never reads or mutates process.env
 * beyond PATH: the child gets an explicit environment, so tests are
 * order-independent.
 *
 * opts.token: string | null (null = GITHUB_TOKEN absent). Default: TOKEN.
 * opts.seed:  write a valid-looking file at the output path first. Default true.
 * opts.readOnlyStatic: make the static directory read-only (0555) before the run.
 * opts.breakTable: overwrite the temp copy of invisible-table.mjs with a syntax error.
 * opts.spawnTimeout: ms before the child is killed (default 20000).
 */
export function runFetch(scenario, opts = {}) {
	if (!existsSync(join(here, SCRIPT_NAME))) {
		throw new Error(`fetch script missing: ${join(here, SCRIPT_NAME)} does not exist`);
	}
	const tmp = mkdtempSync(join(tmpdir(), 'projects-fetch-'));
	temps.push(tmp);
	const scriptsDir = join(tmp, 'frontend', 'scripts');
	const staticDir = join(tmp, 'frontend', 'static');
	mkdirSync(scriptsDir, { recursive: true });
	mkdirSync(staticDir, { recursive: true });
	for (const f of readdirSync(here)) {
		if (!NOT_SCRIPT(f)) copyFileSync(join(here, f), join(scriptsDir, f));
	}
	const outPath = join(realpathSync(staticDir), 'projects.json');
	const scenarioPath = join(tmp, 'scenario.json');
	const logPath = join(tmp, 'requests.log');
	if (opts.breakTable) writeFileSync(join(scriptsDir, 'invisible-table.mjs'), 'export const INVISIBLE_RANGES = ;\n');
	writeFileSync(scenarioPath, JSON.stringify(scenario));
	writeFileSync(logPath, '');
	const seeded = JSON.stringify({ fetched_at: '2000-01-01T00:00:00.000Z', groups: [] });
	if (opts.seed !== false) writeFileSync(join(staticDir, 'projects.json'), seeded);

	if (opts.readOnlyStatic) chmodSync(staticDir, 0o555);

	const env = {
		PATH: process.env.PATH,
		PROJECTS_FETCH_SCENARIO: scenarioPath,
		PROJECTS_FETCH_LOG: logPath,
	};
	const token = opts.token === undefined ? TOKEN : opts.token;
	if (token !== null) env.GITHUB_TOKEN = token;

	const startedAt = Date.now();
	const r = spawnSync(
		process.execPath,
		['--import', pathToFileURL(DOUBLE).href, join('frontend', 'scripts', SCRIPT_NAME)],
		{ cwd: tmp, env, encoding: 'utf8', timeout: opts.spawnTimeout ?? 20000 },
	);
	const finishedAt = Date.now();

	const logLines = readFileSync(logPath, 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l));
	const fileExists = existsSync(join(staticDir, 'projects.json'));
	const isFile = fileExists && statSync(join(staticDir, 'projects.json')).isFile();
	const staticListing = readdirSync(staticDir).sort();
	const fileText = isFile ? readFileSync(join(staticDir, 'projects.json'), 'utf8') : null;
	return {
		status: r.status,
		signal: r.signal,
		stdout: r.stdout ?? '',
		stderr: r.stderr ?? '',
		output: `${r.stdout ?? ''}\n${r.stderr ?? ''}`,
		requests: logLines.filter((l) => l.kind === 'request'),
		booted: logLines.some((l) => l.kind === 'boot'),
		bootLine: logLines.find((l) => l.kind === 'boot'),
		timeouts: logLines.filter((l) => l.kind === 'timeout'),
		writes: logLines.filter((l) => l.kind === 'write'),
		renames: logLines.filter((l) => l.kind === 'rename'),
		outPath,
		pid: r.pid,
		fileExists,
		staticListing,
		tmp,
		fileText,
		file: isFile ? safeJson(fileText) : null,
		seeded,
		startedAt,
		finishedAt,
		token,
	};
}

/** Requests to one route (origin + path, query ignored). */
export function requestsTo(result, routeUrl) {
	return result.requests.filter((q) => {
		const u = new URL(q.url);
		return u.origin + u.pathname === routeUrl;
	});
}
