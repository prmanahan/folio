/**
 * R-27, frontend half (docs/specs/2026-11-18-live-lists-and-blog-move.md): the admin sidebar and the
 * dashboard hold no element whose href starts `/admin/articles`, and no Articles label, count or
 * quick action. Modelled on AdminProjectsRemoval.test.ts. The dashboard is served WITH `articles: 5555`
 * so an implementation that still reads the key would render it.
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
	experiences: 1111, skills: 2222, education: 3333, articles: 5555,
	links: 6666, faq_responses: 7777, gaps_weaknesses: 8888, ai_instructions: 9999,
};

function articleHrefs(container: HTMLElement): string[] {
	return [...container.querySelectorAll('a')]
		.map((a) => a.getAttribute('href') ?? '')
		.filter((h) => h.startsWith('/admin/articles'));
}

beforeEach(() => {
	vi.mocked(getDashboard).mockReset();
	vi.mocked(getDashboard).mockResolvedValue(COUNTS as never);
});

describe('R-27 frontend: the admin layout', () => {
	it('renders no articles link, label or count', async () => {
		setMockPathname('/admin');
		const children = createRawSnippet(() => ({ render: () => '<div>child</div>' }));
		const { container } = render(AdminLayout, { props: { children } });
		await waitFor(() => expect(screen.getAllByText('1111').length).toBeGreaterThan(0));
		expect(screen.getAllByText('6666').length).toBeGreaterThan(0);
		expect(articleHrefs(container)).toEqual([]);
		expect(screen.queryByText('5555')).toBeNull();
		expect(container.textContent ?? '').not.toMatch(/article/i);
	});
});

describe('R-27 frontend: the admin dashboard page', () => {
	it('renders no articles link, no New Article action and no articles count', async () => {
		setMockPathname('/admin');
		const { container } = render(AdminPage);
		await waitFor(() => expect(screen.getAllByText('1111').length).toBeGreaterThan(0));
		expect(screen.getAllByText('9999').length).toBeGreaterThan(0);
		expect(articleHrefs(container)).toEqual([]);
		expect(screen.queryByText('5555')).toBeNull();
		expect(container.textContent ?? '').not.toMatch(/article/i);
	});
});
