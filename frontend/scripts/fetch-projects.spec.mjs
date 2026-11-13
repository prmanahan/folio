// @vitest-environment node
//
// Red-phase acceptance tests for frontend/scripts/fetch-projects.mjs
// (folio #4726, plan T1; spec docs/specs/2026-11-11-projects-from-github.md
// R-1 to R-6, R-8, plus plan D12 and D13).
//
// BLACK-BOX. Every test runs the script as a child process through its CLI
// entry, with the GitHub API replaced at the `fetch` boundary
// (projects-fetch-double.mjs, loaded with `node --import`) and a fake token.
// Assertions observe exit code, stdout, stderr, the output file and the
// double's request log. No function of the script is imported or called.
//
// Sanctioned reads, declared:
//   - frontend/src/lib/invisible-table.ts: read as DATA only, to enumerate the
//     rejected class's invisible ranges (spec Data Model names this file as
//     part of the class). No script code is read; none exists yet.
//   - frontend/scripts/check-csp.test.mjs and csp-fixtures/: house style of the
//     existing script tests.
//
// Seam assumptions the spec does not pin (reported to the dispatcher):
//   1. Output path is `<scripts dir>/../static/projects.json`, found whether
//      the script resolves it from its own location or from a repo-root cwd.
//   2. The token is read from the GITHUB_TOKEN environment variable (R-5).
//   3. The script sends `Authorization` with the token in any scheme.
//
// Not named *.test.mjs on purpose: vitest's include must not collect
// check-csp.test.mjs (R-16).
import { afterAll, describe, expect, it, vi } from 'vitest';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
	MNEMRA,
	PRMANAHAN,
	TOKEN,
	cleanupTemps,
	page,
	readExpected,
	repo,
	repoRoot,
	requestsTo,
	runFetch,
	sharedScenario,
	simple,
} from './projects-test-support.mjs';

vi.setConfig({ testTimeout: 30000 });

afterAll(cleanupTemps);

const CRASH = /ERR_MODULE_NOT_FOUND|Cannot find module|SyntaxError|ReferenceError/;

const okM = () => repo('mnemra', 'fixture-ok-m', { description: 'ok' });
const okP = () => repo('prmanahan', 'fixture-ok-p', { description: 'ok' });

function expectNoTokenLeak(r) {
	expect(r.stdout, 'stdout must not contain the token').not.toContain(TOKEN);
	expect(r.stderr, 'stderr must not contain the token').not.toContain(TOKEN);
	if (r.fileText !== null) expect(r.fileText, 'output file must not contain the token').not.toContain(TOKEN);
}

/**
 * R-2: non-zero exit, no file left (a valid file was seeded), a logged condition, no token leak.
 * `owner` is the owner whose input failed: the failure line must name it, which a validated
 * rejection does (`fetch-projects failed: mnemra: element 1 rejected ...`) and the catch-all
 * for an unexpected crash (`unexpected TypeError`) does not. `TypeError` stays out of CRASH
 * because legitimate network and 3xx failures log `err.name`.
 */
function expectFailClosed(r, owner) {
	expect(r.booted, 'network double must have loaded into the child').toBe(true);
	expect(r.signal).toBeNull();
	expect(r.status, `exit code (output: ${r.output})`).not.toBe(0);
	expect(r.status).not.toBeNull();
	expect(r.fileExists, 'no file may remain at the output path').toBe(false);
	expect(r.output.trim().length, 'a failure must log its condition').toBeGreaterThan(0);
	expect(r.output).not.toMatch(CRASH);
	if (owner !== undefined) {
		expect(r.stderr, `the failure line names ${owner}, not an unexpected crash`).toContain(`fetch-projects failed: ${owner}:`);
	}
	expectNoTokenLeak(r);
}

function expectSuccess(r) {
	expect(r.booted).toBe(true);
	expect(r.signal).toBeNull();
	expect(r.status, `exit code (output: ${r.output})`).toBe(0);
	expect(r.fileExists, 'the script must write the output file').toBe(true);
	expect(r.file, 'the output file must be valid JSON').toBeDefined();
	expectNoTokenLeak(r);
}

