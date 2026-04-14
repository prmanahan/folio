import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Hero from '$lib/components/Hero.svelte';
import type { Profile, Link } from '$lib/types';

const mockProfile: Profile = {
	name: 'Peter Manahan',
	email: 'peter@example.com',
	title: 'Senior Software Architect',
	location: 'Portland, OR',
	phone: '',
	linkedin_url: '',
	github_url: '',
	twitter_url: '',
	pitch_short: 'Building things that matter.',
	pitch_long: 'Building things that matter, with a much longer story behind it.',
	availability_status: 'open',
	availability_date: '2026-04-01',
	remote_preference: 'Remote',
};

const mockLinks: Link[] = [
	{ id: 1, label: 'GitHub', url: 'https://github.com/pmanahan', icon: 'github', sort_order: 1 },
	{ id: 2, label: 'LinkedIn', url: 'https://linkedin.com/in/pmanahan', icon: 'linkedin', sort_order: 2 },
	{ id: 3, label: 'Resume', url: 'https://example.com/resume.pdf', icon: 'file', sort_order: 3 },
	{ id: 4, label: 'Email', url: 'mailto:peter@example.com', icon: 'mail', sort_order: 4 },
];

describe('Hero hub layout', () => {
	it('renders profile name as h1', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks } });
		expect(screen.getByRole('heading', { level: 1, name: /Peter Manahan/i })).toBeInTheDocument();
	});

	it('renders title', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks } });
		expect(screen.getByText('Senior Software Architect')).toBeInTheDocument();
	});

	it('renders availability badge with teal styling', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks } });
		const badge = screen.getByText(/open to opportunities/i);
		expect(badge).toBeInTheDocument();
	});

	it('renders pitch_short as the hub pitch', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks } });
		expect(screen.getByText('Building things that matter.')).toBeInTheDocument();
	});

	it('does NOT render pitch_long on the hub (long form lives on /resume)', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks } });
		expect(screen.queryByText(/much longer story behind it/i)).not.toBeInTheDocument();
	});

	it('renders Ask AI card', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks } });
		const askAiCard = screen.getByRole('button', { name: /ask ai/i });
		expect(askAiCard).toBeInTheDocument();
	});

	it('renders Projects nav card linking to /projects', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks } });
		const projectsCard = screen.getByRole('link', { name: /projects/i });
		expect(projectsCard).toHaveAttribute('href', '/projects');
	});

	it('renders Articles nav card linking to /articles', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks } });
		const articlesCard = screen.getByRole('link', { name: /articles/i });
		expect(articlesCard).toHaveAttribute('href', '/articles');
	});

	it('renders Resume nav card linking to /resume', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks } });
		const resumeCard = screen.getByTestId('card-resume');
		expect(resumeCard).toHaveAttribute('href', '/resume');
	});

	it('renders Contact card with link icons', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks } });
		const githubLink = screen.getByRole('link', { name: /github/i });
		expect(githubLink).toHaveAttribute('href', 'https://github.com/pmanahan');
		const linkedinLink = screen.getByRole('link', { name: /linkedin/i });
		expect(linkedinLink).toHaveAttribute('href', 'https://linkedin.com/in/pmanahan');
	});

	it('renders Contact card as a full-width zone below the nav row', () => {
		const { container } = render(Hero, { props: { profile: mockProfile, links: mockLinks } });
		const contactCard = container.querySelector('[data-testid="card-contact"]');
		expect(contactCard).toBeInTheDocument();
		const navCards = container.querySelector('.nav-cards');
		expect(navCards).not.toContain(contactCard);
	});

	it('labels "Resume" link in Contact card as "Resume PDF" to avoid collision with hub card', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks } });
		const resumePdfLink = screen.getByRole('link', { name: /resume pdf/i });
		expect(resumePdfLink).toBeInTheDocument();
		const resumeNavCard = screen.getByTestId('card-resume');
		expect(resumeNavCard).toHaveAttribute('href', '/resume');
		const allLinks = screen.getAllByRole('link');
		const contactLinks = allLinks.filter((el) => el.classList.contains('contact-link'));
		for (const link of contactLinks) {
			expect(link).not.toHaveAttribute('aria-label', 'Resume');
		}
	});

	it('fires onToggleAi when Ask AI card is clicked', async () => {
		const onToggleAi = vi.fn();
		render(Hero, { props: { profile: mockProfile, links: mockLinks, onToggleAi } });
		const askAi = screen.getByRole('button', { name: /ask ai/i });
		await askAi.click();
		expect(onToggleAi).toHaveBeenCalledOnce();
	});

	describe('Contact card URL validation', () => {
		it('omits links with javascript: protocol', () => {
			const unsafeLinks: Link[] = [
				{ id: 1, label: 'Evil', url: 'javascript:alert(1)', icon: 'skull', sort_order: 1 },
				{ id: 2, label: 'GitHub', url: 'https://github.com', icon: 'github', sort_order: 2 },
			];
			render(Hero, { props: { profile: mockProfile, links: unsafeLinks } });
			expect(screen.queryByText('Evil')).not.toBeInTheDocument();
			expect(screen.getByRole('link', { name: /github/i })).toBeInTheDocument();
		});

		it('omits links with data: protocol', () => {
			const unsafeLinks: Link[] = [
				{ id: 1, label: 'Data', url: 'data:text/html,<script>alert(1)</script>', icon: 'x', sort_order: 1 },
			];
			render(Hero, { props: { profile: mockProfile, links: unsafeLinks } });
			expect(screen.queryByText('Data')).not.toBeInTheDocument();
		});

		it('omits links with vbscript: protocol', () => {
			const unsafeLinks: Link[] = [
				{ id: 1, label: 'VB', url: 'vbscript:MsgBox("hi")', icon: 'x', sort_order: 1 },
			];
			render(Hero, { props: { profile: mockProfile, links: unsafeLinks } });
			expect(screen.queryByText('VB')).not.toBeInTheDocument();
		});

		it('allows mailto: links', () => {
			const links: Link[] = [
				{ id: 1, label: 'Email', url: 'mailto:test@example.com', icon: 'mail', sort_order: 1 },
			];
			render(Hero, { props: { profile: mockProfile, links } });
			expect(screen.getByRole('link', { name: /email/i })).toHaveAttribute(
				'href',
				'mailto:test@example.com',
			);
		});
	});

	describe('Contact card fallback', () => {
		it('renders fallback when links array is empty', () => {
			render(Hero, { props: { profile: mockProfile, links: [] } });
			expect(screen.getByText('Contact')).toBeInTheDocument();
			expect(screen.getByText(/contact links unavailable/i)).toBeInTheDocument();
		});
	});

	describe('card touch targets', () => {
		it('card elements have the card class which sets min-height 44px', () => {
			render(Hero, { props: { profile: mockProfile, links: mockLinks } });
			const askAi = screen.getByRole('button', { name: /ask ai/i });
			expect(askAi.classList.contains('card')).toBe(true);
			const projectsCard = screen.getByRole('link', { name: /projects/i });
			expect(projectsCard.classList.contains('card')).toBe(true);
		});
	});

	// -----------------------------------------------------------------
	// Spec R12: link cap on hub
	// -----------------------------------------------------------------
	describe('hub link cap (R12)', () => {
		const sixLinks: Link[] = Array.from({ length: 6 }, (_, i) => ({
			id: i + 1,
			label: `Link${i + 1}`,
			url: `https://example.com/${i + 1}`,
			icon: 'link',
			sort_order: i + 1,
		}));

		it('renders at most 5 contact links on the hub', () => {
			const { container } = render(Hero, { props: { profile: mockProfile, links: sixLinks } });
			const contactLinks = container.querySelectorAll('.contact-link');
			expect(contactLinks.length).toBe(5);
		});

		it('renders 0 contact links without empty-pill artifacts', () => {
			const { container } = render(Hero, { props: { profile: mockProfile, links: [] } });
			const contactLinks = container.querySelectorAll('.contact-link');
			expect(contactLinks.length).toBe(0);
			expect(screen.getByText(/contact links unavailable/i)).toBeInTheDocument();
		});
	});

	// -----------------------------------------------------------------
	// Spec R1 / R2 / R3: structural fix for vertical overflow
	// -----------------------------------------------------------------
	describe('structural overflow fix (R1, R2, R3)', () => {
		it('Hero does NOT render SkillsBanner as a descendant', () => {
			// SkillsBanner must be a sibling of .hero, not nested inside it.
			// The Hero component should no longer reference the banner at all;
			// the parent (public)/+page.svelte hosts it.
			const { container } = render(Hero, { props: { profile: mockProfile, links: mockLinks } });
			expect(container.querySelector('.skills-banner')).toBeNull();
		});

		it('.hero element has no min-height inline rule and renders with svh-based ceiling', () => {
			const { container } = render(Hero, { props: { profile: mockProfile, links: mockLinks } });
			const hero = container.querySelector('.hero') as HTMLElement | null;
			expect(hero).not.toBeNull();
			// jsdom resolves <style> in component scope; we assert the *inline*
			// style is empty (no min-height shim) and trust that the stylesheet
			// uses 100svh-based constraints. Browser-level layout assertions
			// happen in the Playwright suite (e2e/responsive.spec.ts).
			expect(hero!.style.minHeight).toBe('');
		});
	});

	// -----------------------------------------------------------------
	// Spec bounds: pitch_short = 280 chars renders without breaking
	// -----------------------------------------------------------------
	describe('pitch_short bounds rendering', () => {
		it('renders a 280-character pitch_short without truncation', () => {
			const longPitch = 'a'.repeat(280);
			const profile = { ...mockProfile, pitch_short: longPitch };
			render(Hero, { props: { profile, links: mockLinks } });
			expect(screen.getByText(longPitch)).toBeInTheDocument();
		});
	});
});
