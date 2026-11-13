/**
 * R-7 (docs/specs/2026-11-11-projects-from-github.md): structural test of
 * .github/workflows/deploy.yml (#4726 T7; hardened to allowlist polarity in the T8 review fix).
 *
 * Reads, and why:
 *  - .github/workflows/deploy.yml, the artifact under test. Read as data and parsed with `yaml`
 *    (devDependency approved by Peter, 2026-11-13); comments are not part of the structure.
 *  - spec R-7 and plan D14 (step name `Fetch projects list`).
 * No implementation code is read or called.
 *
 * Shape: `r7Violations(doc)` is test-side checker logic derived from R-7's clauses, each returning a
 * rule id. Polarity is allowlist: the fetch step and Deploy's env are pinned to exactly the permitted
 * form, so a form the checker did not think to forbid still fails. Three kinds of test:
 *  1. the real file, one test per clause, plus the pins;
 *  2. a compliant baseline (the real file with its fetch step re-set to the compliant one) must have
 *     zero violations, so a seeded failure below cannot be a false positive of the checker;
 *  3. seeded copies of the baseline, one per violation, each must raise the named rule. These guard
 *     the checker itself.
 */
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { parse } from 'yaml';

const WORKFLOW = resolve(import.meta.dirname, '../../../../.github/workflows/deploy.yml');

const FETCH_STEP_NAME = 'Fetch projects list';
const DEPLOY_STEP_NAME = 'Deploy';
const DEPLOY_RUN = 'flyctl deploy --remote-only';
const DEPLOY_JOB = 'deploy';
const DEPLOY_ENV = { FLY_API_TOKEN: '${{ secrets.FLY_API_TOKEN }}' };
const DEPLOY_KEYS = ['name', 'env', 'run'];
const USES_ALLOWLIST =['actions/checkout@v6.0.2', 'superfly/flyctl-actions/setup-flyctl@1.5'];

type Json = null | boolean | number | string | Json[] | { [k: string]: Json };
type Obj = { [k: string]: Json };
type Step = Obj;
type Workflow = Obj;

function load(): Workflow {
	return parse(readFileSync(WORKFLOW, 'utf8')) as Workflow;
}

function clone<T>(v: T): T {
	return structuredClone(v);
}

function jobs(doc: Workflow): Record<string, Obj> {
	return (doc.jobs ?? {}) as Record<string, Obj>;
}

function stepsOf(job: Obj | undefined): Step[] {
	return ((job?.steps as Json[] | undefined) ?? []) as Step[];
}

/** Every string leaf (values and keys) under a node, with the path it was found at. */
function walk(node: Json | undefined, path: string[], out: { path: string[]; text: string }[]) {
	if (node === null || node === undefined) return;
	if (typeof node === 'string') {
		out.push({ path, text: node });
	} else if (Array.isArray(node)) {
		node.forEach((n, i) => walk(n, [...path, String(i)], out));
	} else if (typeof node === 'object') {
		for (const [k, v] of Object.entries(node)) {
			out.push({ path: [...path, k], text: k });
			walk(v, [...path, k], out);
		}
	}
}

/** Expression bodies `${{ ... }}` in a string. */
function expressions(text: string): string[] {
	return [...text.matchAll(/\$\{\{([\s\S]*?)\}\}/g)].map((m) => m[1]);
}