describe('R-1: fetch, authenticate, paginate', () => {
	it('a two-page double yields the repos of both pages', () => {
		// Given the shared fixture: mnemra answers in two pages, prmanahan in one
		// When the script runs
		const r = runFetch(sharedScenario());
		// Then both mnemra pages are in the output
		expectSuccess(r);
		const names = r.file.groups.find((g) => g.owner === 'mnemra').repos.map((x) => x.name);
		expect(names).toContain('fixture-zeta'); // page 1
		expect(names).toContain('fixture-a-b'); // page 2
		expect(requestsTo(r, MNEMRA)).toHaveLength(2);
		expect(requestsTo(r, PRMANAHAN)).toHaveLength(1);
	});

	it('requests only the two allowed URLs, with redirect "error" and the token as authorization', () => {
		const r = runFetch(sharedScenario());
		expectSuccess(r);
		expect(r.requests.length).toBeGreaterThan(0);
		for (const q of r.requests) {
			const u = new URL(q.url);
			expect([MNEMRA, PRMANAHAN]).toContain(u.origin + u.pathname);
			expect(q.redirect, `redirect mode of ${q.url}`).toBe('error');
			// positive control for R-5: the token really flowed to the API
			expect(q.authorization ?? '', `authorization of ${q.url}`).toContain(TOKEN);
		}
	});

	it('D13: the first request for the mnemra org endpoint carries type=public', () => {
		const r = runFetch(sharedScenario());
		expectSuccess(r);
		const first = requestsTo(r, MNEMRA)[0];
		expect(first, 'the mnemra org endpoint must be requested').toBeDefined();
		expect(new URL(first.url).searchParams.get('type')).toBe('public');
	});

	it('exactly 10 pages is accepted (boundary)', () => {
		// Given mnemra answers in 10 linked pages of one repo each
		const pages = [];
		for (let i = 1; i <= 10; i++) {
			const next = i < 10 ? `${MNEMRA}?page=${i + 1}` : null;
			pages.push(page([repo('mnemra', `fixture-p${String(i).padStart(2, '0')}`)], next));
		}
		const s = simple([], [okP()]);
		s.routes[MNEMRA] = { responses: pages };
		const r = runFetch(s);
		expectSuccess(r);
		expect(requestsTo(r, MNEMRA)).toHaveLength(10);
		expect(r.file.groups[0].repos).toHaveLength(10);
	});

	it('an 11th page is never fetched and fails the run', () => {
		// Given every mnemra page links to a further page
		const s = simple([], [okP()]);
		s.routes[MNEMRA] = { responses: [page([okM()], `${MNEMRA}?page=2`)], repeatLast: true };
		const r = runFetch(s);
		expectFailClosed(r, 'mnemra');
		expect(requestsTo(r, MNEMRA).length).toBeLessThanOrEqual(10);
		expect(requestsTo(r, MNEMRA).length, 'the 10 allowed pages were fetched').toBe(10);
	});

	it.each([
		['3xx response', { status: 301, headers: { location: `${MNEMRA}?page=2` }, body: [] }],
		['302 to another origin', { status: 302, headers: { location: 'https://evil.example/repos' }, body: [] }],
	])('%s fails the run', (_label, spec) => {
		const s = simple([], [okP()]);
		s.routes[MNEMRA] = { responses: [spec, page([okM()])] };
		const r = runFetch(s);
		expectFailClosed(r, 'mnemra');
		expect(r.requests.every((q) => new URL(q.url).origin === 'https://api.github.com')).toBe(true);
	});

	it.each([
		['another origin', 'https://evil.example/orgs/mnemra/repos?page=2'],
		['http scheme (different origin)', 'http://api.github.com/orgs/mnemra/repos?page=2'],
		['another port (different origin)', 'https://api.github.com:8443/orgs/mnemra/repos?page=2'],
		['another path (the other owner)', `${PRMANAHAN}?page=2`],
		['a longer path', `${MNEMRA}/extra?page=2`],
		['a shorter path', 'https://api.github.com/orgs/mnemra?page=2'],
	])('a next link to %s fails the run and is not followed', (_label, nextUrl) => {
		const s = simple([], [okP()]);
		s.routes[MNEMRA] = { responses: [page([okM()], nextUrl), page([repo('mnemra', 'fixture-trap')])] };
		s.routes[nextUrl.split('?')[0]] ??= { responses: [page([repo('mnemra', 'fixture-trap')])] };
		const r = runFetch(s);
		expectFailClosed(r, 'mnemra');
		expect(requestsTo(r, MNEMRA), 'the bad next link is not followed').toHaveLength(1);
		const nextRoute = nextUrl.split('?')[0];
		const foreign = r.requests.filter((q) => q.url.split('?')[0] === nextRoute);
		expect(foreign, 'no request is sent to the linked URL').toHaveLength(0);
	});

	it.each([
		[
			'more than one rel="next" link',
			`<${MNEMRA}?page=2>; rel="next", <${MNEMRA}?page=3>; rel="next"`,
		],
		['a next link that does not parse as a URL', '<http://>; rel="next"'],
		[
			'a next link with userinfo',
			'<https://x:y@api.github.com/orgs/mnemra/repos?page=2>; rel="next"',
		],
	])('%s fails the run and is not followed', (_label, link) => {
		const s = simple([], [okP()]);
		s.routes[MNEMRA] = {
			responses: [page([okM()], null, { headers: { link } }), page([repo('mnemra', 'fixture-trap')])],
			repeatLast: true,
		};
		const r = runFetch(s);
		expectFailClosed(r, 'mnemra');
		expect(requestsTo(r, MNEMRA), 'no next link is followed').toHaveLength(1);
		expect(requestsTo(r, PRMANAHAN), 'the second owner is never reached').toHaveLength(0);
	});

	it('a next link on the same origin and path is followed even with a different query', () => {
		const s = simple([], [okP()]);
		s.routes[MNEMRA] = {
			responses: [page([okM()], `${MNEMRA}?per_page=100&page=2&cursor=abc`), page([repo('mnemra', 'fixture-second')])],
		};
		const r = runFetch(s);
		expectSuccess(r);
		expect(r.file.groups[0].repos.map((x) => x.name).sort()).toEqual(['fixture-ok-m', 'fixture-second']);
	});

	it('git check-ignore lists frontend/static/projects.json', () => {
		const r = spawnSync('git', ['check-ignore', 'frontend/static/projects.json'], { cwd: repoRoot, encoding: 'utf8' });
		expect(r.status, `git check-ignore stdout=${r.stdout} stderr=${r.stderr}`).toBe(0);
	});

	it('git ls-files prints nothing for frontend/static/projects.json', () => {
		const r = spawnSync('git', ['ls-files', 'frontend/static/projects.json'], { cwd: repoRoot, encoding: 'utf8' });
		expect(r.status).toBe(0);
		expect(r.stdout).toBe('');
	});
});

