/**
 * R-15, frontend half (docs/specs/2026-11-11-projects-from-github.md): the admin layout and the
 * admin dashboard page render no projects link and no projects count. Red phase for #4726 T7.
 *
 * Reads, and why:
 *  - src/routes/admin/+layout.svelte and +page.svelte: the consumers R-15 names. Read for the one
 *    module they call (`getDashboard`/`logout` from #lib/admin-api.ts), which is the boundary
 *    substituted here, and for their `onMount` data flow.
 *  - src/lib/admin-types.ts: DashboardCounts, to know which keys exist today.
 * The dashboard response is served WITH `projects: 4242` so an implementation that still reads the
 * key renders it; after R-15 the backend no longer sends it and nothing may show it either way.
 * Every other count is a distinct number, so the test waits on a real render before asserting absence.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, waitFor, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';

vi.mock('#lib/admin-api.ts', () => ({
	getDashboard: vi.fn(),
	logout: vi.fn(),
}));

import AdminLayout from '../../routes/admin/+layout.svelte';
import AdminPage from '../../routes/admin/+page.svelte';
import { getDashboard } from '#lib/admin-api.ts';
import { setMockPathname } from '../../test/mocks/app/state';

const COUNTS: Record<string, number> = {
	experiences: 1111,
	skills: 2222,
	education: 3333,
	projects: 4242,
	articles: 5555,
	links: 6666,
	faq_responses: 7777,
	gaps_weaknesses: 8888,
	ai_instructions: 9999,
};

function norm(s: string | null | undefined): string {
	return (s ?? '').replace(/\s+/g, ' ').trim();
}

function projectLinks(container: HTMLElement): string[] {
	return [...container.querySelectorAll('a')]
		.map((a) => a.getAttribute('href') ?? '')
		.filter((h) => /project/i.test(h));
}

beforeEach(() => {
	vi.mocked(getDashboard).mockReset();
	vi.mocked(getDashboard).mockResolvedValue(COUNTS as never);
});

describe('R-15 frontend: the admin layout', () => {
	it('renders no projects link and no projects count', async () => {
		setMockPathname('/admin');
		const children = createRawSnippet(() => ({ render: () => '<div data-testid="child">child</div>' }));
		const { container } = render(AdminLayout, { props: { children } });

		// the nav is rendered and the counts loaded: other counts show
		await waitFor(() => expect(screen.getAllByText('1111').length).toBeGreaterThan(0));
		expect(screen.getAllByText('6666').length).toBeGreaterThan(0);

		expect(projectLinks(container)).toEqual([]);
		expect(screen.queryByText('4242')).toBeNull();
		expect(norm(container.textContent)).not.toMatch(/project/i);
	});
});

describe('R-15 frontend: the admin dashboard page', () => {
	it('renders no projects link, no New Project action and no projects count', async () => {
		setMockPathname('/admin');
		const { container } = render(AdminPage);

		await waitFor(() => expect(screen.getAllByText('1111').length).toBeGreaterThan(0));
		expect(screen.getAllByText('9999').length).toBeGreaterThan(0);

		expect(projectLinks(container)).toEqual([]);
		expect(screen.queryByText('4242')).toBeNull();
		expect(norm(container.textContent)).not.toMatch(/project/i);
	});
});
