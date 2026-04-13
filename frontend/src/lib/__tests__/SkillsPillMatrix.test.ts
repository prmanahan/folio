import { describe, it, expect } from 'vitest';
import { render, screen, within } from '@testing-library/svelte';
import SkillsPillMatrix from '$lib/components/SkillsPillMatrix.svelte';
import type { Skill } from '$lib/types';

const mockSkills: Skill[] = [
	{ id: 1, skill_name: 'Rust', category: 'strong', years_experience: 3, last_used: '2026' },
	{ id: 2, skill_name: 'TypeScript', category: 'strong', years_experience: 8, last_used: '2026' },
	{ id: 3, skill_name: 'Python', category: 'moderate', years_experience: 5, last_used: '2025' },
	{ id: 4, skill_name: 'Go', category: 'moderate', years_experience: 2, last_used: '2024' },
	{ id: 5, skill_name: 'Elixir', category: 'gap', years_experience: 1, last_used: '2023' },
];

describe('SkillsPillMatrix', () => {
	it('renders three proficiency bands', () => {
		const { container } = render(SkillsPillMatrix, { props: { skills: mockSkills } });
		const bands = container.querySelectorAll('.band');
		expect(bands.length).toBe(3);
	});

	it('renders Strong band with correct label', () => {
		render(SkillsPillMatrix, { props: { skills: mockSkills } });
		expect(screen.getByText('Strong')).toBeInTheDocument();
	});

	it('renders Moderate band with correct label', () => {
		render(SkillsPillMatrix, { props: { skills: mockSkills } });
		expect(screen.getByText('Moderate')).toBeInTheDocument();
	});

	it('renders Learning band (gap category) with "Learning" label', () => {
		render(SkillsPillMatrix, { props: { skills: mockSkills } });
		expect(screen.getByText('Learning')).toBeInTheDocument();
	});

	it('renders skill names as pills', () => {
		render(SkillsPillMatrix, { props: { skills: mockSkills } });
		expect(screen.getByText('Rust')).toBeInTheDocument();
		expect(screen.getByText('TypeScript')).toBeInTheDocument();
		expect(screen.getByText('Python')).toBeInTheDocument();
		expect(screen.getByText('Elixir')).toBeInTheDocument();
	});

	it('groups skills into correct bands', () => {
		const { container } = render(SkillsPillMatrix, { props: { skills: mockSkills } });
		const strongBand = container.querySelector('[data-testid="skill-band-strong"]') as HTMLElement;
		expect(within(strongBand).getByText('Rust')).toBeInTheDocument();
		expect(within(strongBand).getByText('TypeScript')).toBeInTheDocument();
		// Python is moderate, should not be in Strong band
		expect(within(strongBand).queryByText('Python')).not.toBeInTheDocument();
	});

	it('puts gap skills in Learning band', () => {
		const { container } = render(SkillsPillMatrix, { props: { skills: mockSkills } });
		const gapBand = container.querySelector('[data-testid="skill-band-gap"]') as HTMLElement;
		expect(within(gapBand).getByText('Elixir')).toBeInTheDocument();
	});

	it('renders pills as role="listitem" inside role="list"', () => {
		render(SkillsPillMatrix, { props: { skills: mockSkills } });
		const lists = screen.getAllByRole('list');
		expect(lists.length).toBeGreaterThanOrEqual(3);
		// Each band has a list
		const listitems = screen.getAllByRole('listitem');
		expect(listitems.length).toBe(mockSkills.length);
	});

	it('does not display counts or percentages', () => {
		const { container } = render(SkillsPillMatrix, { props: { skills: mockSkills } });
		const text = container.textContent ?? '';
		expect(text).not.toMatch(/\d+%/);
		expect(text).not.toMatch(/\d+ skills/);
	});

	it('skips bands with no skills in that category', () => {
		const strongOnly: Skill[] = [
			{ id: 1, skill_name: 'Rust', category: 'strong', years_experience: 3, last_used: '2026' },
		];
		const { container } = render(SkillsPillMatrix, { props: { skills: strongOnly } });
		const bands = container.querySelectorAll('.band');
		expect(bands.length).toBe(1);
		expect(screen.queryByText('Moderate')).not.toBeInTheDocument();
		expect(screen.queryByText('Learning')).not.toBeInTheDocument();
	});

	it('renders empty state gracefully with no skills', () => {
		const { container } = render(SkillsPillMatrix, { props: { skills: [] } });
		const bands = container.querySelectorAll('.band');
		expect(bands.length).toBe(0);
	});
});