describe('R-2: fail closed, no file, no retry, no fallback', () => {
	it('a failed request (network error) on the first owner', () => {
		const s = simple([okM()], [okP()]);
		s.routes[MNEMRA] = { responses: [{ networkError: true }] };
		const r = runFetch(s);
		expectFailClosed(r, 'mnemra');
		expect(requestsTo(r, MNEMRA), 'no retry').toHaveLength(1);
	});

	it('a failed request (network error) on the second owner after the first succeeded', () => {
		const s = simple([okM()], [okP()]);
		s.routes[PRMANAHAN] = { responses: [{ networkError: true }] };
		const r = runFetch(s);
		expectFailClosed(r, 'prmanahan');
		expect(requestsTo(r, PRMANAHAN), 'no retry').toHaveLength(1);
	});

	it.each([[401], [403], [404], [500], [503]])('a %i response fails the run, without retry', (status) => {
		const s = simple([okM()], [okP()]);
		s.routes[MNEMRA] = { responses: [{ status, body: { message: 'nope' } }, page([okM()])] };
		const r = runFetch(s);
		expectFailClosed(r, 'mnemra');
		expect(requestsTo(r, MNEMRA), 'no retry').toHaveLength(1);
		expect(r.output, 'the failing status is named').toContain(String(status));
	});

	it('a non-2xx on the second owner leaves no file even though the first owner succeeded', () => {
		const s = simple([okM()], [okP()]);
		s.routes[PRMANAHAN] = { responses: [{ status: 503, body: {} }] };
		expectFailClosed(runFetch(s), 'prmanahan');
	});

	it.each([
		['a JSON object', { rawBody: '{"message":"Bad credentials"}' }],
		['a JSON string', { rawBody: '"fixture-ok"' }],
		['JSON null', { rawBody: 'null' }],
		['a JSON number', { rawBody: '5' }],
		['invalid JSON', { rawBody: '[{"name":' }],
		['HTML', { rawBody: '<html><body>rate limited</body></html>' }],
		['an empty body', { rawBody: '' }],
	])('a 200 whose body is %s (not a JSON array)', (_label, spec) => {
		const s = simple([okM()], [okP()]);
		s.routes[MNEMRA] = { responses: [{ status: 200, ...spec }] };
		expectFailClosed(runFetch(s), 'mnemra');
	});

	it('page 2 of 2 answers 200 with an empty-string body: the truncated list is rejected, not shipped', () => {
		// Given mnemra page 1 is valid and links to page 2, and page 2 is the JSON string ""
		const s = simple([okM()], [okP()]);
		s.routes[MNEMRA] = { responses: [page([okM()], `${MNEMRA}?page=2`), { status: 200, rawBody: '""' }] };
		// When the script runs
		const r = runFetch(s);
		// Then it fails closed naming mnemra, and page 2 was fetched
		expectFailClosed(r, 'mnemra');
		expect(requestsTo(r, MNEMRA)).toHaveLength(2);
	});

	it('the output file is deleted before the first request, as seen from inside the network boundary', () => {
		// Given a valid file at the output path and a run that succeeds
		const r = runFetch(sharedScenario());
		expectSuccess(r);
		// Positive control: the seed was on disk when the double loaded (the preload runs after the seed)
		expect(r.bootLine.outExists, 'seeded file must exist at boot, or "gone at request" proves nothing').toBe(true);
		// Then at every request the file was already gone
		expect(r.requests.length).toBeGreaterThan(0);
		for (const q of r.requests) expect(q.outExists, `output file present at request ${q.url}`).toBe(false);
	});

	it('a non-array body on the second owner', () => {
		const s = simple([okM()], [okP()]);
		s.routes[PRMANAHAN] = { responses: [{ status: 200, rawBody: '{"items":[]}' }] };
		expectFailClosed(runFetch(s), 'prmanahan');
	});

	it('an owner with an empty list', () => {
		expectFailClosed(runFetch(simple([], [okP()])), 'mnemra');
	});

	it('the second owner with an empty list', () => {
		expectFailClosed(runFetch(simple([okM()], [])), 'prmanahan');
	});

	it('an owner whose repos are all excluded (fork, archived, .github) has zero repos', () => {
		const r = runFetch(
			simple(
				[
					repo('mnemra', 'fixture-f', { fork: true }),
					repo('mnemra', 'fixture-a', { archived: true }),
					repo('mnemra', '.github'),
				],
				[okP()],
			),
		);
		expectFailClosed(r, 'mnemra');
	});

	it('a successful run replaces the seeded file', () => {
		const r = runFetch(simple([okM()], [okP()]));
		expectSuccess(r);
		expect(r.fileText).not.toBe(r.seeded);
		expect(r.file.fetched_at).not.toBe('2000-01-01T00:00:00.000Z');
	});
});

describe('D12: GITHUB_TOKEN absent or empty fails closed', () => {
	it.each([
		['absent', null],
		['empty', ''],
	])('with GITHUB_TOKEN %s: non-zero exit, file deleted, zero requests, condition named', (_label, token) => {
		const r = runFetch(sharedScenario(), { token });
		expect(r.booted, 'network double must have loaded into the child').toBe(true);
		expect(r.signal).toBeNull();
		expect(r.status, `exit code (output: ${r.output})`).not.toBe(0);
		expect(r.status).not.toBeNull();
		expect(r.fileExists, 'the seeded file must be deleted').toBe(false);
		expect(r.requests, 'no request may be sent without a token').toHaveLength(0);
		expect(r.output, 'the output names the condition').toMatch(/GITHUB_TOKEN/);
		expect(r.output).not.toMatch(CRASH);
	});
});

