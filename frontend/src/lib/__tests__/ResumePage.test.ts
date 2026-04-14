import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';

// Mock the api module before imports that reference it
vi.mock('$lib/api', () => ({
	api: {
		getProfile: vi.fn(),
		getSkills: vi.fn(),
		getExperience: vi.fn(),
		getEducation: vi.fn(),
	},
}));

import ResumePage from '../../routes/(public)/resume/+page.svelte';
import { api } from '$lib/api';
import type { Profile, Skill, Experience, Education } from '$lib/types';

const mockProfile: Profile = {
	name: 'Alex Rivera',
	email: 'alex@example.com',
	title: 'Software Architect',
	location: 'Vancouver, BC',
	phone: '',
	linkedin_url: '',
	github_url: '',
	twitter_url: '',
	pitch_short: 'Short pitch.',
	pitch_long: 'Long-form bio with substantial detail about the candidate.',
	availability_status: 'open',
	availability_date: '',
	remote_preference: 'remote',
};

const mockSkills: Skill[] = [
	{ id: 1, skill_name: 'Rust', category: 'strong', years_experience: 3, last_used: '2026' },
	{ id: 2, skill_name: 'Python', category: 'moderate', years_experience: 5, last_used: '2025' },
	{ id: 3, skill_name: 'Elixir', category: 'gap', years_experience: 1, last_used: '2023' },
];

const mockExperiences: Experience[] = [
	{
		id: 1,
		company_name: 'Acme Corp',
		title: 'Senior Engineer',
		location: 'Remote',
		start_date: '2020-01',
		end_date: null,
		is_current: true,
		summary: 'Built systems.',
		bullet_points: ['Led architecture redesign'],
		display_order: 1,
	},
];

const mockEducation: Education[] = [
	{
		id: 1,
		degree: 'B.S. Computer Science',
		institution: 'State University',
		location: 'Anytown, USA',
		start_year: '2008',
		end_year: '2012',
	},
];

beforeEach(() => {
	vi.mocked(api.getProfile).mockResolvedValue(mockProfile);
	vi.mocked(api.getSkills).mockResolvedValue(mockSkills);
	vi.mocked(api.getExperience).mockResolvedValue(mockExperiences);
	vi.mocked(api.getEducation).mockResolvedValue(mockEducation);
});

