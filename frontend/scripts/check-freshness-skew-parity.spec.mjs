// @vitest-environment node
//
// R-34 (docs/specs/2026-11-18-live-lists-and-blog-move.md): the checks' skew allowance and the
// server's ahead-of-now bound are one value, 5 minutes, and a test fails when they differ.
// Cargo is not run here: the Rust constant is read from the source file as text.
import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const RUST = resolve(import.meta.dirname, '../../libs/site-core/projects_list.rs');
const SCRIPTS = ['check-freshness-daily.mjs', 'check-freshness-post-deploy.mjs'];
const AGREED_MINUTES = 5;

/** The minutes in `pub const MAX_AGE_AHEAD: Duration = Duration::minutes(N);`, or null. */
export function rustMinutes(src) {
	const m = /\bconst\s+MAX_AGE_AHEAD\s*:\s*Duration\s*=\s*Duration::minutes\(\s*(\d+)\s*\)\s*;/.exec(src);
	return m ? Number(m[1]) : null;
}

/** The minutes in `const SKEW_MINUTES = N;` of a check script, or null. */
export function nodeMinutes(src) {
	const m = /^const\s+SKEW_MINUTES\s*=\s*(\d+)\s*;/m.exec(src);
	return m ? Number(m[1]) : null;
}

/** Every way the Rust bound and the scripts' allowances fail to be the one agreed value. */
export function parityViolations(rustSrc, nodeSrcs) {
	const v = [];
	const r = rustMinutes(rustSrc);
	if (r === null) v.push('rust-unreadable');
	else if (r !== AGREED_MINUTES) v.push(`rust-not-${AGREED_MINUTES}`);
	for (const [name, src] of Object.entries(nodeSrcs)) {
		const n = nodeMinutes(src);
		if (n === null) v.push(`node-unreadable@${name}`);
		else if (n !== AGREED_MINUTES) v.push(`node-not-${AGREED_MINUTES}@${name}`);
		if (r !== null && n !== null && r !== n) v.push(`differ@${name}`);
	}
	return v;
}

const readScript = (name) => {
	const p = resolve(import.meta.dirname, name);
	return existsSync(p) ? readFileSync(p, 'utf8') : '';
};
const realNode = () => Object.fromEntries(SCRIPTS.map((n) => [n, readScript(n)]));
const realRust = () => readFileSync(RUST, 'utf8');

describe('R-34: skew parity between the Rust bound and the two check scripts', () => {
	it('the real files agree, and the value is 5 minutes', () => {
		expect(parityViolations(realRust(), realNode())).toEqual([]);
		expect(rustMinutes(realRust())).toBe(5);
		for (const s of SCRIPTS) expect(nodeMinutes(readScript(s)), s).toBe(5);
	});

	it('control: the extractors read a known-good pair of fixtures', () => {
		expect(rustMinutes('pub const MAX_AGE_AHEAD: Duration = Duration::minutes(5);')).toBe(5);
		expect(nodeMinutes('const SKEW_MINUTES = 5;\nconst SKEW_MS = SKEW_MINUTES * 60_000;')).toBe(5);
		expect(parityViolations('pub const MAX_AGE_AHEAD: Duration = Duration::minutes(5);', { a: 'const SKEW_MINUTES = 5;' })).toEqual([]);
	});

	it('seed: the Rust bound differs from the scripts', () => {
		const rust = realRust().replace(/Duration::minutes\(5\)/, 'Duration::minutes(6)');
		expect(rust).not.toBe(realRust());
		const v = parityViolations(rust, realNode());
		expect(v).toContain('rust-not-5');
		for (const s of SCRIPTS) expect(v).toContain(`differ@${s}`);
	});

	for (const s of SCRIPTS) {
		it(`seed: ${s} differs from the Rust bound`, () => {
			const node = realNode();
			node[s] = node[s].replace(/^const SKEW_MINUTES = 5;/m, 'const SKEW_MINUTES = 4;');
			expect(node[s]).not.toBe(realNode()[s]);
			const v = parityViolations(realRust(), node);
			expect(v).toContain(`differ@${s}`);
			expect(v).toContain(`node-not-5@${s}`);
		});

		it(`seed: ${s} with the constant renamed is unreadable, not silently equal`, () => {
			const node = realNode();
			node[s] = node[s].replace(/SKEW_MINUTES/g, 'SKEW_MIN');
			expect(parityViolations(realRust(), node)).toContain(`node-unreadable@${s}`);
		});
	}

	it('seed: the Rust constant renamed is unreadable', () => {
		const rust = realRust().replace(/MAX_AGE_AHEAD/g, 'MAX_AHEAD');
		expect(parityViolations(rust, realNode())).toContain('rust-unreadable');
	});
});