describe('R-3: what is listed', () => {
	it('lists a repo whose name appears nowhere else in folio, and not a fork, an archived repo or .github', () => {
		const r = runFetch(
			simple(
				[
					repo('mnemra', 'fixture-unique-only-here-4726'),
					repo('mnemra', 'fixture-a-fork', { fork: true }),
					repo('mnemra', 'fixture-an-archived', { archived: true }),
					repo('mnemra', '.github'),
				],
				[okP(), repo('prmanahan', '.GitHub')],
			),
		);
		expectSuccess(r);
		expect(r.file.groups[0].repos.map((x) => x.name)).toEqual(['fixture-unique-only-here-4726']);
		expect(r.file.groups[1].repos.map((x) => x.name)).toEqual(['fixture-ok-p']);
	});

	it('has no other exclusion: .github-prefixed names, templates, disabled repos and a fork:false/archived:false repo are listed', () => {
		const r = runFetch(
			simple(
				[
					repo('mnemra', '.github.io'),
					repo('mnemra', '.githubx'),
					repo('mnemra', 'fixture-template', { is_template: true }),
					repo('mnemra', 'fixture-disabled', { disabled: true }),
				],
				[okP()],
			),
		);
		expectSuccess(r);
		expect(r.file.groups[0].repos.map((x) => x.name).sort()).toEqual(
			['.github.io', '.githubx', 'fixture-disabled', 'fixture-template'].sort(),
		);
	});

	it('accepts the owner login case-insensitively', () => {
		const r = runFetch(simple([repo('MNEMRA', 'fixture-upper')], [repo('PrManahan', 'fixture-mixed-owner')]));
		expectSuccess(r);
	});
});

describe('R-4: every element is validated, any failure fails the run', () => {
	const failing = (mutate, owner = 'mnemra') => {
		const bad = mutate(repo(owner, 'fixture-bad', { description: 'ok' }));
		return owner === 'mnemra' ? simple([okM(), bad], [okP()]) : simple([okM()], [okP(), bad]);
	};
	const replaceElement = (el) => () => el;

	it.each([
		['null', replaceElement(null)],
		['an array', replaceElement([])],
		['a string', replaceElement('fixture-bad')],
		['a number', replaceElement(5)],
	])('an element that is %s (not a plain object)', (_l, m) => {
		expectFailClosed(runFetch(failing(m)), 'mnemra');
	});

	it.each([
		['private: true', (e) => ({ ...e, private: true })],
		['private: "false" (string)', (e) => ({ ...e, private: 'false' })],
		['private: null', (e) => ({ ...e, private: null })],
		['private missing', ({ private: _p, ...e }) => e],
		['fork: "no" (not boolean)', (e) => ({ ...e, fork: 'no' })],
		['fork: null', (e) => ({ ...e, fork: null })],
		['fork missing', ({ fork: _f, ...e }) => e],
		['archived: 0 (not boolean)', (e) => ({ ...e, archived: 0 })],
		['archived missing', ({ archived: _a, ...e }) => e],
		['owner login of someone else', (e) => ({ ...e, owner: { login: 'someone-else' } })],
		['owner login of the other owner', (e) => ({ ...e, owner: { login: 'prmanahan' } })],
		['owner missing', ({ owner: _o, ...e }) => e],
		['owner login missing', (e) => ({ ...e, owner: {} })],
		['owner login not a string', (e) => ({ ...e, owner: { login: 5 } })],
	])('an element with %s', (_l, m) => {
		expectFailClosed(runFetch(failing(m)), 'mnemra');
	});

	it('a private repo that is also a fork still fails the run (R-4 before R-3)', () => {
		expectFailClosed(runFetch(failing((e) => ({ ...e, private: true, fork: true }))), 'mnemra');
	});

	it('an element failing R-4 on the second owner', () => {
		expectFailClosed(runFetch(failing((e) => ({ ...e, private: true }), 'prmanahan')), 'prmanahan');
	});

	it.each([
		['empty', ''],
		['101 characters', 'a'.repeat(101)],
		['a space', 'has space'],
		['a slash', 'a/b'],
		['a non-ASCII letter', 'bäd'],
		['a trailing newline', 'fixture-x\n'],
		['a tab', 'fixture\tx'],
		['a number', 5],
		['null', null],
		['an array', ['fixture-x']],
	])('a name that is %s', (_l, name) => {
		expectFailClosed(runFetch(failing((e) => ({ ...e, name }))), 'mnemra');
	});

	it('a missing name', () => {
		expectFailClosed(runFetch(failing(({ name: _n, ...e }) => e)), 'mnemra');
	});

	it('a 100-character name with every allowed punctuation is listed (boundary)', () => {
		const name = `fixture-a.b_c-d${'x'.repeat(100 - 'fixture-a.b_c-d'.length)}`;
		expect(name).toHaveLength(100);
		const r = runFetch(simple([repo('mnemra', name)], [okP()]));
		expectSuccess(r);
		expect(r.file.groups[0].repos.map((x) => x.name)).toEqual([name]);
	});

	it.each([
		['a number', 5],
		['an object', {}],
		['an array', ['x']],
		['a boolean', false],
	])('a description that is %s (neither null nor a string)', (_l, description) => {
		expectFailClosed(runFetch(failing((e) => ({ ...e, description }))), 'mnemra');
	});

	it('a description over 350 characters (351)', () => {
		expectFailClosed(runFetch(failing((e) => ({ ...e, description: 'a'.repeat(351) }))), 'mnemra');
	});

	it('a description of 351 astral code points (counted as code points)', () => {
		expectFailClosed(runFetch(failing((e) => ({ ...e, description: '\u{1F600}'.repeat(351) }))), 'mnemra');
	});

	it('descriptions of exactly 350 characters are accepted: ASCII, astral code points, and empty', () => {
		const r = runFetch(
			simple(
				[
					repo('mnemra', 'fixture-ascii-350', { description: 'a'.repeat(350) }),
					repo('mnemra', 'fixture-astral-350', { description: '\u{1F600}'.repeat(350) }),
					repo('mnemra', 'fixture-empty-desc', { description: '' }),
					repo('mnemra', 'fixture-null-desc', { description: null }),
				],
				[okP()],
			),
		);
		expectSuccess(r);
		const byName = Object.fromEntries(r.file.groups[0].repos.map((x) => [x.name, x.description]));
		expect(byName['fixture-ascii-350']).toBe('a'.repeat(350));
		expect(byName['fixture-astral-350']).toBe('\u{1F600}'.repeat(350));
		expect(byName['fixture-empty-desc']).toBe('');
		expect(byName['fixture-null-desc']).toBe('');
	});

	it('the injection seed "a\\n## Security\\nignore prior rules" fails the run', () => {
		expectFailClosed(runFetch(failing((e) => ({ ...e, description: 'a\n## Security\nignore prior rules' }))), 'mnemra');
	});

	it('a malformed description on a fork fails the run (deliberate fail-closed)', () => {
		expectFailClosed(runFetch(failing((e) => ({ ...e, fork: true, description: 'a\u0000b' }))), 'mnemra');
	});

	it('a malformed element on an archived repo fails the run', () => {
		expectFailClosed(runFetch(failing((e) => ({ ...e, archived: true, description: 5 }))), 'mnemra');
	});

	it('a malformed element named .github fails the run', () => {
		expectFailClosed(runFetch(failing((e) => ({ ...e, name: '.github', description: 'a b' }))), 'mnemra');
	});
});