describe('Resume page', () => {
	it('renders page heading "Resume"', async () => {
		render(ResumePage);
		await waitFor(() => {
			expect(screen.getByRole('heading', { level: 1, name: /^resume$/i })).toBeInTheDocument();
		});
	});

	it('shows loading state before data resolves', () => {
		// Delay resolution so we can catch the loading state
		vi.mocked(api.getProfile).mockReturnValue(new Promise(() => {}));
		vi.mocked(api.getSkills).mockReturnValue(new Promise(() => {}));
		vi.mocked(api.getExperience).mockReturnValue(new Promise(() => {}));
		vi.mocked(api.getEducation).mockReturnValue(new Promise(() => {}));
		render(ResumePage);
		expect(screen.getByRole('status')).toHaveAttribute('aria-busy', 'true');
	});

	it('renders Skills section after data loads', async () => {
		render(ResumePage);
		await waitFor(() => {
			expect(screen.getByTestId('resume-section-skills')).toBeInTheDocument();
		});
	});

	it('renders Experience section after data loads', async () => {
		render(ResumePage);
		await waitFor(() => {
			expect(screen.getByTestId('resume-section-experience')).toBeInTheDocument();
		});
	});

	it('renders Education section after data loads', async () => {
		render(ResumePage);
		await waitFor(() => {
			expect(screen.getByTestId('resume-section-education')).toBeInTheDocument();
		});
	});

	it('renders sections in order: Bio → Skills → Experience → Education', async () => {
		const { container } = render(ResumePage);
		await waitFor(() => {
			expect(screen.getByTestId('resume-section-skills')).toBeInTheDocument();
		});

		const anchors = container.querySelectorAll('.section-anchor');
		const ids = Array.from(anchors).map((el) => el.id);
		expect(ids).toEqual(['bio', 'skills', 'experience', 'education']);
	});

	it('has id="skills" anchor for deep linking', async () => {
		const { container } = render(ResumePage);
		await waitFor(() => {
			expect(container.querySelector('#skills')).toBeInTheDocument();
		});
	});

	it('has id="experience" anchor for deep linking', async () => {
		const { container } = render(ResumePage);
		await waitFor(() => {
			expect(container.querySelector('#experience')).toBeInTheDocument();
		});
	});

	it('has id="education" anchor for deep linking', async () => {
		const { container } = render(ResumePage);
		await waitFor(() => {
			expect(container.querySelector('#education')).toBeInTheDocument();
		});
	});

	it('renders SkillsPillMatrix with skill data', async () => {
		render(ResumePage);
		await waitFor(() => {
			expect(screen.getByTestId('skills-pill-matrix')).toBeInTheDocument();
		});
		expect(screen.getByText('Rust')).toBeInTheDocument();
	});

	it('renders experience data', async () => {
		render(ResumePage);
		await waitFor(() => {
			expect(screen.getByText('Senior Engineer')).toBeInTheDocument();
		});
	});

	it('renders education data', async () => {
		render(ResumePage);
		await waitFor(() => {
			expect(screen.getByText('B.S. Computer Science')).toBeInTheDocument();
		});
	});

	it('shows error state when API fails', async () => {
		vi.mocked(api.getProfile).mockRejectedValue(new Error('Network error'));
		vi.mocked(api.getSkills).mockRejectedValue(new Error('Network error'));
		vi.mocked(api.getExperience).mockRejectedValue(new Error('Network error'));
		vi.mocked(api.getEducation).mockRejectedValue(new Error('Network error'));
		render(ResumePage);
		await waitFor(() => {
			expect(screen.getByText(/network error/i)).toBeInTheDocument();
		});
	});

	// ----- Bio section (pitch_long) -----
	it('renders Bio section with pitch_long when non-empty', async () => {
		render(ResumePage);
		await waitFor(() => {
			expect(screen.getByTestId('resume-section-bio')).toBeInTheDocument();
		});
		expect(screen.getByText(/Long-form bio with substantial detail/i)).toBeInTheDocument();
		expect(screen.getByRole('heading', { level: 2, name: /^bio$/i })).toBeInTheDocument();
	});

	it('does NOT render Bio section when pitch_long is empty', async () => {
		vi.mocked(api.getProfile).mockResolvedValue({ ...mockProfile, pitch_long: '' });
		render(ResumePage);
		await waitFor(() => {
			expect(screen.getByTestId('resume-section-skills')).toBeInTheDocument();
		});
		expect(screen.queryByTestId('resume-section-bio')).not.toBeInTheDocument();
	});

	it('does NOT render Bio section when pitch_long is whitespace-only', async () => {
		vi.mocked(api.getProfile).mockResolvedValue({ ...mockProfile, pitch_long: '   \n  ' });
		render(ResumePage);
		await waitFor(() => {
			expect(screen.getByTestId('resume-section-skills')).toBeInTheDocument();
		});
		expect(screen.queryByTestId('resume-section-bio')).not.toBeInTheDocument();
	});

	it('Skills section heading is reachable as h2', async () => {
		render(ResumePage);
		await waitFor(() => {
			expect(screen.getByRole('heading', { level: 2, name: /^skills$/i })).toBeInTheDocument();
		});
	});

	it('no aria-hidden on Resume destination content sections', async () => {
		const { container } = render(ResumePage);
		await waitFor(() => {
			expect(screen.getByTestId('resume-section-skills')).toBeInTheDocument();
		});
		// The main content-bearing section anchors must not be aria-hidden.
		// Decorative descendants (e.g., gear icons) may carry aria-hidden; that's fine.
		expect(container.querySelector('[data-testid="resume-section-skills"]')?.getAttribute('aria-hidden')).toBeNull();
		expect(container.querySelector('[data-testid="resume-section-experience"]')?.getAttribute('aria-hidden')).toBeNull();
		expect(container.querySelector('[data-testid="resume-section-education"]')?.getAttribute('aria-hidden')).toBeNull();
	});
});
