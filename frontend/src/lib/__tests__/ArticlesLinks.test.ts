/**
 * R-26 (docs/specs/2026-11-18-live-lists-and-blog-move.md): the Articles link at all four sites has
 * `href` exactly https://mnemra.dev/blog/, `target="_blank"`, `rel="noopener noreferrer"`, and no
 * extra accessible "opens another site" cue (V-Consistency over V-Accessibility, ruled 2026-11-19).
 * One test per site. Only `#lib/api.ts` (the network boundary) is substituted; components render for real.
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, within } from '@testing-library/svelte';

vi.mock('#lib/api.ts', () => ({
	api: {
		getProfile: vi.fn().mockRejectedValue(new Error('hub failed')),
		getLinks: vi.fn().mockRejectedValue(new Error('hub failed')),
		getSkills: vi.fn().mockRejectedValue(new Error('hub failed')),
	},
}));

import Hero from '#lib/components/Hero.svelte';
import HeroSkeleton from '#lib/components/HeroSkeleton.svelte';
import HubPage from '../../routes/(public)/+page.svelte';

const BLOG = 'https://mnemra.dev/blog/';

function expectBlogLink(link: HTMLElement) {
	expect(link.getAttribute('href')).toBe(BLOG);
	expect(link.getAttribute('target')).toBe('_blank');
	expect(link.getAttribute('rel')).toBe('noopener noreferrer');
}

/** No extra accessible cue: the computed accessible name is exactly the visible label text and there is
 *  no accessible description (jest-dom, covers aria-label, role=img children, aria-description).
 *  The attribute/wording list below is a backstop. */
function expectNoCue(link: HTMLElement, visibleName: string) {
	expect(link).toHaveAccessibleName(visibleName);
	expect(link).not.toHaveAccessibleDescription();
	for (const attr of ['aria-label', 'aria-labelledby', 'aria-describedby', 'title']) {
		expect(link.hasAttribute(attr), attr).toBe(false);
	}
	expect(link.querySelector('.sr-only, .visually-hidden, [aria-hidden], svg, img')).toBeNull();
	expect(link.textContent ?? '').not.toMatch(/new tab|new window|external|opens|another site/i);
}

const mockProfile = {
	id: 1, name: 'Test Person', title: 'Engineer', location: 'Here', email: 'a@b.c',
	summary: 's', pitch: 'p', initials: 'TP', status: 'open', badge: 'b',
} as never;

describe('R-26 Articles link sites', () => {
	it('Hero', () => {
		render(Hero, { props: { profile: mockProfile, links: [] } });
		const link = screen.getByTestId('card-articles');
		expectBlogLink(link);
		expectNoCue(link, 'Articles Thoughts and write-ups');
	});

	it('HeroSkeleton', () => {
		render(HeroSkeleton);
		const link = screen.getByTestId('skeleton-card-articles');
		expectBlogLink(link);
		expectNoCue(link, 'Articles Thoughts and write-ups');
	});

	it('hub-error fallback', async () => {
		render(HubPage);
		// Wait for the error branch first: until then HeroSkeleton renders its own Articles link.
		await screen.findByText('hub failed');
		const hubError = document.querySelector<HTMLElement>('.hub-error');
		expect(hubError).not.toBeNull();
		const link = within(hubError as HTMLElement).getByRole('link', { name: /articles/i });
		expectBlogLink(link);
		expectNoCue(link, 'Articles');
	});
});