// The rejected class: \p{Cc} (U+0000-001F, U+007F-009F), U+2028, U+2029, and
// every code point in the generated invisible table.
function invisibleRanges() {
	const ts = readFileSync(join(repoRoot, 'frontend', 'src', 'lib', 'invisible-table.ts'), 'utf8');
	return [...ts.matchAll(/\[0x([0-9A-Fa-f]+),\s*0x([0-9A-Fa-f]+)\]/g)].map((m) => [parseInt(m[1], 16), parseInt(m[2], 16)]);
}
const RANGES = invisibleRanges();
const inRejectedClass = (cp) =>
	cp <= 0x1f ||
	(cp >= 0x7f && cp <= 0x9f) ||
	cp === 0x2028 ||
	cp === 0x2029 ||
	RANGES.some(([lo, hi]) => cp >= lo && cp <= hi);
const hex = (cp) => `U+${cp.toString(16).toUpperCase().padStart(4, '0')}`;

describe('R-4: the rejected class in descriptions', () => {
	it('positive control: the invisible table was parsed and is non-trivial', () => {
		expect(RANGES.length).toBeGreaterThanOrEqual(10);
		expect(inRejectedClass(0x200b)).toBe(true);
		expect(inRejectedClass(0xe0041)).toBe(true);
	});

	const cc = [...Array.from({ length: 0x20 }, (_, i) => i), ...Array.from({ length: 0x21 }, (_, i) => 0x7f + i)];
	it.each(cc.map((cp) => [hex(cp), cp]))('a description containing the control character %s', (_l, cp) => {
		const r = runFetch(
			simple([okM(), repo('mnemra', 'fixture-bad', { description: `a${String.fromCodePoint(cp)}b` })], [okP()]),
		);
		expectFailClosed(r, 'mnemra');
	});

	it.each([[0x85], [0x2028], [0x2029]].map(([cp]) => [hex(cp), cp]))(
		'a description containing %s (the line-separator kinds)',
		(_l, cp) => {
			expectFailClosed(
				runFetch(simple([okM(), repo('mnemra', 'fixture-bad', { description: `a${String.fromCodePoint(cp)}b` })], [okP()])),
				'mnemra',
			);
		},
	);

	const seeds = new Set();
	for (const [lo, hi] of RANGES) {
		seeds.add(lo);
		seeds.add(hi);
		if (hi - lo > 2) seeds.add(lo + Math.floor((hi - lo) / 2));
	}
	it.each([...seeds].sort((a, b) => a - b).map((cp) => [hex(cp), cp]))(
		'a description containing the invisible code point %s',
		(_l, cp) => {
			expectFailClosed(
				runFetch(simple([okM(), repo('mnemra', 'fixture-bad', { description: `a${String.fromCodePoint(cp)}b` })], [okP()])),
				'mnemra',
			);
		},
	);

	it('code points just outside the class are accepted and kept exactly', () => {
		// Given a description around each code point adjacent to a rejected range
		const adjacent = new Set([0x20, 0x7e, 0xa0, 0xa1, 0x2010, 0x200a]);
		for (const [lo, hi] of RANGES) {
			adjacent.add(lo - 1);
			adjacent.add(hi + 1);
		}
		for (const cp of [0x1f, 0x7f, 0x9f, 0x2027, 0x202a]) adjacent.delete(cp);
		const allowed = [...adjacent].filter((cp) => cp > 0x1f && !inRejectedClass(cp)).sort((a, b) => a - b);
		expect(allowed.length).toBeGreaterThan(10);
		const repos = allowed.map((cp) =>
			repo('mnemra', `fixture-adj-${cp.toString(16)}`, { description: `a${String.fromCodePoint(cp)}b` }),
		);
		const r = runFetch(simple(repos, [okP()]));
		expectSuccess(r);
		const byName = Object.fromEntries(r.file.groups[0].repos.map((x) => [x.name, x.description]));
		for (const cp of allowed) {
			expect(byName[`fixture-adj-${cp.toString(16)}`], hex(cp)).toBe(`a${String.fromCodePoint(cp)}b`);
		}
	});

	it('ordinary non-ASCII text is accepted', () => {
		const r = runFetch(
			simple([repo('mnemra', 'fixture-intl', { description: 'café 日本語 \u{1F600} — ok' })], [okP()]),
		);
		expectSuccess(r);
		expect(r.file.groups[0].repos[0].description).toBe('café 日本語 \u{1F600} — ok');
	});
});

