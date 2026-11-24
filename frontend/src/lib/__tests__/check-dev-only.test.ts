/**
 * scripts/check-dev-only.sh, run for each of its outcomes (live-lists plan T9;
 * Warden F-5, Cerberus M7).
 *
 * The script is a control: it holds a package out of the shipped dependency
 * graph, and a control that has never been seen to fail may not work. These
 * run the real script against this repository's real Cargo.lock, so each
 * outcome is a seeded run and not a reading of the code:
 *  - a package in the shipped graph (reqwest, a normal dependency of site-core) exits 1;
 *  - a dev-only package (rcgen) exits 0;
 *  - a package absent from the lockfile exits 1, naming the removal;
 *  - no argument, or two, exits 1.
 *
 * Needs `cargo` on PATH, as the CI test job has it before `npm run test:unit`.
 */
import { describe, it, expect } from 'vitest';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const ROOT = resolve(import.meta.dirname, '../../../..');
const SCRIPT = 'scripts/check-dev-only.sh';

function run(args: string[]) {
	const r = spawnSync(SCRIPT, args, { cwd: ROOT, encoding: 'utf8', timeout: 120_000 });
	if (r.error) throw r.error;
	return { code: r.status, out: `${r.stdout}${r.stderr}` };
}

describe('scripts/check-dev-only.sh', () => {
	it('exits 1 for a package in the shipped graph', () => {
		const r = run(['reqwest']);
		expect(r.code).toBe(1);
		expect(r.out).toContain('is in the shipped dependency graph');
	}, 130_000);

	it('exits 0 for a dev-only package', () => {
		const r = run(['rcgen']);
		expect(r.code).toBe(0);
		expect(r.out).toContain('rcgen is not in the normal/build dependency graph');
	}, 130_000);

	it('exits 1 for a package absent from Cargo.lock, and says what to remove', () => {
		const r = run(['folio-no-such-package']);
		expect(r.code).toBe(1);
		expect(r.out).toContain('folio-no-such-package is no longer in Cargo.lock');
		// The step is named by what to search for: not every guard runs in every workflow.
		expect(r.out).toContain("grep -rn 'check-dev-only.sh folio-no-such-package' .github/workflows");
		expect(r.out).toContain('any ignore that depends on it');
	}, 130_000);

	it('exits 1 with no argument', () => {
		const r = run([]);
		expect(r.code).toBe(1);
		expect(r.out).toContain('usage: scripts/check-dev-only.sh <package>');
	});

	it('exits 1 with two arguments', () => {
		const r = run(['rcgen', 'reqwest']);
		expect(r.code).toBe(1);
		expect(r.out).toContain('usage: scripts/check-dev-only.sh <package>');
	});
});
