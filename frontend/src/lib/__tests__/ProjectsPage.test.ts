/**
 * Adopted rendering R-13 and R-14 (docs/specs/2026-11-11-projects-from-github.md) of the /projects
 * page, served from `/api/lists/projects`. The failed-load outcomes are R-24's
 * (docs/specs/2026-11-18-live-lists-and-blog-move.md), asserted in ProjectsPageUnavailable.test.ts;
 * R-13's "existing error text" no longer applies.
 *
 * Reads, and why:
 *  - frontend/scripts/projects-fixtures/list-expected.json: the expected list file (spec R-8), used
 *    as the served body and as the oracle for hrefs and order;
 *  - spec R-13 and R-14, the Data Model.
 * Only global `fetch` is substituted (the network boundary); the page is rendered for real.
 *
 * Structure-neutral on purpose: owners are found by `h2`, repos by their github.com anchors in
 * document order, text by the page body with `h1`/`h2` removed. The spec does not pin list markup.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, waitFor } from '@testing-library/svelte';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import ProjectsPage from '../../routes/(public)/projects/+page.svelte';

type Repo = { name: string; description: string };
type Group = { owner: string; repos: Repo[] };
type ListFile = { fetched_at: string; groups: Group[] };

const EXPECTED: ListFile = JSON.parse(
	readFileSync(resolve(import.meta.dirname, '../../../scripts/projects-fixtures/list-expected.json'), 'utf8'),
);

let fetchMock: ReturnType<typeof vi.fn>;

/** Serve `/api/lists/projects` with the given response; any other URL gets a 500, so a page that still
 *  reads the old API shows its own error instead of crashing the test. */
function serve(projectsJson: () => Response) {
	fetchMock = vi.fn(async (input: unknown) => {
		const url = typeof input === 'string' ? input : String((input as { url?: string }).url ?? input);
		if (url === '/api/lists/projects') return projectsJson();
		return new Response('nope', { status: 500 });
	});
	vi.stubGlobal('fetch', fetchMock);
}

function serveList(list: unknown) {
	serve(() => new Response(JSON.stringify(list), { status: 200, headers: { 'content-type': 'application/json' } }));
}

function requestedProjectsList(): boolean {
	return fetchMock.mock.calls.some((c) => String(c[0]) === '/api/lists/projects');
}

function norm(s: string | null | undefined): string {
	return (s ?? '').replace(/\s+/g, ' ').trim();
}

/** Page text with the h1 and every h2 removed. */
function bodyTextWithoutHeadings(container: HTMLElement): string {
	const copy = container.cloneNode(true) as HTMLElement;
	copy.querySelectorAll('h1, h2').forEach((h) => h.remove());
	return norm(copy.textContent);
}

function repoAnchors(container: HTMLElement): HTMLAnchorElement[] {
	return [...container.querySelectorAll<HTMLAnchorElement>('a')].filter((a) =>
		/^https:\/\/github\.com\/[^/]+\/[^/]+$/.test(a.getAttribute('href') ?? ''), // owner links (R-24) have one segment
	);
}

/** A valid two-owner list (R-24 / Data Model: exactly mnemra then prmanahan, each with a repo) whose
 *  first repo is `fixture-solo` with the given description. */
function soloGroups(description: string): Group[] {
	return [
		{ owner: 'mnemra', repos: [{ name: 'fixture-solo', description }] },
		{ owner: 'prmanahan', repos: [{ name: 'fixture-other', description: '' }] },
	];
}

/** Text of the first repo entry, so assertions ignore the second owner's entry. */
function firstItemText(container: HTMLElement): string {
	return norm(container.querySelector('li')?.textContent);
}