describe('R-4 parity with the Rust reader (lead decisions, #4726)', () => {
	const rawScenario = (rawMnemra) => {
		const s = simple([okM()], [okP()]);
		s.routes[MNEMRA] = { responses: [{ status: 200, rawBody: rawMnemra }] };
		return s;
	};

	it.each([
		['a lone high surrogate', 'a\ud800b'],
		['a lone low surrogate', 'a\udc00b'],
		['a reversed pair (low then high)', '\udc00\ud800'],
		['a high surrogate at the end', 'abc\ud83d'],
	])('a description containing %s fails the run, naming the owner', (_l, description) => {
		// Given a raw JSON body whose description carries an unpaired surrogate escape
		const raw = JSON.stringify([okM(), repo('mnemra', 'fixture-bad', { description })]);
		expect(raw, 'precondition: the escape is on the wire').toMatch(/\\ud[89a-f][0-9a-f]{2}/i);
		// When the script runs, Then it fails closed naming mnemra
		expectFailClosed(runFetch(rawScenario(raw)), 'mnemra');
	});

	it('a valid surrogate pair in a description is still accepted (control)', () => {
		const raw = JSON.stringify([okM(), repo('mnemra', 'fixture-pair', { description: 'a\ud83d\ude00b' })]);
		const r = runFetch(rawScenario(raw));
		expectSuccess(r);
		const d = r.file.groups[0].repos.find((x) => x.name === 'fixture-pair').description;
		expect(d).toBe('a\u{1F600}b');
	});

	it('the same repo name on two pages of one owner fails the run', () => {
		const s = simple([], [okP()]);
		s.routes[MNEMRA] = {
			responses: [page([okM()], `${MNEMRA}?page=2`), page([repo('mnemra', 'fixture-ok-m', { description: 'dup' })])],
		};
		expectFailClosed(runFetch(s), 'mnemra');
	});

	it('a case-only duplicate (fixture-A and fixture-a) in one owner fails the run', () => {
		const r = runFetch(simple([repo('mnemra', 'fixture-A'), repo('mnemra', 'fixture-a')], [okP()]));
		expectFailClosed(r, 'mnemra');
	});

	it('a case-only duplicate on the second owner fails the run, naming it', () => {
		const r = runFetch(simple([okM()], [repo('prmanahan', 'fixture-A'), repo('prmanahan', 'fixture-a')]));
		expectFailClosed(r, 'prmanahan');
	});

	it('the same name under the two different owners is fine (control)', () => {
		const r = runFetch(simple([repo('mnemra', 'fixture-shared')], [repo('prmanahan', 'fixture-shared')]));
		expectSuccess(r);
	});
});

describe('R-2: the write, and what the failure log carries', () => {
	const canChmod = typeof process.getuid === 'function' && process.getuid() !== 0;

	it.skipIf(!canChmod)('a write failure logs a closed cause code (EACCES), not the runtime message or the path', () => {
		// Given a static directory the script cannot write into, and a run that otherwise succeeds
		const r = runFetch(simple([okM()], [okP()]), { seed: false, readOnlyStatic: true });
		// Then it exits non-zero with no file, and the log carries the code
		expect(r.booted).toBe(true);
		expect(r.signal).toBeNull();
		expect(r.status, `exit code (output: ${r.output})`).not.toBe(0);
		expect(r.status).not.toBeNull();
		expect(r.fileExists).toBe(false);
		expect(r.stderr, 'the cause class is a closed code').toMatch(/\bEACCES\b/);
		// And never the runtime's message text or the temp path
		expect(r.output).not.toMatch(/permission denied/i);
		expect(r.output).not.toContain(r.tmp);
		expect(r.output).not.toMatch(CRASH);
		expectNoTokenLeak(r);
	});

	it('a body-read failure is logged as such, not as "not JSON", and without the runtime message', () => {
		// Given a 200 whose body stream errors while being read
		const s = simple([okM()], [okP()]);
		s.routes[MNEMRA] = { responses: [{ status: 200, bodyError: true }] };
		const r = runFetch(s);
		// Then it fails closed naming mnemra
		expectFailClosed(r, 'mnemra');
		// And the log does not call it a JSON problem or echo the error text
		expect(r.output).not.toMatch(/not JSON/i);
		expect(r.output).not.toContain('SENTINEL-BODY-READ-FAILURE');
	});

	it('control: a 200 whose text is not JSON is still logged as not JSON', () => {
		const s = simple([okM()], [okP()]);
		s.routes[MNEMRA] = { responses: [{ status: 200, rawBody: '<html>' }] };
		const r = runFetch(s);
		expectFailClosed(r, 'mnemra');
		expect(r.output).toMatch(/not JSON/i);
	});

	it('a successful run leaves exactly the output file in the static directory, no temp sibling', () => {
		const r = runFetch(simple([okM()], [okP()]));
		expectSuccess(r);
		expect(r.staticListing).toEqual(['projects.json']);
	});

	it('a failed run leaves nothing in the static directory', () => {
		const r = runFetch(simple([], [okP()]));
		expectFailClosed(r, 'mnemra');
		expect(r.staticListing).toEqual([]);
	});
});

