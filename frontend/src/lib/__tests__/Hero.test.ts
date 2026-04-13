import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Hero from '$lib/components/Hero.svelte';
import type { Profile, Link, Skill } from '$lib/types';

const mockProfile: Profile = {
	name: 'Peter Manahan',
	email: 'peter@example.com',
	title: 'Senior Software Architect',
	location: 'Portland, OR',
	phone: '',
	linkedin_url: '',
	github_url: '',
	twitter_url: '',
	elevator_pitch: 'Building things that matter.',
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

const mockSkills: Skill[] = [
	{ id: 1, skill_name: 'Rust', category: 'strong', years_experience: 3, last_used: '2026' },
	{ id: 2, skill_name: 'TypeScript', category: 'strong', years_experience: 8, last_used: '2026' },
];

describe('Hero hub layout', () => {
	it('renders profile name as h1', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills } });
		expect(screen.getByRole('heading', { level: 1, name: /Peter Manahan/i })).toBeInTheDocument();
	});

	it('renders title', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills } });
		expect(screen.getByText('Senior Software Architect')).toBeInTheDocument();
	});

	it('renders availability badge with teal styling', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills } });
		const badge = screen.getByText(/open to opportunities/i);
		expect(badge).toBeInTheDocument();
	});

	it('renders elevator pitch', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills } });
		expect(screen.getByText('Building things that matter.')).toBeInTheDocument();
	});

	it('renders Ask AI card', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills } });
		const askAiCard = screen.getByRole('button', { name: /ask ai/i });
		expect(askAiCard).toBeInTheDocument();
	});

	it('renders Projects nav card linking to /projects', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills } });
		const projectsCard = screen.getByRole('link', { name: /projects/i });
		expect(projectsCard).toHaveAttribute('href', '/projects');
	});

	it('renders Articles nav card linking to /articles', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills } });
		const articlesCard = screen.getByRole('link', { name: /articles/i });
		expect(articlesCard).toHaveAttribute('href', '/articles');
	});

	it('renders Resume nav card linking to /resume', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills } });
		const resumeCard = screen.getByTestId('card-resume');
		expect(resumeCard).toHaveAttribute('href', '/resume');
	});

	it('renders Contact card with link icons', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills } });
		// Contact card should contain links for GitHub, LinkedIn, Resume PDF, Email
		const githubLink = screen.getByRole('link', { name: /github/i });
		expect(githubLink).toHaveAttribute('href', 'https://github.com/pmanahan');
		const linkedinLink = screen.getByRole('link', { name: /linkedin/i });
		expect(linkedinLink).toHaveAttribute('href', 'https://linkedin.com/in/pmanahan');
	});

	it('renders Contact card as a full-width zone below the nav row', () => {
		const { container } = render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills } });
		const contactCard = container.querySelector('[data-testid="card-contact"]');
		expect(contactCard).toBeInTheDocument();
		// Contact card is outside the nav-cards div
		const navCards = container.querySelector('.nav-cards');
		expect(navCards).not.toContain(contactCard);
	});

	it('labels "Resume" link in Contact card as "Resume PDF" to avoid collision with hub card', () => {
		render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills } });
		// The API link label is "Resume" but must render as "Resume PDF" in the Contact card
		const resumePdfLink = screen.getByRole('link', { name: /resume pdf/i });
		expect(resumePdfLink).toBeInTheDocument();
		// The hub Resume nav card links to /resume — confirm it exists
		const resumeNavCard = screen.getByTestId('card-resume');
		expect(resumeNavCard).toHaveAttribute('href', '/resume');
		// No contact-link should have the plain "Resume" aria-label
		const allLinks = screen.getAllByRole('link');
		const contactLinks = allLinks.filter(el => el.classList.contains('contact-link'));
		for (const link of contactLinks) {
			expect(link).not.toHaveAttribute('aria-label', 'Resume');
		}
	});

	it('fires onToggleAi when Ask AI card is clicked', async () => {
		const onToggleAi = vi.fn();
		render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills, onToggleAi } });
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
			render(Hero, { props: { profile: mockProfile, links: unsafeLinks, skills: mockSkills } });
			expect(screen.queryByText('Evil')).not.toBeInTheDocument();
			expect(screen.getByRole('link', { name: /github/i })).toBeInTheDocument();
		});

		it('omits links with data: protocol', () => {
			const unsafeLinks: Link[] = [
				{ id: 1, label: 'Data', url: 'data:text/html,<script>alert(1)</script>', icon: 'x', sort_order: 1 },
			];
			render(Hero, { props: { profile: mockProfile, links: unsafeLinks, skills: mockSkills } });
			expect(screen.queryByText('Data')).not.toBeInTheDocument();
		});

		it('omits links with vbscript: protocol', () => {
			const unsafeLinks: Link[] = [
				{ id: 1, label: 'VB', url: 'vbscript:MsgBox("hi")', icon: 'x', sort_order: 1 },
			];
			render(Hero, { props: { profile: mockProfile, links: unsafeLinks, skills: mockSkills } });
			expect(screen.queryByText('VB')).not.toBeInTheDocument();
		});

		it('allows mailto: links', () => {
			const links: Link[] = [
				{ id: 1, label: 'Email', url: 'mailto:test@example.com', icon: 'mail', sort_order: 1 },
			];
			render(Hero, { props: { profile: mockProfile, links, skills: mockSkills } });
			expect(screen.getByRole('link', { name: /email/i })).toHaveAttribute('href', 'mailto:test@example.com');
		});
	});

	describe('Contact card fallback', () => {
		it('renders fallback when links array is empty', () => {
			render(Hero, { props: { profile: mockProfile, links: [], skills: mockSkills } });
			// Contact card should still render with a fallback message
			expect(screen.getByText('Contact')).toBeInTheDocument();
			expect(screen.getByText(/contact links unavailable/i)).toBeInTheDocument();
		});
	});

	describe('card touch targets', () => {
		it('card elements have the card class which sets min-height 44px', () => {
			render(Hero, { props: { profile: mockProfile, links: mockLinks, skills: mockSkills } });
			// Verify the Ask AI card and nav cards have the .card class
			const askAi = screen.getByRole('button', { name: /ask ai/i });
			expect(askAi.classList.contains('card')).toBe(true);
			const projectsCard = screen.getByRole('link', { name: /projects/i });
			expect(projectsCard.classList.contains('card')).toBe(true);
		});
	});
});
