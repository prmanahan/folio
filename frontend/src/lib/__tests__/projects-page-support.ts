/** Shared assertions for the /projects page tests: R-24's unavailable state (plan T12, #4853). */
import { expect } from 'vitest';

export const UNAVAILABLE_LINE = 'The project list is temporarily unavailable; try again shortly.';

/** Every anchor whose href is on github.com: the two owner links of the unavailable state, or repo links. */
export function githubLinks(c: HTMLElement): HTMLAnchorElement[] {
	return [...c.querySelectorAll<HTMLAnchorElement>('a')].filter((a) =>
		(a.getAttribute('href') ?? '').startsWith('https://github.com/'),
	);
}

export function alertLine(c: HTMLElement): HTMLElement | null {
	return c.querySelector<HTMLElement>('[role="alert"]');
}

/** R-24: the exact line, no entries, no `No projects yet.`, and the two owner links. */
export function expectUnavailable(c: HTMLElement) {
	const el = alertLine(c);
	expect(el).not.toBeNull();
	expect(el?.textContent?.trim()).toBe(UNAVAILABLE_LINE);
	expect(c.querySelectorAll('h2').length).toBe(0);
	expect(c.textContent).not.toContain('No projects yet.');
	const links = githubLinks(c);
	expect(links.map((a) => a.getAttribute('href'))).toEqual([
		'https://github.com/mnemra',
		'https://github.com/prmanahan',
	]);
	const texts = links.map((a) => (a.textContent ?? '').trim());
	expect(texts[0]).toContain('mnemra');
	expect(texts[1]).toContain('prmanahan');
	expect(new Set(texts).size).toBe(2);
	for (const a of links) {
		expect(a.getAttribute('target')).toBe('_blank');
		expect(a.getAttribute('rel')).toBe('noopener noreferrer');
	}
}