/** What a secret reference inside one expression names. `*` is a form we cannot resolve to one name. */
function secretNames(expr: string): string[] {
	if (!/\bsecrets\b/.test(expr)) return [];
	const names: string[] = [];
	for (const m of expr.matchAll(/\bsecrets\s*\.\s*([A-Za-z_][A-Za-z0-9_-]*)/g)) names.push(m[1]);
	for (const m of expr.matchAll(/\bsecrets\s*\[\s*(['"])([^'"]*)\1\s*\]/g)) names.push(m[2]);
	// toJSON(secrets), secrets[var], secrets.* and any other whole-object use
	const resolved = [...expr.matchAll(/\bsecrets\b/g)].length;
	const named = [...expr.matchAll(/\bsecrets\s*\.\s*[A-Za-z_]/g)].length +
		[...expr.matchAll(/\bsecrets\s*\[\s*(['"])[^'"]*\1\s*\]/g)].length;
	if (resolved > named) names.push('*');
	return names;
}

function r7Violations(doc: Workflow): string[] {
	const v: string[] = [];
	const J = jobs(doc);

	// permissions exactly `contents: read`, and no job sets permissions
	const perms = doc.permissions as Obj | undefined;
	if (
		!perms ||
		Object.keys(perms).length !== 1 ||
		perms.contents !== 'read'
	) {
		v.push('permissions-not-contents-read');
	}
	for (const job of Object.values(J)) {
		if ('permissions' in job) v.push('job-sets-permissions');
	}

	// no workflow-level or job-level env mentions FLY_API_TOKEN
	const envLeaves: { path: string[]; text: string }[] = [];
	walk(doc.env as Json, ['env'], envLeaves);
	for (const [name, job] of Object.entries(J)) walk(job.env as Json, [name, 'env'], envLeaves);
	if (envLeaves.some((l) => l.text.includes('FLY_API_TOKEN'))) v.push('env-mentions-fly-token');

	// the deploy job: steps, Deploy, the fetch step
	const deploy = J[DEPLOY_JOB];
	const dsteps = stepsOf(deploy);
	const deployIdx = dsteps.findIndex((s) => s.name === DEPLOY_STEP_NAME);
	const fetchIdxs = dsteps.map((s, i) => (s.name === FETCH_STEP_NAME ? i : -1)).filter((i) => i >= 0);
	const fetchStep = fetchIdxs.length === 1 ? dsteps[fetchIdxs[0]] : undefined;

	// secrets outside Deploy: only GITHUB_TOKEN, and only as `env` of the fetch step
	for (const [jobName, job] of Object.entries(J)) {
		const leaves: { path: string[]; text: string }[] = [];
		walk(job as Json, [jobName], leaves);
		const isDeployStepLeaf = (path: string[]) => {
			if (jobName !== DEPLOY_JOB) return false;
			const i = path.indexOf('steps');
			if (i < 0) return false;
			return Number(path[i + 1]) === deployIdx && deployIdx >= 0;
		};
		for (const leaf of leaves) {
			if (isDeployStepLeaf(leaf.path)) continue;
			for (const expr of expressions(leaf.text)) {
				for (const name of secretNames(expr)) {
					if (name !== 'GITHUB_TOKEN') {
						v.push('secret-other-than-github-token-outside-deploy');
						continue;
					}
					const si = leaf.path.indexOf('steps');
					const isFetchEnv =
						jobName === DEPLOY_JOB &&
						si >= 0 &&
						fetchIdxs.length === 1 &&
						Number(leaf.path[si + 1]) === fetchIdxs[0] &&
						leaf.path[si + 2] === 'env';
					if (!isFetchEnv) v.push('github-token-not-fetch-step-env');
				}
			}
		}
	}
	// workflow-level keys other than jobs
	{
		const leaves: { path: string[]; text: string }[] = [];
		for (const [k, val] of Object.entries(doc)) if (k !== 'jobs') walk(val, [k], leaves);
		for (const leaf of leaves) {
			for (const expr of expressions(leaf.text)) {
				if (secretNames(expr).length > 0) v.push('secret-at-workflow-level');
			}
		}
	}

	// github.token / github['token'] / toJSON(github) is a token reference: allowed nowhere
	{
		const leaves: { path: string[]; text: string }[] = [];
		walk(doc as Json, [], leaves);
		for (const leaf of leaves) {
			for (const expr of expressions(leaf.text)) {
				if (
					/\bgithub\s*\.\s*token\b/i.test(expr) ||
					/\bgithub\s*\[\s*(['"])token\1\s*\]/i.test(expr) ||
					/\btoJSON\s*\(\s*github\s*\)/i.test(expr)
				) {
					v.push('github-context-token-reference');
				}
			}
		}
	}

	// the fetch step: exists once, expected name, before Deploy, no escape hatches
	if (fetchIdxs.length !== 1) {
		v.push('fetch-step-missing-or-duplicated');
	} else {
		const s = fetchStep as Step;
		if (!isDeepStrictEqual(s, COMPLIANT_FETCH)) v.push('fetch-step-not-pinned');
		if (deployIdx < 0 || fetchIdxs[0] > deployIdx) v.push('fetch-step-not-before-deploy');
		if ('continue-on-error' in s) v.push('fetch-step-continue-on-error');
		if ('if' in s) v.push('fetch-step-if');
		if ('shell' in s) v.push('fetch-step-shell');
		if (typeof s.run !== 'string' || s.run.trim() === '') {
			v.push('fetch-step-no-run');
		} else {
			if (s.run.includes('|')) v.push('fetch-step-pipe-or-or');
		}
	}

	// Deploy's run: exactly the command
	if (deployIdx < 0) {
		v.push('deploy-step-missing');
	} else if (typeof dsteps[deployIdx].run !== 'string' || (dsteps[deployIdx].run as string).trim() !== DEPLOY_RUN) {
		v.push('deploy-run-not-exact');
	}
	if (deployIdx >= 0) {
		const d = dsteps[deployIdx];
		if (!isDeepStrictEqual(d.env, DEPLOY_ENV)) v.push('deploy-env-not-pinned');
		if (Object.keys(d).some((k) => !DEPLOY_KEYS.includes(k))) v.push('deploy-step-unexpected-key');
	}

	// every `uses:` in the deploy job is on the allowlist
	const uses: string[] = [];
	if (typeof deploy?.uses === 'string') uses.push(deploy.uses);
	for (const s of dsteps) if (typeof s.uses === 'string') uses.push(s.uses);
	for (const u of uses) if (!USES_ALLOWLIST.includes(u)) v.push('uses-not-on-allowlist');

	return [...new Set(v)];
}

/** A compliant fetch step, as the plan describes it (D14); inserted in front of Deploy. */
const COMPLIANT_FETCH: Step = {
	name: FETCH_STEP_NAME,
	env: { GITHUB_TOKEN: '${{ secrets.GITHUB_TOKEN }}' },
	run: 'node frontend/scripts/fetch-projects.mjs',
};

function withFetchStep(doc: Workflow, step: Step = COMPLIANT_FETCH): Workflow {
	const d = clone(doc);
	const steps = stepsOf(jobs(d)[DEPLOY_JOB]);
	// Idempotent: the real file gains its own fetch step in T8, and the baseline must not hold two.
	for (let j = steps.length - 1; j >= 0; j--) {
		if (steps[j].name === FETCH_STEP_NAME) steps.splice(j, 1);
	}
	const i = steps.findIndex((s) => s.name === DEPLOY_STEP_NAME);
	steps.splice(i, 0, clone(step));
	return d;
}

function deploySteps(d: Workflow): Step[] {
	return stepsOf(jobs(d)[DEPLOY_JOB]);
}

function stepByName(d: Workflow, name: string): Step {
	const s = deploySteps(d).find((x) => x.name === name);
	if (!s) throw new Error(`seed setup: no step named ${name}`);
	return s;
}

describe('R-7: the real deploy.yml', () => {
	const real = () => load();

	it('R-7 permissions is exactly contents: read and no job sets permissions', () => {
		const v = r7Violations(real());
		expect(v).not.toContain('permissions-not-contents-read');
		expect(v).not.toContain('job-sets-permissions');
	});

	it('R-7 no workflow-level or job-level env mentions FLY_API_TOKEN', () => {
		expect(r7Violations(real())).not.toContain('env-mentions-fly-token');
	});

	it('R-7 outside Deploy the only secret referenced is GITHUB_TOKEN, and only as env on the fetch step', () => {
		const v = r7Violations(real());
		expect(v).not.toContain('secret-other-than-github-token-outside-deploy');
		expect(v).not.toContain('github-token-not-fetch-step-env');
		expect(v).not.toContain('secret-at-workflow-level');
	});

	it('R-7 the fetch step "Fetch projects list" exists exactly once in the deploy job', () => {
		const names = deploySteps(real()).map((s) => s.name);
		expect(names.filter((n) => n === FETCH_STEP_NAME)).toEqual([FETCH_STEP_NAME]);
	});

	it('R-7 the fetch step comes before Deploy', () => {
		const names = deploySteps(real()).map((s) => s.name);
		const f = names.indexOf(FETCH_STEP_NAME);
		const d = names.indexOf(DEPLOY_STEP_NAME);
		expect(f).toBeGreaterThanOrEqual(0);
		expect(d).toBeGreaterThanOrEqual(0);
		expect(f).toBeLessThan(d);
	});

	it('R-7 the fetch step has no continue-on-error, if, shell, pipe or ||', () => {
		const step = deploySteps(real()).find((s) => s.name === FETCH_STEP_NAME);
		expect(step, 'fetch step must exist').toBeDefined();
		const s = step as Step;
		expect('continue-on-error' in s).toBe(false);
		expect('if' in s).toBe(false);
		expect('shell' in s).toBe(false);
		expect(typeof s.run).toBe('string');
		expect(String(s.run).trim()).not.toBe('');
		expect(String(s.run)).not.toContain('|');
	});

	it('R-7 Deploy run is exactly "flyctl deploy --remote-only"', () => {
		const deploy = stepByName(real(), DEPLOY_STEP_NAME);
		expect(String(deploy.run).trim()).toBe(DEPLOY_RUN);
	});

	it('R-7 every uses: in the deploy job is on the allowlist', () => {
		const uses = deploySteps(real())
			.map((s) => s.uses)
			.filter((u): u is string => typeof u === 'string');
		expect(uses.length).toBeGreaterThan(0);
		for (const u of uses) expect(USES_ALLOWLIST).toContain(u);
	});

	it('R-7 the fetch step is exactly the pinned form and Deploy env is exactly FLY_API_TOKEN', () => {
		const d = real();
		expect(stepByName(d, FETCH_STEP_NAME)).toEqual(COMPLIANT_FETCH);
		const deploy = stepByName(d, DEPLOY_STEP_NAME);
		expect(deploy.env).toEqual(DEPLOY_ENV);
		expect('with' in deploy).toBe(false);
	});

	it('R-7 the whole real file has no violation', () => {
		expect(r7Violations(real())).toEqual([]);
	});
});

describe('R-7: the checker, on seeded copies of a compliant baseline', () => {
	const baseline = () => withFetchStep(load());

	it('control: the compliant baseline has no violation', () => {
		expect(r7Violations(baseline())).toEqual([]);
	});

	it('seed: removed fetch step', () => {
		const d = baseline();
		const steps = deploySteps(d);
		steps.splice(steps.findIndex((s) => s.name === FETCH_STEP_NAME), 1);
		expect(r7Violations(d)).toContain('fetch-step-missing-or-duplicated');
	});

	it('seed: renamed fetch step', () => {
		const d = baseline();
		stepByName(d, FETCH_STEP_NAME).name = 'Get projects';
		expect(r7Violations(d)).toContain('fetch-step-missing-or-duplicated');
	});

	it('seed: fetch step after Deploy', () => {
		const d = baseline();
		const steps = deploySteps(d);
		const [f] = steps.splice(steps.findIndex((s) => s.name === FETCH_STEP_NAME), 1);
		steps.splice(steps.findIndex((s) => s.name === DEPLOY_STEP_NAME) + 1, 0, f);
		expect(r7Violations(d)).toContain('fetch-step-not-before-deploy');
	});

	it.each([
		['continue-on-error', 'fetch-step-continue-on-error', { 'continue-on-error': true }],
		['if', 'fetch-step-if', { if: "github.ref == 'refs/heads/main'" }],
		['shell', 'fetch-step-shell', { shell: 'bash {0}' }],
	])('seed: fetch step sets %s', (_k, rule, extra) => {
		const d = baseline();
		Object.assign(stepByName(d, FETCH_STEP_NAME), extra);
		expect(r7Violations(d)).toContain(rule);
	});

	it.each([
		['a pipe', 'node frontend/scripts/fetch-projects.mjs | tee out.log'],
		['||', 'node frontend/scripts/fetch-projects.mjs || true'],
	])('seed: fetch run contains %s', (_k, run) => {
		const d = baseline();
		stepByName(d, FETCH_STEP_NAME).run = run;
		expect(r7Violations(d)).toContain('fetch-step-pipe-or-or');
	});

	it.each([
		['an unlisted action', 'actions/cache@v4'],
		['checkout at another version', 'actions/checkout@v6.0.3'],
		['checkout on a floating ref', 'actions/checkout@v6'],
		['setup-flyctl on master', 'superfly/flyctl-actions/setup-flyctl@master'],
	])('seed: deploy job uses %s', (_k, uses) => {
		const d = baseline();
		deploySteps(d).splice(1, 0, { name: 'Extra', uses });
		expect(r7Violations(d)).toContain('uses-not-on-allowlist');
	});

	it.each([
		['--skip-release-command appended', 'flyctl deploy --remote-only --skip-release-command'],
		['--skip-release-command first', 'flyctl deploy --skip-release-command --remote-only'],
		['a second command', 'flyctl deploy --remote-only && true'],
		['without --remote-only', 'flyctl deploy'],
	])('seed: Deploy run has %s', (_k, run) => {
		const d = baseline();
		stepByName(d, DEPLOY_STEP_NAME).run = run;
		expect(r7Violations(d)).toContain('deploy-run-not-exact');
	});

	it('seed: workflow permissions broadened', () => {
		const d = baseline();
		d.permissions = { contents: 'write' };
		expect(r7Violations(d)).toContain('permissions-not-contents-read');
	});

	it('seed: workflow permissions gain a second key', () => {
		const d = baseline();
		d.permissions = { contents: 'read', 'id-token': 'write' };
		expect(r7Violations(d)).toContain('permissions-not-contents-read');
	});

	it('seed: a job sets permissions', () => {
		const d = baseline();
		jobs(d)[DEPLOY_JOB].permissions = { contents: 'read' };
		expect(r7Violations(d)).toContain('job-sets-permissions');
	});

	it.each([
		['workflow-level', (d: Workflow) => (d.env = { FLY_API_TOKEN: 'x' })],
		['job-level', (d: Workflow) => (jobs(d)[DEPLOY_JOB].env = { FLY_API_TOKEN: '${{ secrets.FLY_API_TOKEN }}' })],
		['test-job-level', (d: Workflow) => (jobs(d).test.env = { FLY_API_TOKEN: 'x' })],
	])('seed: %s env mentions FLY_API_TOKEN', (_k, mutate) => {
		const d = baseline();
		mutate(d);
		expect(r7Violations(d)).toContain('env-mentions-fly-token');
	});

	it.each([
		['FLY_API_TOKEN in the fetch step env', (d: Workflow) => (stepByName(d, FETCH_STEP_NAME).env = { GITHUB_TOKEN: '${{ secrets.GITHUB_TOKEN }}', X: '${{ secrets.FLY_API_TOKEN }}' })],
		['secrets[...] bracket form in the fetch step env', (d: Workflow) => (stepByName(d, FETCH_STEP_NAME).env = { GITHUB_TOKEN: "${{ secrets['FLY_API_TOKEN'] }}" })],
		['toJSON(secrets) in the fetch step env', (d: Workflow) => (stepByName(d, FETCH_STEP_NAME).env = { ALL: '${{ toJSON(secrets) }}' })],
		['a secret in the test job', (d: Workflow) => (stepsOf(jobs(d).test)[0].env = { T: '${{ secrets.FLY_API_TOKEN }}' })],
		['a secret in the install step with:', (d: Workflow) => (stepByName(d, 'Install flyctl').with = { token: '${{ secrets.FLY_API_TOKEN }}' })],
		['a secret in the post-deploy health check', (d: Workflow) => (stepByName(d, 'Post-deploy health check').env = { T: '${{ secrets.FLY_API_TOKEN }}' })],
	])('seed: %s', (_k, mutate) => {
		const d = baseline();
		mutate(d);
		expect(r7Violations(d)).toContain('secret-other-than-github-token-outside-deploy');
	});

	it.each([
		[
			'${{ github.token }} in a later step env',
			'github-context-token-reference',
			(d: Workflow) => (stepByName(d, 'Post-deploy health check').env = { T: '${{ github.token }}' }),
		],
		[
			"github['token'] in a later step env",
			'github-context-token-reference',
			(d: Workflow) => (stepByName(d, 'Post-deploy health check').env = { T: "${{ github['token'] }}" }),
		],
		[
			'toJSON(github) in the fetch step env',
			'github-context-token-reference',
			(d: Workflow) =>
				(stepByName(d, FETCH_STEP_NAME).env = {
					GITHUB_TOKEN: '${{ secrets.GITHUB_TOKEN }}',
					X: '${{ toJSON(github) }}',
				}),
		],
		[
			'toJSON(secrets) in Deploy env',
			'deploy-env-not-pinned',
			(d: Workflow) =>
				(stepByName(d, DEPLOY_STEP_NAME).env = {
					FLY_API_TOKEN: '${{ secrets.FLY_API_TOKEN }}',
					X: '${{ toJSON(secrets) }}',
				}),
		],
		[
			'secrets.GITHUB_TOKEN in Deploy env',
			'deploy-env-not-pinned',
			(d: Workflow) =>
				(stepByName(d, DEPLOY_STEP_NAME).env = {
					FLY_API_TOKEN: '${{ secrets.FLY_API_TOKEN }}',
					G: '${{ secrets.GITHUB_TOKEN }}',
				}),
		],
		[
			'a with: on Deploy',
			'deploy-step-unexpected-key',
			(d: Workflow) => (stepByName(d, DEPLOY_STEP_NAME).with = { token: 'x' }),
		],
		[
			'a fetch run wrapped in if ! ...; then echo warn; fi',
			'fetch-step-not-pinned',
			(d: Workflow) =>
				(stepByName(d, FETCH_STEP_NAME).run =
					'if ! node frontend/scripts/fetch-projects.mjs; then echo warn; fi'),
		],
		[
			'set +e added to the fetch run',
			'fetch-step-not-pinned',
			(d: Workflow) =>
				(stepByName(d, FETCH_STEP_NAME).run = 'set +e\nnode frontend/scripts/fetch-projects.mjs'),
		],
		[
			'an extra key on the fetch step',
			'fetch-step-not-pinned',
			(d: Workflow) => (stepByName(d, FETCH_STEP_NAME)['timeout-minutes'] = 5),
		],
	])('seed (allowlist): %s', (_k, rule, mutate) => {
		const d = baseline();
		mutate(d);
		expect(r7Violations(d)).toContain(rule);
	});

	it.each([
		['in the fetch step run:', (d: Workflow) => (stepByName(d, FETCH_STEP_NAME).run = 'node x.mjs ${{ secrets.GITHUB_TOKEN }}')],
		['in the install step env', (d: Workflow) => (stepByName(d, 'Install flyctl').env = { GITHUB_TOKEN: '${{ secrets.GITHUB_TOKEN }}' })],
		['in the test job env', (d: Workflow) => (stepsOf(jobs(d).test)[0].env = { GITHUB_TOKEN: '${{ secrets.GITHUB_TOKEN }}' })],
	])('seed: GITHUB_TOKEN outside the fetch step env, %s', (_k, mutate) => {
		const d = baseline();
		mutate(d);
		expect(r7Violations(d)).toContain('github-token-not-fetch-step-env');
	});
});
