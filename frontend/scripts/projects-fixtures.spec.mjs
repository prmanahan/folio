// @vitest-environment node
//
// R-9 (spec docs/specs/2026-11-11-projects-from-github.md, plan T1 and D10):
// every repo name in every committed fixture of both kinds under the D10 root
// (frontend/scripts/projects-fixtures/) matches ^fixture-. Reads only the
// committed fixtures; the script under test is not involved, so this test is
// green from the moment the fixtures exist. Its guard against a vacuous pass
// is the "kinds found" assertions and the seeded-violation control.
//
// Fixture kinds, by file-name prefix: `api-*.json` (GitHub API responses,
// owner -> pages -> repo elements) and `list-*.json` (list files: groups ->
// repos). A .json file of neither kind fails the test.
import { describe, expect, it } from 'vitest';
import { mkdtempSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fixtureRoot } from './projects-test-support.mjs';

const PREFIX = /^fixture-/;

function namesIn(kind, doc) {
	if (kind === 'list') return doc.groups.flatMap((g) => g.repos.map((r) => r.name));
	return Object.values(doc).flatMap((o) => o.pages.flatMap((p) => p.body.map((r) => r.name)));
}

/** Scan a fixture root: returns { api: n, list: n, offenders: [...], unknown: [...] }. */
function scan(root) {
	const out = { api: 0, list: 0, offenders: [], unknown: [] };
	for (const f of readdirSync(root, { recursive: true })) {
		if (!String(f).endsWith('.json')) continue;
		const kind = /(^|\/)api-[^/]*\.json$/.test(f) ? 'api' : /(^|\/)list-[^/]*\.json$/.test(f) ? 'list' : null;
		if (!kind) {
			out.unknown.push(f);
			continue;
		}
		out[kind] += 1;
		const doc = JSON.parse(readFileSync(join(root, f), 'utf8'));
		for (const name of namesIn(kind, doc)) {
			if (typeof name !== 'string' || !PREFIX.test(name)) out.offenders.push(`${f}: ${JSON.stringify(name)}`);
		}
	}
	return out;
}

describe('R-9: committed fixtures live under the D10 root and every repo name matches ^fixture-', () => {
	it('finds at least one list-file fixture and one API-response fixture, and every name matches', () => {
		const r = scan(fixtureRoot);
		expect(r.list, 'list-file fixtures found').toBeGreaterThanOrEqual(1);
		expect(r.api, 'API-response fixtures found').toBeGreaterThanOrEqual(1);
		expect(r.unknown, 'a .json fixture of neither kind').toEqual([]);
		expect(r.offenders).toEqual([]);
	});

	it('control: the scanner reports a seeded offender in both kinds, and fails on an empty root', () => {
		const root = mkdtempSync(join(tmpdir(), 'projects-r9-'));
		writeFileSync(
			join(root, 'api-seed.json'),
			JSON.stringify({ mnemra: { pages: [{ link: null, body: [{ name: 'mnemra-core' }] }] } }),
		);
		writeFileSync(join(root, 'list-seed.json'), JSON.stringify({ groups: [{ owner: 'mnemra', repos: [{ name: 'folio' }] }] }));
		const seeded = scan(root);
		expect(seeded.offenders).toHaveLength(2);
		const empty = scan(mkdtempSync(join(tmpdir(), 'projects-r9-empty-')));
		expect(empty.api + empty.list).toBe(0);
	});
});
