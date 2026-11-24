/**
 * R-7 (docs/specs/2026-11-11-projects-from-github.md): structural test of
 * .github/workflows/deploy.yml (the pin and checkout rules run over every file in .github/workflows; #4726 T7; hardened to allowlist
 * polarity in the T8 review fix; action pins and checkout pin added for the amended R-7).
 *
 * Reads, and why:
 *  - every file in the workflows directory (deploy, test, audit), the artifacts under test. Read as
 *    data and parsed with `yaml` (devDependency approved by Peter, 2026-11-13); comments are not
 *    part of the structure.
 *  - spec R-7 and plan D14 (step name `Fetch projects list`).
 *  - The five action SHAs below are the commits the Base refs resolve to, checked on 2026-11-18
 *    against the upstream repositories (not copied from the workflow files).
 * No implementation code is called. The R-35 check-job block (T13) also reads the two check scripts,
 * frontend/scripts/check-freshness-daily.mjs and check-freshness-post-deploy.mjs, as text, to scan
 * their imports and tokens; it never runs them.
 *
 * Shape: `r7Violations(doc)` is test-side checker logic derived from R-7's clauses, each returning a
 * rule id. Polarity is allowlist: the fetch step and Deploy's env are pinned to exactly the permitted
 * form, so a form the checker did not think to forbid still fails. Three kinds of test:
 *  1. the real file, one test per clause, plus the pins;
 *  2. a compliant baseline (the real file with its fetch step re-set to the compliant one) must have
 *     zero violations, so a seeded failure below cannot be a false positive of the checker;
 *  3. seeded copies of the baseline, one per violation, each must raise the named rule. These guard
 *     the checker itself.
 * The action pins (`pinViolations`) and the checkout pin (`checkoutViolations`) run over every workflow file in the directory.
 * Their seeds are applied one seeded copy per seed and `uses:` (or checkout step), and each must
 * raise the rule at that very locator, so a seed cannot be caught by an unrelated clause.
 */
