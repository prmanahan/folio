/**
 * Wrong-shape and out-of-contract list documents on /projects show R-24's unavailable line, the two
 * owner links and no entries (docs/specs/2026-11-18-live-lists-and-blog-move.md). The document rules
 * are the Data Model of the adopted projects spec (docs/specs/2026-11-11-projects-from-github.md, blob
 * 32f66b75): two groups, mnemra then prmanahan, each with a repo; the name pattern; a description of at
 * most 350 code points with no rejected-class code point; no unknown key at any level.
 *
 * Reads, and why:
 *  - frontend/scripts/projects-fixtures/list-expected.json: a valid list file, mutated one field at a
 *    time so each case differs from valid in exactly one way.
 * Only global `fetch` is substituted (the network boundary); the page is rendered for real.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, waitFor } from '@testing-library/svelte';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import ProjectsPage from '../../routes/(public)/projects/+page.svelte';
import { alertLine, expectUnavailable } from './projects-page-support';

async function expectErrorState(container: HTMLElement) {
	await waitFor(() => expect(alertLine(container)).not.toBeNull());
	expectUnavailable(container);
}

type Json = null | boolean | number | string | Json[] | { [k: string]: Json };
type Repo = { name: Json; description: Json };
type Group = { owner: Json; repos: Repo[] };
type ListFile = { fetched_at?: Json; groups: Group[] };

const EXPECTED: ListFile = JSON.parse(
	readFileSync(resolve(import.meta.dirname, '../../../scripts/projects-fixtures/list-expected.json'), 'utf8'),
);

function valid(): ListFile {
	return structuredClone(EXPECTED);
}

function serveList(list: unknown) {
	vi.stubGlobal(
		'fetch',
		vi.fn(async (input: unknown) => {
			const url = typeof input === 'string' ? input : String((input as { url?: string }).url ?? input);
			if (url === '/api/lists/projects') {
				return new Response(JSON.stringify(list), { status: 200, headers: { 'content-type': 'application/json' } });
			}
			return new Response('nope', { status: 500 });
		}),
	);
}

beforeEach(() => {
	vi.unstubAllGlobals();
});
afterEach(() => {
	vi.unstubAllGlobals();
});

describe('R-24: a wrong-shape list file shows the unavailable line and no entries', () => {
	it('the fixture has at least one group with a repo, so each mutation below changes real data', () => {
		expect(EXPECTED.groups.length).toBeGreaterThan(0);
		expect(EXPECTED.groups[0].repos.length).toBeGreaterThan(0);
	});

	it('fetched_at missing', async () => {
		const list = valid();
		delete list.fetched_at;
		serveList(list);
		const { container } = render(ProjectsPage);
		await expectErrorState(container);
	});

	it('an owner that is a number', async () => {
		const list = valid();
		list.groups[0].owner = 42;
		serveList(list);
		const { container } = render(ProjectsPage);
		await expectErrorState(container);
	});

	it('a name that is a number', async () => {
		const list = valid();
		list.groups[0].repos[0].name = 7;
		serveList(list);
		const { container } = render(ProjectsPage);
		await expectErrorState(container);
	});

	it('a description that is null', async () => {
		const list = valid();
		list.groups[0].repos[0].description = null;
		serveList(list);
		const { container } = render(ProjectsPage);
		await expectErrorState(container);
	});

	it('a duplicate repo name within one owner', async () => {
		const list = valid();
		const repos = list.groups[0].repos;
		repos.push({ name: repos[0].name, description: 'second entry with the same name' });
		serveList(list);
		const { container } = render(ProjectsPage);
		await expectErrorState(container);
	});
});

describe('Data Model: one rule per case, each shows the unavailable line', () => {
	const rejects: [string, (l: ListFile) => void][] = [
		['no groups (an empty list is not a valid document)', (l) => (l.groups = [])],
		['one group only', (l) => l.groups.splice(1)],
		['three groups', (l) => l.groups.push({ owner: 'mnemra', repos: [{ name: 'fixture-x', description: '' }] })],
		['owners swapped: prmanahan then mnemra', (l) => l.groups.reverse()],
		['an owner other than mnemra or prmanahan', (l) => (l.groups[1].owner = 'someone-else')],
		['an owner repeated', (l) => (l.groups[1].owner = 'mnemra')],
		['an owner with an empty repos array', (l) => (l.groups[1].repos = [])],
		['a name with a space', (l) => (l.groups[0].repos[0].name = 'fixture a')],
		['a name with a slash', (l) => (l.groups[0].repos[0].name = 'fixture/a')],
		['an empty name', (l) => (l.groups[0].repos[0].name = '')],
		['a name of 101 characters', (l) => (l.groups[0].repos[0].name = 'a'.repeat(101))],
		['a description of 351 code points', (l) => (l.groups[0].repos[0].description = 'a'.repeat(351))],
		[
			'a description of 351 code points made of astral characters (code points, not UTF-16 units)',
			(l) => (l.groups[0].repos[0].description = '\u{1F600}'.repeat(351)),
		],
		['a description with a newline (Cc)', (l) => (l.groups[0].repos[0].description = 'a\nb')],
		['a description with DEL (Cc)', (l) => (l.groups[0].repos[0].description = 'a\u007fb')],
		['a description with U+0085 (C1 control)', (l) => (l.groups[0].repos[0].description = 'a\u0085b')],
		['a description with U+2028', (l) => (l.groups[0].repos[0].description = 'a\u2028b')],
		['a description with U+2029', (l) => (l.groups[0].repos[0].description = 'a\u2029b')],
		['a description with a zero-width space (invisible table)', (l) => (l.groups[0].repos[0].description = 'a\u200bb')],
		['a description with a bidi control (invisible table)', (l) => (l.groups[0].repos[0].description = 'a\u202eb')],
		['a description with a lone high surrogate', (l) => (l.groups[0].repos[0].description = 'a\ud800b')],
		['a description with a lone low surrogate', (l) => (l.groups[0].repos[0].description = 'a\udc00b')],
		['a description with a reversed surrogate pair', (l) => (l.groups[0].repos[0].description = '\udc00\ud800')],
		['a description ending in a high surrogate', (l) => (l.groups[0].repos[0].description = 'abc\ud83d')],
		['fetched_at with a numeric offset', (l) => (l.fetched_at = '2026-11-13T12:00:00.000+00:00')],
		['fetched_at with a lowercase t', (l) => (l.fetched_at = '2026-11-13t12:00:00.000Z')],
		['fetched_at with a lowercase z', (l) => (l.fetched_at = '2026-11-13T12:00:00.000z')],
		['fetched_at with no fraction', (l) => (l.fetched_at = '2026-11-13T12:00:00Z')],
		['fetched_at with a one-digit fraction', (l) => (l.fetched_at = '2026-11-13T12:00:00.0Z')],
		['fetched_at with a four-digit fraction', (l) => (l.fetched_at = '2026-11-13T12:00:00.0000Z')],
		['fetched_at that is not a calendar instant (Feb 30)', (l) => (l.fetched_at = '2026-02-30T12:00:00.000Z')],
		['a case-only twin within an owner', (l) => l.groups[0].repos.splice(1, 0, { name: 'fixture-A-B', description: '' })],
		['an out-of-order pair within an owner', (l) => l.groups[0].repos.reverse()],
		['a description with a tag character (invisible table)', (l) => (l.groups[0].repos[0].description = 'a\u{E0041}b')],
		['an unknown key at the top level', (l) => Object.assign(l, { extra: 1 })],
		['an unknown key on a group', (l) => Object.assign(l.groups[0], { extra: 1 })],
		['an unknown key on a repo', (l) => Object.assign(l.groups[0].repos[0], { url: 'https://github.com/mnemra/x' })],
	];
	for (const [name, mutate] of rejects) {
		it(`rejects ${name}`, async () => {
			const list = valid();
			mutate(list);
			serveList(list);
			const { container } = render(ProjectsPage);
			await expectErrorState(container);
		});
	}

	const accepts: [string, (l: ListFile) => void][] = [
		['a name of exactly 100 characters', (l) => (l.groups[0].repos[0].name = 'a'.repeat(100))],
		['a description of exactly 350 code points', (l) => (l.groups[0].repos[0].description = 'a'.repeat(350))],
		[
			'a description of 350 astral code points (700 UTF-16 units)',
			(l) => (l.groups[0].repos[0].description = '\u{1F600}'.repeat(350)),
		],
		// Last of mnemra: lowercased 'fixture.z_y-z' sorts after 'fixture-mixed-b' ('.' > '-'), so order holds.
		['a name with dot, underscore and hyphen', (l) => (l.groups[0].repos[4].name = 'fixture.z_y-Z')],
	];
	for (const [name, mutate] of accepts) {
		it(`accepts ${name}`, async () => {
			const list = valid();
			mutate(list);
			serveList(list);
			const { container } = render(ProjectsPage);
			await waitFor(() => expect(container.querySelectorAll('h2').length).toBe(2));
			expect(alertLine(container)).toBeNull();
		});
	}
});