describe('R-2: import order, timeout and the atomic write (hardening pins, #4726)', () => {
	it('M1: a broken invisible table fails the run closed, with the stale file already deleted and no request sent', () => {
		// Given a seeded output file and an invisible-table module that does not parse
		const r = runFetch(simple([okM()], [okP()]), { breakTable: true });
		// Then the run exits 1 with the catch-all line, no file, and nothing was requested
		expect(r.booted).toBe(true);
		expect(r.signal).toBeNull();
		expect(r.status, `exit code (output: ${r.output})`).toBe(1);
		expect(r.fileExists, 'the stale seeded file must not survive').toBe(false);
		expect(r.requests).toHaveLength(0);
		expect(r.stderr).toContain('fetch-projects failed: unexpected SyntaxError');
	});

	it('M2: every request carries an abort signal, built from a 30000 ms timeout', () => {
		const r = runFetch(simple([okM()], [okP()]));
		expectSuccess(r);
		expect(r.requests.length).toBeGreaterThan(0);
		for (const q of r.requests) expect(q.hasSignal, `no AbortSignal on ${q.url}`).toBe(true);
		expect(r.timeouts.length).toBeGreaterThan(0);
		for (const t of r.timeouts) expect(t.ms).toBe(30000);
	});

	it('M2: a hanging mnemra page 1 times out into a closed, named failure with no file', () => {
		// Given mnemra page 1 never answers (the double shortens the real timer to 50 ms)
		const s = simple([okM()], [okP()]);
		s.routes[MNEMRA] = { responses: [{ hang: true }] };
		const r = runFetch(s, { spawnTimeout: 8000 });
		// Then it ends by its own timeout, not by the spawn timeout killing it
		expect(r.signal, 'child was killed by the spawn timeout: the request has no working timeout').toBeNull();
		expect(r.status, `exit code (output: ${r.output})`).toBe(1);
		expect(r.stderr.trim()).toBe('fetch-projects failed: mnemra: request failed on page 1 (TimeoutError)');
		expect(r.fileExists).toBe(false);
		expect(r.timeouts.map((t) => t.ms)).toEqual([30000]);
		expect(requestsTo(r, PRMANAHAN)).toHaveLength(0);
		expectNoTokenLeak(r);
	});

	it('M3: a successful run never writes the output path directly, and renames exactly one temp into it', () => {
		const r = runFetch(simple([okM()], [okP()]));
		expectSuccess(r);
		expect(r.writes.length).toBeGreaterThan(0);
		for (const w of r.writes) expect(w.path, 'direct write to the output path').not.toBe(r.outPath);
		expect(r.writes.map((w) => w.path)).toEqual([`${r.outPath}.${r.pid}.tmp`]);
		expect(r.renames).toEqual([{ kind: 'rename', from: `${r.outPath}.${r.pid}.tmp`, to: r.outPath }]);
	});

	it('M3: a rename that fails after the temp write leaves no file and no temp sibling, and exits 1', () => {
		// Given the temp file is written and then the rename throws
		const s = simple([okM()], [okP()]);
		s.failRename = true;
		const r = runFetch(s);
		// Then the temp write did happen (the failure is after it), and the cleanup removed it
		expect(r.writes.map((w) => w.path)).toEqual([`${r.outPath}.${r.pid}.tmp`]);
		expect(r.renames).toHaveLength(1);
		expect(r.signal).toBeNull();
		expect(r.status, `exit code (output: ${r.output})`).toBe(1);
		expect(r.stderr).toContain('fetch-projects failed: unexpected EXDEV');
		expect(r.fileExists).toBe(false);
		expect(r.staticListing, 'no temp sibling may remain').toEqual([]);
		expectNoTokenLeak(r);
	});

	it('L1 (red until fixed): when the output path cannot be removed, the temp file is still removed and the log names the output path', () => {
		// Given the output path becomes a non-empty directory just before the last response,
		// so the rename fails after the temp write and the cleanup's own rmSync(OUT) throws
		const s = simple([okM()], [okP()]);
		s.routes[PRMANAHAN] = { responses: [{ ...page([okP()]), outAsDir: true }] };
		const r = runFetch(s);
		expect(r.writes.map((w) => w.path), 'the temp write must have happened').toEqual([`${r.outPath}.${r.pid}.tmp`]);
		expect(r.signal).toBeNull();
		expect(r.status, `exit code (output: ${r.output})`).toBe(1);
		// Then the temp file is gone: only the output path (a directory) remains
		expect(r.staticListing, 'the temp file must be removed even when removing the output path fails').toEqual(['projects.json']);
		// And the failure line says the output path (by name, not by absolute path) could not be removed
		expect(r.stderr).toMatch(/projects\.json[^\n]*could not be removed|could not remove[^\n]*projects\.json/);
		expect(r.stderr, 'only the output path failed to remove, so the temp file is not named').not.toMatch(/\.tmp/);
		expect(r.stderr).not.toContain(r.tmp);
		expectNoTokenLeak(r);
	});
});

