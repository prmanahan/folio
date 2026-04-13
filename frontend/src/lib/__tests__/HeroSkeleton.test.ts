import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import HeroSkeleton from '$lib/components/HeroSkeleton.svelte';

describe('HeroSkeleton', () => {
	it('renders with aria-busy="true"', () => {
		render(HeroSkeleton);
		const container = screen.getByRole('status');
		expect(container).toHaveAttribute('aria-busy', 'true');
	});

	it('renders skeleton placeholder blocks', () => {
		const { container } = render(HeroSkeleton);
		const skeletonBlocks = container.querySelectorAll('.skeleton-block');
		// Should have multiple placeholder blocks (name, title, badge, pitch, etc.)
		expect(skeletonBlocks.length).toBeGreaterThanOrEqual(4);
	});

	it('renders static nav cards during loading', () => {
		render(HeroSkeleton);
		// All 5 cards render even during loading (they are static, not data-dependent)
		expect(screen.getByRole('link', { name: /projects/i })).toBeInTheDocument();
		expect(screen.getByRole('link', { name: /articles/i })).toBeInTheDocument();
		expect(screen.getByRole('link', { name: /resume/i })).toBeInTheDocument();
		expect(screen.getByText('Contact')).toBeInTheDocument();
	});

	it('renders Resume card linking to /resume', () => {
		render(HeroSkeleton);
		const resumeCard = screen.getByRole('link', { name: /resume/i });
		expect(resumeCard).toHaveAttribute('href', '/resume');
	});

	it('renders Contact as full-width zone outside nav row', () => {
		const { container } = render(HeroSkeleton);
		const contactCard = container.querySelector('[data-testid="skeleton-card-contact"]');
		expect(contactCard).toBeInTheDocument();
		const navCards = container.querySelector('.nav-cards');
		// Contact card lives outside nav-cards
		expect(navCards).not.toContain(contactCard);
	});

	it('has skeleton-pulse class for animation', () => {
		const { container } = render(HeroSkeleton);
		const pulsingElements = container.querySelectorAll('.skeleton-pulse');
		expect(pulsingElements.length).toBeGreaterThan(0);
	});
});