import { describe, it, expect } from 'vitest';
import { readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { parse } from 'yaml';

const WORKFLOW = resolve(import.meta.dirname, '../../../../.github/workflows/deploy.yml');
const WORKFLOW_DIR = resolve(import.meta.dirname, '../../../../.github/workflows');

const FETCH_STEP_NAME = 'Fetch projects list';
const DEPLOY_STEP_NAME = 'Deploy';
const DEPLOY_RUN = 'flyctl deploy --remote-only';
const DEPLOY_JOB = 'deploy';
const DEPLOY_ENV = { FLY_API_TOKEN: '${{ secrets.FLY_API_TOKEN }}' };
const DEPLOY_KEYS = ['name', 'env', 'run'];

/** One entry per action used at the Base: its path, the commit its Base ref resolves to, the Base ref. */
const ACTIONS = {
	checkout: {
		path: 'actions/checkout',
		sha: 'de0fac2e4500dabe0009e67214ff5f5447ce83dd',
		baseRef: 'v6.0.2'
	},
	setupNode: {
		path: 'actions/setup-node',
		sha: '53b83947a5a98c8d113130e565377fae1a50d02f',
		baseRef: 'v6.3.0'
	},
	rustToolchain: {
		path: 'dtolnay/rust-toolchain',
		sha: '516c5a67e611cdaf2ef0bb5d78620e3c7d245fe6',
		baseRef: '1.98.0'
	},
	rustCache: {
		path: 'Swatinem/rust-cache',
		sha: 'c19371144df3bb44fab255c43d04cbc2ab54d1c4',
		baseRef: 'v2.9.1'
	},
	setupFlyctl: {
		path: 'superfly/flyctl-actions/setup-flyctl',
		sha: 'fc53c09e1bc3be6f54706524e3b82c4f462f77be',
		baseRef: '1.5'
	}
} as const;
const pair = (a: { path: string; sha: string }) => `${a.path}@${a.sha}`;
/** The exact-pair allowlist for every `uses:` in every workflow file. */
const PIN_ALLOWLIST: string[] = Object.values(ACTIONS).map(pair);
/** The deploy job's narrower set: the checkout pair and the setup-flyctl pair, nothing else. */
const USES_ALLOWLIST: string[] = [pair(ACTIONS.checkout), pair(ACTIONS.setupFlyctl)];
const USES_PIN_RE = /^[A-Za-z0-9-]+\/[A-Za-z0-9._-]+(\/[A-Za-z0-9._-]+)*@[0-9a-f]{40}$/;
const FLYCTL_VERSION_RE = /^[0-9]+\.[0-9]+\.[0-9]+$/;
const UNLISTED_SHA = 'f'.repeat(40);

type Json = null | boolean | number | string | Json[] | { [k: string]: Json };
type Obj = { [k: string]: Json };
type Step = Obj;
type Workflow = Obj;

function load(file: string = WORKFLOW): Workflow {
	return parse(readFileSync(file, 'utf8')) as Workflow;
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
	for (const u of uses) if (!USES_ALLOWLIST.includes(u)) v.push('uses-not-on-deploy-allowlist');

	return [...new Set(v)];
}

type UseSite = { loc: string; file: string; job: string; step: number | null; value: string };

/** Every `uses:` at step or job level, with a locator `<file>:<job>:<step index | job>`. */
function useSites(file: string, doc: Workflow): UseSite[] {
	const out: UseSite[] = [];
	for (const [job, j] of Object.entries(jobs(doc))) {
		if (typeof j.uses === 'string') {
			out.push({ loc: `${file}:${job}:job`, file, job, step: null, value: j.uses });
		}
		stepsOf(j).forEach((s, i) => {
			if (typeof s.uses === 'string') {
				out.push({ loc: `${file}:${job}:${i}`, file, job, step: i, value: s.uses });
			}
		});
	}
	return out;
}

/** The action pins, over one file: shape, then exact-pair allowlist, then the setup-flyctl version. */
function pinViolations(file: string, doc: Workflow): string[] {
	const v: string[] = [];
	for (const u of useSites(file, doc)) {
		if (!USES_PIN_RE.test(u.value)) v.push(`uses-not-sha-pinned@${u.loc}`);
		if (!PIN_ALLOWLIST.includes(u.value)) v.push(`uses-not-on-pin-allowlist@${u.loc}`);
		if (u.value.startsWith('superfly/flyctl-actions/setup-flyctl@') && u.step !== null) {
			const w = stepsOf(jobs(doc)[u.job])[u.step].with as Obj | undefined;
			const ver = w?.version;
			if (typeof ver !== 'string' || !FLYCTL_VERSION_RE.test(ver)) v.push(`flyctl-version@${u.loc}`);
		}
	}
	return v;
}

/** The checkout pin, over one file: `with` deep-equals {persist-credentials: false} (boolean). */
function checkoutViolations(file: string, doc: Workflow): string[] {
	const v: string[] = [];
	for (const u of useSites(file, doc)) {
		if (u.step === null || !u.value.startsWith('actions/checkout@')) continue;
		const w = stepsOf(jobs(doc)[u.job])[u.step].with;
		if (!isDeepStrictEqual(w, { 'persist-credentials': false })) v.push(`checkout-pin@${u.loc}`);
	}
	return v;
}

/** Opt-out polarity: every `*.yml` and `*.yaml` in the workflows directory is checked, so a new workflow is covered without an edit here. */
function workflowFiles(): string[] {
	return readdirSync(WORKFLOW_DIR)
		.filter((n) => n.endsWith('.yml') || n.endsWith('.yaml'))
		.sort();
}
function loadAll(): Record<string, Workflow> {
	return Object.fromEntries(workflowFiles().map((n) => [n, load(resolve(WORKFLOW_DIR, n))]));
}
function allPinViolations(docs: Record<string, Workflow>): string[] {
	return Object.entries(docs).flatMap(([n, d]) => [...pinViolations(n, d), ...checkoutViolations(n, d)]);
}
/** The step (or, for a job-level `uses:`, the job) at a locator. */
function nodeAt(docs: Record<string, Workflow>, loc: string): Obj {
	const [file, job, idx] = loc.split(':');
	const j = jobs(docs[file])[job];
	return idx === 'job' ? j : stepsOf(j)[Number(idx)];
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

	it('R-7 every uses: in the deploy job is the checkout pair or the setup-flyctl pair', () => {
		const uses = deploySteps(real())
			.map((s) => s.uses)
			.filter((u): u is string => typeof u === 'string');
		expect(uses.length).toBeGreaterThan(0);
		for (const u of uses) expect(USES_ALLOWLIST).toContain(u);
		expect(r7Violations(real())).not.toContain('uses-not-on-deploy-allowlist');
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
		['an unlisted action', `actions/cache@${UNLISTED_SHA}`],
		['an allowlisted action outside the narrow set (setup-node)', pair(ACTIONS.setupNode)],
		['an allowlisted action outside the narrow set (rust-cache)', pair(ACTIONS.rustCache)],
		['checkout at its Base tag', 'actions/checkout@v6.0.2'],
		['checkout on a floating ref', 'actions/checkout@v6'],
		['setup-flyctl on master', 'superfly/flyctl-actions/setup-flyctl@master'],
	])('seed: deploy job uses %s', (_k, uses) => {
		const d = baseline();
		deploySteps(d).splice(1, 0, { name: 'Extra', uses });
		expect(r7Violations(d)).toContain('uses-not-on-deploy-allowlist');
	});

	it('seed: the deploy job gains a job-level uses', () => {
		const d = baseline();
		jobs(d)[DEPLOY_JOB].uses = pair(ACTIONS.setupNode);
		expect(r7Violations(d)).toContain('uses-not-on-deploy-allowlist');
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

describe('R-7: action pins and checkout pin, over every workflow file', () => {
	const docs = () => loadAll();
	const allSites = (d: Record<string, Workflow>) => Object.entries(d).flatMap(([n, w]) => useSites(n, w));
	const baseRefOf = (value: string): string => {
		const a = Object.values(ACTIONS).find((x) => x.path === value.split('@')[0]);
		return (a as { baseRef: string }).baseRef;
	};

	// The seven jobs holding a checkout step (R-7 names four; audit.yml and the two freshness checks
	// add their own), each as the first step.
	const CHECKOUT_LOCS = [
		'audit.yml:audit:0',
		'deploy.yml:test:0',
		'deploy.yml:deploy:0',
		'deploy.yml:post-deploy-freshness:0',
		'freshness-daily.yml:freshness:0',
		'test.yml:invisible-tables:0',
		'test.yml:test:0',
	];
	const FLYCTL_LOC = 'deploy.yml:deploy:1';

	describe('controls: the real files', () => {
		it('control: the real workflow files raise no pin or checkout violation', () => {
			expect(allPinViolations(docs())).toEqual([]);
		});

		it('control: the files checked are exactly the directory listing of .github/workflows', () => {
			const listed = readdirSync(WORKFLOW_DIR).filter((n) => /\.ya?ml$/.test(n));
			expect(Object.keys(docs()).sort()).toEqual([...listed].sort());
			// positive control: the listing is not empty and holds the three known files
			expect(listed).toEqual(expect.arrayContaining(['audit.yml', 'deploy.yml', 'test.yml']));
		});

		it('control: 18 uses:, 1 in audit.yml, 9 in deploy.yml, 2 in freshness-daily.yml and 6 in test.yml', () => {
			const d = docs();
			expect(useSites('audit.yml', d['audit.yml'])).toHaveLength(1);
			expect(useSites('deploy.yml', d['deploy.yml'])).toHaveLength(9);
			expect(useSites('freshness-daily.yml', d['freshness-daily.yml'])).toHaveLength(2);
			expect(useSites('test.yml', d['test.yml'])).toHaveLength(6);
			expect(allSites(d)).toHaveLength(18);
		});

		it('control: every uses: value matches the shape regex and an exact allowlist pair', () => {
			for (const u of allSites(docs())) {
				expect(u.value, u.loc).toMatch(USES_PIN_RE);
				expect(PIN_ALLOWLIST, u.loc).toContain(u.value);
			}
		});

		it('control: the allowlist holds the five Base actions and each is used at least once', () => {
			expect(PIN_ALLOWLIST).toHaveLength(5);
			const used = new Set(allSites(docs()).map((u) => u.value));
			for (const p of PIN_ALLOWLIST) expect(used, p).toContain(p);
		});

		it('control: exactly one setup-flyctl step, in the deploy job of deploy.yml', () => {
			const flyctl = allSites(docs()).filter((u) =>
				u.value.startsWith('superfly/flyctl-actions/setup-flyctl@'),
			);
			expect(flyctl.map((u) => u.loc)).toEqual([FLYCTL_LOC]);
		});

		it('control: the setup-flyctl step has with.version as an exact x.y.z string', () => {
			const w = nodeAt(docs(), FLYCTL_LOC).with as Obj;
			expect(typeof w.version).toBe('string');
			expect(w.version).toMatch(FLYCTL_VERSION_RE);
		});

		it('control: a checkout step in each of seven jobs, and in no other', () => {
			const checkouts = allSites(docs()).filter((u) => u.value.startsWith('actions/checkout@'));
			expect(checkouts.map((u) => u.loc)).toEqual(CHECKOUT_LOCS);
			expect(new Set(checkouts.map((u) => `${u.file}:${u.job}`)).size).toBe(7);
		});

		it('control: every checkout step has with deep-equal to {persist-credentials: false}, a boolean', () => {
			const d = docs();
			for (const loc of CHECKOUT_LOCS) {
				const w = nodeAt(d, loc).with;
				expect(w, loc).toStrictEqual({ 'persist-credentials': false });
				expect(typeof (w as Obj)['persist-credentials'], loc).toBe('boolean');
			}
		});
	});

	describe('seeds: one seeded copy per seed and uses:, over all 18', () => {
		const sites = allSites(docs());
		const seeds: [string, string, (u: UseSite) => string][] = [
			['the Base ref in place of the SHA', 'uses-not-sha-pinned', (u) => `${u.value.split('@')[0]}@${baseRefOf(u.value)}`],
			['the SHA cut to 7 characters', 'uses-not-sha-pinned', (u) => u.value.slice(0, u.value.indexOf('@') + 8)],
			['a 40-hex SHA on no allowlist pair', 'uses-not-on-pin-allowlist', (u) => `${u.value.split('@')[0]}@${UNLISTED_SHA}`],
		];

		it('there are 18 sites to seed', () => {
			expect(sites).toHaveLength(18);
		});

		for (const u of sites) {
			for (const [name, rule, mutate] of seeds) {
				it(`seed: ${u.loc} (${u.value.split('@')[0]}) with ${name}`, () => {
					const d = docs();
					nodeAt(d, u.loc).uses = mutate(u);
					expect(allPinViolations(d)).toContain(`${rule}@${u.loc}`);
				});
			}
		}

		it('seed: the unlisted 40-hex SHA still matches the shape, so only the allowlist catches it', () => {
			const d = docs();
			nodeAt(d, sites[0].loc).uses = `${sites[0].value.split('@')[0]}@${UNLISTED_SHA}`;
			const v = allPinViolations(d);
			expect(v).toContain(`uses-not-on-pin-allowlist@${sites[0].loc}`);
			expect(v).not.toContain(`uses-not-sha-pinned@${sites[0].loc}`);
		});

		it('seed: an uppercase-hex SHA is not a lowercase pin', () => {
			const d = docs();
			nodeAt(d, 'deploy.yml:test:0').uses = `actions/checkout@${ACTIONS.checkout.sha.toUpperCase()}`;
			expect(allPinViolations(d)).toContain('uses-not-sha-pinned@deploy.yml:test:0');
		});

		it('seed: a new workflow file with a tag-pinned uses: and a checkout without persist-credentials fails', () => {
			const d = docs();
			d['new.yml'] = {
				on: 'push',
				jobs: {
					j: { 'runs-on': 'ubuntu-latest', steps: [{ uses: 'actions/checkout@v6' }] }
				}
			};
			const v = allPinViolations(d);
			expect(v).toContain('uses-not-sha-pinned@new.yml:j:0');
			expect(v).toContain('checkout-pin@new.yml:j:0');
		});

		it('seed: a job-level uses: with a tag is caught', () => {
			const d = docs();
			jobs(d['test.yml']).test.uses = 'octo/reusable/.github/workflows/x.yml@v1';
			expect(allPinViolations(d)).toContain('uses-not-sha-pinned@test.yml:test:job');
		});
	});

	describe('seeds: the setup-flyctl version rule, once on the setup-flyctl step', () => {
		it('control: the seed locator is the setup-flyctl step', () => {
			expect(nodeAt(docs(), FLYCTL_LOC).uses).toBe(pair(ACTIONS.setupFlyctl));
		});

		it('seed: with.version removed', () => {
			const d = docs();
			delete (nodeAt(d, FLYCTL_LOC).with as Obj).version;
			expect(allPinViolations(d)).toContain(`flyctl-version@${FLYCTL_LOC}`);
		});

		it('seed: with removed', () => {
			const d = docs();
			delete nodeAt(d, FLYCTL_LOC).with;
			expect(allPinViolations(d)).toContain(`flyctl-version@${FLYCTL_LOC}`);
		});

		it.each(['latest', '0.5', '^0.5.10', '>=0.5.10', 'v0.5.10', '0.5.10-rc1', '0.5.10 '])(
			'seed: version %j is not an exact x.y.z',
			(version) => {
				const d = docs();
				(nodeAt(d, FLYCTL_LOC).with as Obj).version = version;
				expect(allPinViolations(d)).toContain(`flyctl-version@${FLYCTL_LOC}`);
			},
		);

		it('seed: a numeric version is not a string', () => {
			const d = docs();
			(nodeAt(d, FLYCTL_LOC).with as Obj).version = 0.5;
			expect(allPinViolations(d)).toContain(`flyctl-version@${FLYCTL_LOC}`);
		});
	});

	describe('seeds: the checkout pin, one seeded copy per seed and checkout step, over all seven', () => {
		const withSeeds: [string, (s: Obj) => void][] = [
			['with: removed', (s) => void delete s.with],
			['persist-credentials: true', (s) => void ((s.with as Obj)['persist-credentials'] = true)],
			["persist-credentials: 'false' (a string)", (s) => void ((s.with as Obj)['persist-credentials'] = 'false')],
			['an added token: key', (s) => void ((s.with as Obj).token = 'x')],
			['an added fetch-depth: key', (s) => void ((s.with as Obj)['fetch-depth'] = 0)],
		];

		for (const loc of CHECKOUT_LOCS) {
			for (const [name, mutate] of withSeeds) {
				it(`seed: ${loc} with ${name}`, () => {
					const d = docs();
					mutate(nodeAt(d, loc));
					expect(allPinViolations(d)).toEqual([`checkout-pin@${loc}`]);
				});
			}
		}

		it('seed: the pin keys on the action, not its version (a bumped checkout without with: is caught)', () => {
			const d = docs();
			const s = nodeAt(d, CHECKOUT_LOCS[0]);
			s.uses = 'actions/checkout@v7.0.0';
			delete s.with;
			expect(allPinViolations(d)).toContain(`checkout-pin@${CHECKOUT_LOCS[0]}`);
		});
	});
});

// ---- R-35, the dev-only guard steps (plan D9, D18) ----
//
// deny.toml ignores an advisory by ID across the whole graph, which is sound only while the
// package stays out of the shipped graph; rcgen is held to dev-only the same way. Each guard is
// one step that runs scripts/check-dev-only.sh with one package, and nothing may soften it: a step
// with `continue-on-error`, `if`, `shell` or `working-directory` is a guard that can be skipped.

type GuardSpec = { file: string; job: string; name: string; run: string };

const GUARDS: GuardSpec[] = [
	{ file: 'test.yml', job: 'test', name: 'Check chrono stays dev-only', run: 'scripts/check-dev-only.sh chrono' },
	{ file: 'test.yml', job: 'test', name: 'Check rcgen stays dev-only', run: 'scripts/check-dev-only.sh rcgen' },
	{ file: 'deploy.yml', job: 'test', name: 'Check rcgen stays dev-only', run: 'scripts/check-dev-only.sh rcgen' },
];

/** A guard step has exactly the keys `name` and `run`; any other key is a violation. */
function guardViolations(docs: Record<string, Workflow>): string[] {
	const v: string[] = [];
	for (const g of GUARDS) {
		const loc = `${g.file}:${g.job}:${g.name}`;
		const doc = docs[g.file];
		const matches = stepsOf(doc ? jobs(doc)[g.job] : undefined).filter((s) => s.name === g.name);
		if (matches.length !== 1) {
			v.push(`guard-step-count@${loc}=${matches.length}`);
			continue;
		}
		const step = matches[0];
		if (step.run !== g.run) v.push(`guard-run@${loc}`);
		if (!isDeepStrictEqual(Object.keys(step).sort(), ['name', 'run'])) v.push(`guard-keys@${loc}`);
	}
	return v;
}

describe('R-35: the dev-only guard steps', () => {
	const docs = () => clone(loadAll());
	const guardIn = (d: Record<string, Workflow>, g: GuardSpec): Step =>
		stepsOf(jobs(d[g.file])[g.job]).find((s) => s.name === g.name) as Step;

	it('the real workflows hold every guard step, exactly', () => {
		expect(guardViolations(docs())).toEqual([]);
	});

	for (const g of GUARDS) {
		const loc = `${g.file}:${g.job}:${g.name}`;
		it(`the real ${g.file} ${g.job} job has "${g.name}" running exactly ${g.run}`, () => {
			const step = guardIn(docs(), g);
			expect(step.run).toBe(g.run);
			expect(Object.keys(step).sort()).toEqual(['name', 'run']);
		});

		it(`seed: ${loc} removed`, () => {
			const d = docs();
			const steps = stepsOf(jobs(d[g.file])[g.job]);
			steps.splice(steps.indexOf(guardIn(d, g)), 1);
			expect(guardViolations(d)).toEqual([`guard-step-count@${loc}=0`]);
		});

		it(`seed: ${loc} renamed`, () => {
			const d = docs();
			guardIn(d, g).name = `${g.name} (renamed)`;
			expect(guardViolations(d)).toEqual([`guard-step-count@${loc}=0`]);
		});

		it(`seed: ${loc} with another run`, () => {
			const d = docs();
			guardIn(d, g).run = `${g.run} || true`;
			expect(guardViolations(d)).toEqual([`guard-run@${loc}`]);
		});

		it(`seed: ${loc} runs the script for another package`, () => {
			const d = docs();
			guardIn(d, g).run = g.run.replace(/ \w+$/, ' serde');
			expect(guardViolations(d)).toEqual([`guard-run@${loc}`]);
		});

		const forbidden: [string, Json][] = [
			['continue-on-error', true],
			['if', '${{ always() }}'],
			['shell', 'bash'],
			['working-directory', 'frontend'],
			['env', { X: '1' }],
			['with', { x: '1' }],
		];
		for (const [key, value] of forbidden) {
			it(`seed: ${loc} gains the key ${key}`, () => {
				const d = docs();
				guardIn(d, g)[key] = value;
				expect(guardViolations(d)).toEqual([`guard-keys@${loc}`]);
			});
		}
	}
});

// ---- R-35, the freshness check jobs (plan D9, T13) ----
//
// The two check jobs run a Node script against the live site, so what they may hold and what
// the scripts may import are controls. Allowlist polarity throughout: a job key, a step key, an
// import specifier or a trigger the author did not enumerate is a violation. Each rule is proven
// on seeded copies of an embedded compliant baseline, so a seed cannot fail for an unrelated
// reason, and the real files are checked against the same function.
//
// What the scans establish, exactly: the scripts import nothing but `node:timers/promises` by a
// literal specifier, pass `fetch` only `redirect` and `signal` (the scripts' own specs), and
// contain none of the scanned tokens. A TLS or proxy setting reached by a route none of these
// covers is not checked.

const SCRIPTS_DIR = resolve(import.meta.dirname, '../../../scripts');
const DAILY_FILE = 'freshness-daily.yml';

type CheckSpec = { file: string; job: string; script: string };
const DAILY: CheckSpec = {
	file: DAILY_FILE,
	job: 'freshness',
	script: 'frontend/scripts/check-freshness-daily.mjs'
};
const POST_DEPLOY: CheckSpec = {
	file: 'deploy.yml',
	job: 'post-deploy-freshness',
	script: 'frontend/scripts/check-freshness-post-deploy.mjs'
};
const CHECKS: CheckSpec[] = [DAILY, POST_DEPLOY];

const CHECK_JOB_KEYS = ['name', 'runs-on', 'needs', 'permissions', 'timeout-minutes', 'steps'];
const CHECK_STEP_KEYS = ['name', 'uses', 'with', 'run'];
const DAILY_CRON_RE = /^\d{1,2} \d{1,2} \* \* \*$/;
const IMPORT_ALLOWLIST = ['node:timers/promises'];
const FORBIDDEN_TOKENS = [
	'process.env',
	'process[',
	'globalThis',
	'global',
	'NODE_TLS',
	'Symbol.for',
	'getBuiltinModule',
	'dispatcher',
	'eval(',
	'Function('
];

const tokenRefs = (leaf: string): boolean =>
	expressions(leaf).some(
		(e) =>
			/\bsecrets\b/.test(e) ||
			/\bgithub\s*\.\s*token\b/i.test(e) ||
			/\bgithub\s*\[\s*(['"])token\1\s*\]/i.test(e) ||
			/\btoJSON\s*\(\s*github\s*\)/i.test(e)
	);

/** Every R-35 check-job rule, over the daily workflow and deploy.yml. Rule ids carry a locator. */
function checkJobViolations(docs: Record<string, Workflow>): string[] {
	const v: string[] = [];
	const permsOk = (p: Json | undefined) => isDeepStrictEqual(p, { contents: 'read' });

	for (const c of CHECKS) {
		const loc = `${c.file}:${c.job}`;
		const doc = docs[c.file];
		const job = doc ? jobs(doc)[c.job] : undefined;
		if (!doc || !job) {
			v.push(`job-missing@${loc}`);
			continue;
		}

		// workflow level: no env, no secret or token reference outside the jobs
		if ('env' in doc) v.push(`workflow-env@${c.file}`);
		for (const [k, val] of Object.entries(doc)) {
			if (k === 'jobs') continue;
			const leaves: { path: string[]; text: string }[] = [];
			walk(val as Json, [k], leaves);
			if (leaves.some((l) => tokenRefs(l.text))) v.push(`secret-reference@${c.file}:workflow`);
		}

		// triggers, permissions
		if (c === DAILY) {
			const on = doc.on as Obj | undefined;
			const keys = on && typeof on === 'object' ? Object.keys(on) : [];
			const sched = on?.schedule;
			if (!Array.isArray(sched) || sched.length === 0) v.push(`daily-no-schedule@${c.file}`);
			else if (
				!sched.every(
					(s) =>
						typeof s === 'object' &&
						s !== null &&
						!Array.isArray(s) &&
						isDeepStrictEqual(Object.keys(s), ['cron']) &&
						typeof s.cron === 'string' &&
						DAILY_CRON_RE.test(s.cron)
				)
			)
				v.push(`daily-cron-not-daily@${c.file}`);
			if (!keys.includes('workflow_dispatch')) v.push(`daily-no-dispatch@${c.file}`);
			if (keys.some((k) => k !== 'schedule' && k !== 'workflow_dispatch')) v.push(`daily-extra-trigger@${c.file}`);
			if (!permsOk(doc.permissions as Json | undefined)) v.push(`workflow-permissions@${c.file}`);
			if ('permissions' in job && !permsOk(job.permissions)) v.push(`job-permissions@${loc}`);
		} else if ('permissions' in job) {
			v.push(`post-deploy-sets-permissions@${loc}`);
		}
		if (c === POST_DEPLOY && job.needs !== 'deploy') v.push(`needs@${loc}`);

		// no environment, no env, no secret, on the job or its steps
		if ('environment' in job) v.push(`environment@${loc}`);
		if ('env' in job) v.push(`env@${loc}`);
		const steps = stepsOf(job);
		steps.forEach((s, i) => {
			if ('env' in s) v.push(`env@${loc}:${i}`);
		});
		const leaves: { path: string[]; text: string }[] = [];
		walk(job as Json, [c.job], leaves);
		if (leaves.some((l) => tokenRefs(l.text))) v.push(`secret-reference@${loc}`);

		// the run, the time limit
		const runs = steps.filter((s) => 'run' in s);
		if (runs.length !== 1) v.push(`run-count@${loc}=${runs.length}`);
		steps.forEach((s, i) => {
			if ('run' in s && s.run !== `node ${c.script}`) v.push(`run-not-exact@${loc}:${i}`);
		});
		if (job['timeout-minutes'] !== 10) v.push(`timeout-not-10@${loc}`);

		// key allowlists
		for (const k of Object.keys(job)) if (!CHECK_JOB_KEYS.includes(k)) v.push(`job-key:${k}@${loc}`);
		steps.forEach((s, i) => {
			for (const k of Object.keys(s)) if (!CHECK_STEP_KEYS.includes(k)) v.push(`step-key:${k}@${loc}:${i}`);
		});
	}
	return [...new Set(v)];
}

/**
 * Imports, static or dynamic: literal specifiers only, and only from the allowlist.
 *
 * Polarity is permit-the-enumerated: every occurrence of the word `import` (other than
 * `import.meta`) must be consumed by one strict accepted form, and so must every `export` that
 * re-exports `from` a module. An occurrence no accepted form consumes is a violation, so a
 * spelling the scanner did not anticipate (no whitespace, a comment inside the statement) fails
 * rather than passing unseen. The cost is that the word `import` may not appear in a script's
 * comments; the two scripts are worded to avoid it.
 */
function importViolations(src: string): string[] {
	const v: string[] = [];
	const judge = (spec: string) => {
		if (!IMPORT_ALLOWLIST.includes(spec)) v.push(`import-not-allowed:${spec}`);
	};
	const STATIC = /^import\s*(?:[\w$*{}\s,]+?\s*from\s*)?(['"])([^'"`\\]*)\1/;
	const DYNAMIC = /^import\s*\(\s*(['"])([^'"`\\]*)\1\s*\)/;
	for (const m of src.matchAll(/\bimport\b(?!\s*\.\s*meta\b)/g)) {
		const rest = src.slice(m.index);
		const hit = DYNAMIC.exec(rest) ?? STATIC.exec(rest);
		if (hit) judge(hit[2]);
		else v.push('import-non-literal');
	}
	const REEXPORT = /^export\s*(?:\*(?:\s*as\s+[\w$]+)?|\{[^}]*\})\s*from\s*(['"])([^'"`\\]*)\1/;
	for (const m of src.matchAll(/\bexport\b/g)) {
		const rest = src.slice(m.index);
		const hit = REEXPORT.exec(rest);
		if (hit) judge(hit[2]);
		else if (/\bfrom\b/.test(rest.split(';')[0])) v.push('import-non-literal');
	}
	if (/\brequire\s*\(/.test(src)) v.push('import-non-literal');
	return [...new Set(v)];
}

const tokenViolations = (src: string): string[] =>
	FORBIDDEN_TOKENS.filter((t) => src.includes(t)).map((t) => `token:${t}`);

const readScript = (script: string): string => {
	try {
		return readFileSync(resolve(SCRIPTS_DIR, script.replace('frontend/scripts/', '')), 'utf8');
	} catch {
		return '';
	}
};

const checkoutStep = (): Step => ({
	uses: pair(ACTIONS.checkout),
	with: { 'persist-credentials': false }
});
const nodeStep = (): Step => ({ uses: pair(ACTIONS.setupNode), with: { 'node-version': 22 } });

/** A compliant pair of workflow documents, independent of the real files. */
function baselineDocs(): Record<string, Workflow> {
	return {
		[DAILY_FILE]: {
			name: 'Freshness (daily)',
			on: { schedule: [{ cron: '17 6 * * *' }], workflow_dispatch: null },
			permissions: { contents: 'read' },
			jobs: {
				[DAILY.job]: {
					name: 'Daily list freshness check',
					'runs-on': 'ubuntu-latest',
					'timeout-minutes': 10,
					steps: [checkoutStep(), nodeStep(), { name: 'Check freshness', run: `node ${DAILY.script}` }]
				}
			}
		},
		'deploy.yml': {
			name: 'Deploy',
			on: { push: { branches: ['main'] } },
			permissions: { contents: 'read' },
			jobs: {
				deploy: { name: 'Deploy to Fly.io', 'runs-on': 'ubuntu-latest', steps: [] },
				[POST_DEPLOY.job]: {
					name: 'Post-deploy list freshness check',
					'runs-on': 'ubuntu-latest',
					needs: 'deploy',
					'timeout-minutes': 10,
					steps: [checkoutStep(), nodeStep(), { name: 'Check freshness', run: `node ${POST_DEPLOY.script}` }]
				}
			}
		}
	};
}

describe('R-35: the freshness check jobs, the real files', () => {
	it('the real workflows raise no check-job violation', () => {
		expect(checkJobViolations(loadAll())).toEqual([]);
	});

	it('control: the compliant baseline raises none', () => {
		expect(checkJobViolations(baselineDocs())).toEqual([]);
	});

	it('control: the checker reports a violation on a known-bad input (no workflows)', () => {
		expect(checkJobViolations({})).toEqual([
			`job-missing@${DAILY.file}:${DAILY.job}`,
			`job-missing@${POST_DEPLOY.file}:${POST_DEPLOY.job}`
		]);
	});

	it('the daily workflow file exists and the post-deploy job sits in deploy.yml', () => {
		const d = loadAll();
		expect(Object.keys(d)).toContain(DAILY_FILE);
		expect(Object.keys(jobs(d['deploy.yml']))).toContain(POST_DEPLOY.job);
	});

	it('each check script exists', () => {
		for (const c of CHECKS) expect(readScript(c.script), c.script).not.toBe('');
	});
});

describe('R-35: the freshness check jobs, seeded violations', () => {
	type Seed = [name: string, rule: string, mutate: (d: Record<string, Workflow>) => void];
	const dailyJob = (d: Record<string, Workflow>) => jobs(d[DAILY_FILE])[DAILY.job];
	const postJob = (d: Record<string, Workflow>) => jobs(d['deploy.yml'])[POST_DEPLOY.job];
	const runStep = (j: Obj) => stepsOf(j).find((s) => 'run' in s) as Step;
	const dailyLoc = `${DAILY.file}:${DAILY.job}`;
	const postLoc = `${POST_DEPLOY.file}:${POST_DEPLOY.job}`;
	const dailyOn = (d: Record<string, Workflow>) => d[DAILY_FILE].on as Obj;

	const seeds: Seed[] = [
		// the daily workflow's triggers
		['daily: no schedule', `daily-no-schedule@${DAILY_FILE}`, (d) => void delete dailyOn(d).schedule],
		['daily: empty schedule list', `daily-no-schedule@${DAILY_FILE}`, (d) => void (dailyOn(d).schedule = [])],
		['daily: a schedule with no cron', `daily-cron-not-daily@${DAILY_FILE}`, (d) => void (dailyOn(d).schedule = [{}])],
		['daily: a cron every five minutes', `daily-cron-not-daily@${DAILY_FILE}`, (d) => void (dailyOn(d).schedule = [{ cron: '*/5 * * * *' }])],
		['daily: an hourly cron', `daily-cron-not-daily@${DAILY_FILE}`, (d) => void (dailyOn(d).schedule = [{ cron: '17 * * * *' }])],
		['daily: a weekly cron', `daily-cron-not-daily@${DAILY_FILE}`, (d) => void (dailyOn(d).schedule = [{ cron: '17 6 * * 1' }])],
		['daily: a non-string cron', `daily-cron-not-daily@${DAILY_FILE}`, (d) => void (dailyOn(d).schedule = [{ cron: 5 }])],
		['daily: no workflow_dispatch', `daily-no-dispatch@${DAILY_FILE}`, (d) => void delete dailyOn(d).workflow_dispatch],
		['daily: a push trigger', `daily-extra-trigger@${DAILY_FILE}`, (d) => void (dailyOn(d).push = null)],
		['daily: a pull_request_target trigger', `daily-extra-trigger@${DAILY_FILE}`, (d) => void (dailyOn(d).pull_request_target = null)],
		// the daily workflow's permissions
		['daily: workflow permissions broadened to write-all', `workflow-permissions@${DAILY_FILE}`, (d) => void (d[DAILY_FILE].permissions = 'write-all')],
		['daily: workflow permissions gain a second key', `workflow-permissions@${DAILY_FILE}`, (d) => void ((d[DAILY_FILE].permissions as Obj)['id-token'] = 'write')],
		['daily: workflow contents: write', `workflow-permissions@${DAILY_FILE}`, (d) => void ((d[DAILY_FILE].permissions as Obj).contents = 'write')],
		['daily: workflow permissions removed', `workflow-permissions@${DAILY_FILE}`, (d) => void delete d[DAILY_FILE].permissions],
		['daily: workflow permissions empty', `workflow-permissions@${DAILY_FILE}`, (d) => void (d[DAILY_FILE].permissions = {})],
		['daily: job permissions contents: write', `job-permissions@${dailyLoc}`, (d) => void (dailyJob(d).permissions = { contents: 'write' })],
		['daily: job permissions gain a second key', `job-permissions@${dailyLoc}`, (d) => void (dailyJob(d).permissions = { contents: 'read', actions: 'write' })],
		// the post-deploy job
		['post-deploy: the job sets permissions', `post-deploy-sets-permissions@${postLoc}`, (d) => void (postJob(d).permissions = { contents: 'read' })],
		['post-deploy: needs removed', `needs@${postLoc}`, (d) => void delete postJob(d).needs],
		['post-deploy: needs test', `needs@${postLoc}`, (d) => void (postJob(d).needs = 'test')],
		['post-deploy: needs a list', `needs@${postLoc}`, (d) => void (postJob(d).needs = ['deploy'])]
	];

	const perJob: [string, (d: Record<string, Workflow>) => Obj, string, CheckSpec, CheckSpec][] = [
		[DAILY_FILE, dailyJob, dailyLoc, DAILY, POST_DEPLOY],
		['deploy.yml', postJob, postLoc, POST_DEPLOY, DAILY]
	];
	for (const [file, jobOf, loc, self, other] of perJob) {
		const tag = file === DAILY_FILE ? 'daily' : 'post-deploy';
		const jobSeeds: Seed[] = [
			// environment, env, secrets
			[`${tag}: an environment`, `environment@${loc}`, (d) => void (jobOf(d).environment = 'production')],
			[`${tag}: job env`, `env@${loc}`, (d) => void (jobOf(d).env = { X: '1' })],
			[`${tag}: step env`, `env@${loc}:2`, (d) => void (runStep(jobOf(d)).env = { X: '1' })],
			[`${tag}: step env on the checkout step`, `env@${loc}:0`, (d) => void (stepsOf(jobOf(d))[0].env = { X: '1' })],
			[`${tag}: workflow-level env`, `workflow-env@${file}`, (d) => void (d[file].env = { X: '1' })],
			[`${tag}: a secret in a with value`, `secret-reference@${loc}`, (d) => void ((stepsOf(jobOf(d))[0].with as Obj).token = '${{ secrets.GITHUB_TOKEN }}')],
			[`${tag}: a secret in the run`, `secret-reference@${loc}`, (d) => void (runStep(jobOf(d)).run = 'echo ${{ secrets.FLY_API_TOKEN }}')],
			[`${tag}: toJSON(secrets)`, `secret-reference@${loc}`, (d) => void ((stepsOf(jobOf(d))[1].with as Obj).x = '${{ toJSON(secrets) }}')],
			[`${tag}: secrets['X']`, `secret-reference@${loc}`, (d) => void ((stepsOf(jobOf(d))[1].with as Obj).x = "${{ secrets['X'] }}")],
			[`${tag}: github.token`, `secret-reference@${loc}`, (d) => void ((stepsOf(jobOf(d))[1].with as Obj).x = '${{ github.token }}')],
			[`${tag}: toJSON(github)`, `secret-reference@${loc}`, (d) => void ((stepsOf(jobOf(d))[1].with as Obj).x = '${{ toJSON(github) }}')],
			[`${tag}: a secret at workflow level`, `secret-reference@${file}:workflow`, (d) => void (d[file].name = '${{ secrets.X }}')],
			// the run, exactly
			[`${tag}: another script`, `run-not-exact@${loc}:2`, (d) => void (runStep(jobOf(d)).run = 'node frontend/scripts/other.mjs')],
			[`${tag}: the other check's script`, `run-not-exact@${loc}:2`, (d) => void (runStep(jobOf(d)).run = `node ${other.script}`)],
			[`${tag}: run with || true`, `run-not-exact@${loc}:2`, (d) => void (runStep(jobOf(d)).run = `node ${self.script} || true`)],
			[`${tag}: run with an argument`, `run-not-exact@${loc}:2`, (d) => void (runStep(jobOf(d)).run = `node ${self.script} --lenient`)],
			[`${tag}: run with a second command`, `run-not-exact@${loc}:2`, (d) => void (runStep(jobOf(d)).run = `node ${self.script}\ncurl https://example.test`)],
			[`${tag}: a second run step`, `run-count@${loc}=2`, (d) => void stepsOf(jobOf(d)).push({ name: 'extra', run: 'echo hi' })],
			[`${tag}: the run step removed`, `run-count@${loc}=0`, (d) => void stepsOf(jobOf(d)).splice(2, 1)],
			// the time limit
			[`${tag}: timeout-minutes removed`, `timeout-not-10@${loc}`, (d) => void delete jobOf(d)['timeout-minutes']],
			[`${tag}: timeout-minutes 11`, `timeout-not-10@${loc}`, (d) => void (jobOf(d)['timeout-minutes'] = 11)],
			[`${tag}: timeout-minutes "10" as a string`, `timeout-not-10@${loc}`, (d) => void (jobOf(d)['timeout-minutes'] = '10')]
		];
		for (const k of [
			'continue-on-error',
			'if',
			'shell',
			'working-directory',
			'environment',
			'container',
			'services',
			'strategy',
			'defaults',
			'outputs',
			'concurrency',
			'uses',
			'secrets',
			'env'
		])
			jobSeeds.push([`${tag}: job key ${k}`, `job-key:${k}@${loc}`, (d) => void (jobOf(d)[k] = k === 'if' ? 'always()' : 'x')]);
		for (const k of ['continue-on-error', 'if', 'shell', 'working-directory', 'env', 'id', 'timeout-minutes'])
			for (const i of [0, 1, 2])
				jobSeeds.push([
					`${tag}: step ${i} key ${k}`,
					`step-key:${k}@${loc}:${i}`,
					(d) => void (stepsOf(jobOf(d))[i][k] = k === 'if' ? 'always()' : 'x')
				]);
		seeds.push(...jobSeeds);
	}

	for (const [name, rule, mutate] of seeds) {
		it(`seed: ${name}`, () => {
			const d = clone(baselineDocs());
			mutate(d);
			expect(checkJobViolations(d)).toContain(rule);
		});
	}

	it('the key seeds cover continue-on-error, if, shell and working-directory at job and step level, for each job', () => {
		for (const k of ['continue-on-error', 'if', 'shell', 'working-directory']) {
			for (const loc of [dailyLoc, postLoc]) {
				expect(seeds.some((s) => s[1] === `job-key:${k}@${loc}`), `job ${k} ${loc}`).toBe(true);
				expect(seeds.some((s) => s[1].startsWith(`step-key:${k}@${loc}:`)), `step ${k} ${loc}`).toBe(true);
			}
		}
	});
});

describe('R-35: the check scripts, imports and forbidden tokens', () => {
	const SEED_SOURCE = "import { setTimeout as sleep } from 'node:timers/promises';\nconst x = 1;\n";

	it('control: the allowlisted import by a literal specifier raises nothing', () => {
		expect(importViolations(SEED_SOURCE)).toEqual([]);
		expect(importViolations("const m = await import('node:timers/promises');")).toEqual([]);
		expect(importViolations('const x = 1;')).toEqual([]);
	});

	it.each(CHECKS.map((c) => [c.script]))('the real %s imports nothing outside the allowlist', (script) => {
		const src = readScript(script);
		expect(src).not.toBe('');
		expect(importViolations(src)).toEqual([]);
	});

	const importSeeds: [string, string, string][] = [
		['a static import of another builtin', "import fs from 'node:fs';", 'import-not-allowed:node:fs'],
		['a static import of the bare timers/promises name', "import { setTimeout } from 'timers/promises';", 'import-not-allowed:timers/promises'],
		['a side-effect import', "import 'node:fs';", 'import-not-allowed:node:fs'],
		['a relative import', "import x from './other.mjs';", 'import-not-allowed:./other.mjs'],
		['a package import', "import x from 'left-pad';", 'import-not-allowed:left-pad'],
		['a namespace import', "import * as os from 'node:os';", 'import-not-allowed:node:os'],
		['a re-export', "export * from 'node:fs';", 'import-not-allowed:node:fs'],
		['a named re-export', "export { readFile } from 'node:fs/promises';", 'import-not-allowed:node:fs/promises'],
		['a dynamic import of another builtin', "const m = await import('node:fs');", 'import-not-allowed:node:fs'],
		['a dynamic import by a variable', 'const m = await import(spec);', 'import-non-literal'],
		['a dynamic import by a concatenation', "const m = await import('node:' + 'fs');", 'import-non-literal'],
		['a dynamic import by a template literal', 'const m = await import(`node:fs`);', 'import-non-literal'],
		['a dynamic import by a call', 'const m = await import(pick());', 'import-non-literal'],
		['a require call', "const fs = require('node:fs');", 'import-non-literal'],
		// forms with no whitespace, or a comment, where a pattern for the usual spelling would miss
		['a static import with no whitespace', "import{a}from'node:fs';", 'import-not-allowed:node:fs'],
		['a side-effect import with no whitespace', "import'node:fs';", 'import-not-allowed:node:fs'],
		['a dynamic import with a comment before the paren', "const m = await import /*x*/ ('node:fs');", 'import-non-literal'],
		['a dynamic import with a comment inside the paren', "const m = await import(/*x*/'node:fs');", 'import-non-literal'],
		['a re-export with no whitespace', "export*from'node:fs';", 'import-not-allowed:node:fs'],
		['the word import in prose (the catch-all fails closed on what it cannot read)', '// an import of nothing in particular', 'import-non-literal'],
		['a default import with a tab', "import\tfs\tfrom\t'node:fs';", 'import-not-allowed:node:fs'],
		['an import followed by a newline and a path', "import fs from\n'node:fs';", 'import-not-allowed:node:fs']
	];
	for (const [name, line, rule] of importSeeds) {
		it(`seed: ${name}`, () => {
			expect(importViolations(`${SEED_SOURCE}${line}\n`)).toContain(rule);
		});
	}

	it.each(CHECKS.map((c) => [c.script]))('the real %s contains none of the forbidden tokens', (script) => {
		const src = readScript(script);
		expect(src).not.toBe('');
		expect(tokenViolations(src)).toEqual([]);
	});

	const tokenSeeds: Record<string, string> = {
		'process.env': 'const a = process.env.X;',
		'process[': "const a = process['env'];",
		globalThis: 'const a = globalThis.fetch;',
		global: 'const a = global.fetch;',
		NODE_TLS: "const a = 'NODE_TLS_REJECT_UNAUTHORIZED';",
		'Symbol.for': "const a = Symbol.for('x');",
		getBuiltinModule: "const a = process.getBuiltinModule('node:fs');",
		dispatcher: 'const a = { dispatcher: x };',
		'eval(': "const a = eval('1');",
		'Function(': "const a = Function('return 1')();"
	};
	it('the seed table has one seed per forbidden token', () => {
		expect(Object.keys(tokenSeeds).sort()).toEqual([...FORBIDDEN_TOKENS].sort());
	});
	for (const token of FORBIDDEN_TOKENS) {
		it(`seed: the token ${token}`, () => {
			expect(tokenViolations(`${SEED_SOURCE}${tokenSeeds[token]}\n`)).toContain(`token:${token}`);
		});
	}
});
