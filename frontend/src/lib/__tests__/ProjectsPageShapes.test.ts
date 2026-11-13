/**
 * R-13 (docs/specs/2026-11-11-projects-from-github.md): wrong-shape list files on /projects show the
 * error state and no entries. Added in the #4726 T8 review fix, beside (not inside) ProjectsPage.test.ts.
 *
 * Reads, and why:
 *  - frontend/scripts/projects-fixtures/list-expected.json: a valid list file, mutated one field at a
 *    time so each case differs from valid in exactly one way;
 *  - the page's error element, located as `.error` or `[role=alert]` (same as ProjectsPage.test.ts);
 *  - spec R-13 and the Data Model (fetched_at, owner and name strings, description a string).
 * Only global `fetch` is substituted (the network boundary); the page is rendered for real.
 *
 * The duplicate-name case is new behaviour: a duplicate repo name within an owner is not a valid list,
 * so the page shows the error state instead of throwing out of Svelte's keyed each.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, waitFor } from '@testing-library/svelte';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import ProjectsPage from '../../routes/(public)/projects/+page.svelte';

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
			if (url === '/projects.json') {
				return new Response(JSON.stringify(list), { status: 200, headers: { 'content-type': 'application/json' } });
			}
			return new Response('nope', { status: 500 });
		}),
	);
}

function errorElement(container: HTMLElement): HTMLElement | null {
	return container.querySelector<HTMLElement>('.error, [role="alert"]');
}

function repoAnchors(container: HTMLElement): HTMLAnchorElement[] {
	return [...container.querySelectorAll<HTMLAnchorElement>('a')].filter((a) =>
		(a.getAttribute('href') ?? '').startsWith('https://github.com/'),
	);
}

async function expectErrorState(container: HTMLElement) {
	await waitFor(() => expect(errorElement(container)).not.toBeNull());
	expect((errorElement(container)?.textContent ?? '').trim()).not.toBe('');
	expect(repoAnchors(container).length).toBe(0);
	expect(container.querySelectorAll('h2').length).toBe(0);
}

beforeEach(() => {
	vi.unstubAllGlobals();
});
afterEach(() => {
	vi.unstubAllGlobals();
});

describe('R-13: a wrong-shape list file shows the error state and no entries', () => {
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