describe('R-5: the token never reaches the file, stdout or stderr', () => {
	it('success path', () => {
		const r = runFetch(sharedScenario());
		expectSuccess(r);
		expect(r.requests.some((q) => (q.authorization ?? '').includes(TOKEN)), 'control: token was sent').toBe(true);
		expect(r.stdout).not.toContain(TOKEN);
		expect(r.stderr).not.toContain(TOKEN);
		expect(r.fileText).not.toContain(TOKEN);
	});

	it.each([
		['an R-1 failure (3xx)', () => {
			const s = simple([okM()], [okP()]);
			s.routes[MNEMRA] = { responses: [{ status: 301, headers: { location: `${MNEMRA}?page=2` }, body: [] }] };
			return s;
		}],
		['an R-2 failure (503)', () => {
			const s = simple([okM()], [okP()]);
			s.routes[MNEMRA] = { responses: [{ status: 503, body: {} }] };
			return s;
		}],
		['an R-4 failure (private repo)', () => simple([okM(), repo('mnemra', 'fixture-priv', { private: true })], [okP()])],
		['an R-4 failure (malformed description)', () => simple([repo('mnemra', 'fixture-bad', { description: 'a\nb' })], [okP()])],
	])('failure path: %s', (_l, make) => {
		const r = runFetch(make());
		expectFailClosed(r, 'mnemra');
		expect(r.stdout).not.toContain(TOKEN);
		expect(r.stderr).not.toContain(TOKEN);
	});
});

describe('R-6: the log', () => {
	it('names, per owner, the included names and their count, each excluded name with its reason, and each empty description', () => {
		const r = runFetch(sharedScenario());
		expectSuccess(r);
		const lines = r.output.split('\n');
		const mnemraIncluded = ['fixture-a-b', 'fixture-a_b', 'fixture-mixed-a', 'fixture-Mixed-B', 'fixture-zeta'];
		const prIncluded = ['fixture-factor', 'fixture-sigil', 'fixture-unique-4726-listed'];
		for (const n of [...mnemraIncluded, ...prIncluded]) {
			expect(r.output, `included name ${n}`).toContain(n);
		}
		// counts, on a line that also names the owner
		expect(lines.some((l) => l.includes('mnemra') && /\b5\b/.test(l)), 'mnemra count 5').toBe(true);
		expect(lines.some((l) => l.includes('prmanahan') && /\b3\b/.test(l)), 'prmanahan count 3').toBe(true);
		// excluded names, each with its reason on the same line
		const excluded = [
			['fixture-forked', /fork/i],
			['fixture-archived', /archiv/i],
			['fixture-prfork', /fork/i],
		];
		for (const [name, reason] of excluded) {
			expect(
				lines.some((l) => l.includes(name) && reason.test(l.replace(name, ''))),
				`excluded ${name} with its reason on one line`,
			).toBe(true);
		}
		// included repos with an empty description, flagged on a line naming them
		const emptyMarker = /(empty|missing|blank|no|without|null|none)\W+descri|descri\w*\W+(is\W+)?(empty|missing|blank|null|none)/i;
		for (const name of ['fixture-a_b', 'fixture-mixed-a', 'fixture-factor']) {
			expect(
				lines.some((l) => l.includes(name) && emptyMarker.test(l)),
				`${name} flagged as having an empty description`,
			).toBe(true);
		}
		for (const name of ['fixture-zeta', 'fixture-sigil']) {
			expect(
				lines.some((l) => l.includes(name) && emptyMarker.test(l)),
				`${name} has a description and must not be flagged`,
			).toBe(false);
		}
	});

	it('on failure the log names the condition that failed', () => {
		const s = simple([okM()], [okP()]);
		s.routes[PRMANAHAN] = { responses: [{ status: 503, body: {} }] };
		const r = runFetch(s);
		expectFailClosed(r, 'prmanahan');
		expect(r.output).toContain('503');
		expect(r.output).toContain('prmanahan');
	});
});

describe('R-8: the output file', () => {
	it('equals the committed expected file except for fetched_at', () => {
		const r = runFetch(sharedScenario());
		expectSuccess(r);
		const { fetched_at: _got, ...got } = r.file;
		const { fetched_at: _exp, ...exp } = readExpected();
		expect(got).toEqual(exp);
	});

	it('orders owners mnemra then prmanahan, repos by ASCII-lowercased name (a-b before a_b, mixed case folded)', () => {
		const r = runFetch(sharedScenario());
		expectSuccess(r);
		expect(r.file.groups.map((g) => g.owner)).toEqual(['mnemra', 'prmanahan']);
		expect(r.file.groups[0].repos.map((x) => x.name)).toEqual([
			'fixture-a-b',
			'fixture-a_b',
			'fixture-mixed-a',
			'fixture-Mixed-B',
			'fixture-zeta',
		]);
	});

	it('holds exactly the Data Model fields: no URL, no extra key at any level', () => {
		const r = runFetch(sharedScenario());
		expectSuccess(r);
		expect(Object.keys(r.file).sort()).toEqual(['fetched_at', 'groups']);
		for (const g of r.file.groups) {
			expect(Object.keys(g).sort()).toEqual(['owner', 'repos']);
			for (const x of g.repos) expect(Object.keys(x).sort()).toEqual(['description', 'name']);
		}
		expect(r.fileText).not.toMatch(/https?:\/\//);
	});

	it('fetched_at matches the Data Model grammar, is a valid instant, and falls within this run', () => {
		const r = runFetch(sharedScenario());
		expectSuccess(r);
		const f = r.file.fetched_at;
		expect(f).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);
		expect(new Date(f).toISOString()).toBe(f);
		const t = Date.parse(f);
		expect(t).toBeGreaterThanOrEqual(r.startedAt - 1000);
		expect(t).toBeLessThanOrEqual(r.finishedAt + 1000);
	});

	it('is stable across runs: the same input yields the same file apart from fetched_at', () => {
		const a = runFetch(sharedScenario());
		const b = runFetch(sharedScenario());
		expectSuccess(a);
		expectSuccess(b);
		const strip = ({ fetched_at: _f, ...rest }) => rest;
		expect(strip(a.file)).toEqual(strip(b.file));
	});
});
