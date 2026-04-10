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
		// Nav cards should render even during loading (they are static, not data-dependent)
		expect(screen.getByRole('link', { name: /projects/i })).toBeInTheDocument();
		expect(screen.getByRole('link', { name: /articles/i })).toBeInTheDocument();
		expect(screen.getByText('Contact')).toBeInTheDocument();
	});

	it('has skeleton-pulse class for animation', () => {
		const { container } = render(HeroSkeleton);
		const pulsingElements = container.querySelectorAll('.skeleton-pulse');
		expect(pulsingElements.length).toBeGreaterThan(0);
	});
});