function esc(s: string): string {
	return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

beforeEach(() => {
	vi.unstubAllGlobals();
});
afterEach(() => {
	vi.unstubAllGlobals();
});

describe('R-13: /projects renders the list file', () => {
	it('loads /api/lists/projects and renders one h2 per owner, in file order, under the single h1', async () => {
		serveList(EXPECTED);
		const { container } = render(ProjectsPage);
		await waitFor(() => expect(container.querySelectorAll('h2').length).toBeGreaterThan(0));

		expect(requestedProjectsList()).toBe(true);
		expect(container.querySelectorAll('h1').length).toBe(1);
		const h2s = [...container.querySelectorAll('h2')].map((h) => norm(h.textContent));
		expect(h2s.length).toBe(EXPECTED.groups.length);
		EXPECTED.groups.forEach((g, i) => expect(h2s[i]).toContain(g.owner));
		// no heading level other than h1 and the owner h2s
		expect(container.querySelectorAll('h3, h4, h5, h6').length).toBe(0);
	});

	it('renders every repo as an anchor to the exact https://github.com/<owner>/<name>, in file order, under its owner heading', async () => {
		serveList(EXPECTED);
		const { container } = render(ProjectsPage);
		await waitFor(() => expect(repoAnchors(container).length).toBeGreaterThan(0));

		// Document order of owner headings and repo anchors must be h2(owner1), a..., h2(owner2), a...
		const got = [...container.querySelectorAll<HTMLElement>('h2, a')]
			.filter((el) => el.tagName === 'H2' || (el.getAttribute('href') ?? '').startsWith('https://github.com/'))
			.map((el) => (el.tagName === 'H2' ? `H2:${norm(el.textContent)}` : `A:${el.getAttribute('href')}`));

		const want: string[] = [];
		for (const g of EXPECTED.groups) {
			want.push(`H2:${g.owner}`);
			for (const r of g.repos) want.push(`A:https://github.com/${g.owner}/${r.name}`);
		}
		// heading text may carry decoration (the spec does not pin it): owner by containment, anchors by equality
		expect(got.length).toBe(want.length);
		want.forEach((w, i) => {
			if (w.startsWith('H2:')) expect(got[i]).toContain(w.slice(3));
			else expect(got[i]).toBe(w);
		});
	});

	it('each repo anchor shows exactly the repo name and opens with target=_blank rel="noopener noreferrer"', async () => {
		serveList(EXPECTED);
		const { container } = render(ProjectsPage);
		await waitFor(() => expect(repoAnchors(container).length).toBeGreaterThan(0));

		const anchors = repoAnchors(container);
		const names = EXPECTED.groups.flatMap((g) => g.repos.map((r) => r.name));
		expect(anchors.length).toBe(names.length);
		anchors.forEach((a, i) => {
			expect(norm(a.textContent)).toBe(names[i]);
			expect(a.getAttribute('target')).toBe('_blank');
			expect(a.getAttribute('rel')).toBe('noopener noreferrer');
		});
	});

	it('each name is followed by its description, in file order, with no placeholder for an empty one', async () => {
		serveList(EXPECTED);
		const { container } = render(ProjectsPage);
		await waitFor(() => expect(repoAnchors(container).length).toBeGreaterThan(0));

		// name, then its description, entry after entry; only non-word glue is allowed between literals,
		// so a placeholder word ("No description") on an empty entry fails the match.
		const glue = '[^A-Za-z0-9_]*';
		const pattern = EXPECTED.groups
			.flatMap((g) => g.repos)
			.map((r) => esc(r.name) + (r.description ? glue + esc(r.description) : ''))
			.join(glue);
		expect(bodyTextWithoutHeadings(container)).toMatch(new RegExp(`^${glue}${pattern}${glue}$`));
	});

	it('an empty description shows the name alone', async () => {
		serveList({
			fetched_at: EXPECTED.fetched_at,
			groups: soloGroups(''),
		});
		const { container } = render(ProjectsPage);
		await waitFor(() => expect(repoAnchors(container).length).toBe(2));

		expect(firstItemText(container)).toBe('fixture-solo');
	});

	it('a described repo shows the name, then the description, and nothing else', async () => {
		serveList({
			fetched_at: EXPECTED.fetched_at,
			groups: soloGroups('Alpha beta'),
		});
		const { container } = render(ProjectsPage);
		await waitFor(() => expect(repoAnchors(container).length).toBe(2));

		expect(firstItemText(container)).toMatch(/^fixture-solo[^A-Za-z0-9_]*Alpha beta$/);
	});
});

describe('R-14: names and descriptions render as text only', () => {
	const HOSTILE = '<img src=x onerror=alert(1)> **b**';

	it('the description <img src=x onerror=alert(1)> **b** creates no img or strong element and its text equals the input', async () => {
		serveList({
			fetched_at: EXPECTED.fetched_at,
			groups: soloGroups(HOSTILE),
		});
		const { container } = render(ProjectsPage);
		await waitFor(() => expect(repoAnchors(container).length).toBe(2));

		expect(container.querySelector('img')).toBeNull();
		expect(container.querySelector('strong')).toBeNull();
		expect(container.querySelector('[onerror]')).toBeNull();
		expect(firstItemText(container)).toMatch(new RegExp(`^fixture-solo[^A-Za-z0-9_]*${esc(HOSTILE)}$`));
	});
});
