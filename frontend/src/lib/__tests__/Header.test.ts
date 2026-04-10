import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, within } from '@testing-library/svelte';
import { setMockPathname } from '$app/state';
import Header from '$lib/components/Header.svelte';

describe('Header', () => {
	afterEach(() => {
		setMockPathname('/');
	});

	describe('PM monogram', () => {
		it('renders PM monogram linking to home', () => {
			setMockPathname('/projects');
			render(Header);
			const monogram = screen.getByRole('link', { name: 'Home' });
			expect(monogram).toBeInTheDocument();
			expect(monogram).toHaveAttribute('href', '/');
			expect(monogram).toHaveTextContent('PM');
		});

		it('has aria-label="Home" on monogram', () => {
			setMockPathname('/projects');
			render(Header);
			const monogram = screen.getByRole('link', { name: 'Home' });
			expect(monogram).toHaveAttribute('aria-label', 'Home');
		});
	});

	describe('breadcrumb on inner pages', () => {
		it('renders correct segments for /projects', () => {
			setMockPathname('/projects');
			render(Header);
			const nav = screen.getByRole('navigation', { name: 'Breadcrumb' });
			expect(nav).toBeInTheDocument();
			// Desktop breadcrumb should show "Projects" as current page
			const desktopList = nav.querySelector('.breadcrumb-desktop') as HTMLElement;
			expect(within(desktopList).getByText('Projects')).toBeInTheDocument();
		});

		it('renders correct segments for /projects/redline', () => {
			setMockPathname('/projects/redline');
			render(Header);
			const nav = screen.getByRole('navigation', { name: 'Breadcrumb' });
			const desktopList = nav.querySelector('.breadcrumb-desktop') as HTMLElement;
			// "Projects" should be a link in desktop breadcrumb
			const projectsLink = within(desktopList).getByRole('link', { name: 'Projects' });
			expect(projectsLink).toHaveAttribute('href', '/projects');
			// "Redline" should be the current page text
			expect(within(desktopList).getByText('Redline')).toBeInTheDocument();
		});

		it('renders correct segments for /articles/my-post', () => {
			setMockPathname('/articles/my-post');
			render(Header);
			const nav = screen.getByRole('navigation', { name: 'Breadcrumb' });
			const desktopList = nav.querySelector('.breadcrumb-desktop') as HTMLElement;
			const articlesLink = within(desktopList).getByRole('link', { name: 'Articles' });
			expect(articlesLink).toHaveAttribute('href', '/articles');
			expect(within(desktopList).getByText('My Post')).toBeInTheDocument();
		});

		it('last segment has aria-current="page"', () => {
			setMockPathname('/projects');
			render(Header);
			const nav = screen.getByRole('navigation', { name: 'Breadcrumb' });
			const desktopList = nav.querySelector('.breadcrumb-desktop') as HTMLElement;
			const current = within(desktopList).getByText('Projects');
			expect(current).toHaveAttribute('aria-current', 'page');
		});

		it('separators have aria-hidden="true"', () => {
			setMockPathname('/projects/redline');
			render(Header);
			const nav = screen.getByRole('navigation', { name: 'Breadcrumb' });
			const desktopList = nav.querySelector('.breadcrumb-desktop') as HTMLElement;
			const separators = within(desktopList).getAllByText('/');
			for (const sep of separators) {
				expect(sep).toHaveAttribute('aria-hidden', 'true');
			}
		});

		it('each segment except last is a link to its route', () => {
			setMockPathname('/projects/redline');
			render(Header);
			const nav = screen.getByRole('navigation', { name: 'Breadcrumb' });
			const desktopList = nav.querySelector('.breadcrumb-desktop') as HTMLElement;
			const projectsLink = within(desktopList).getByRole('link', { name: 'Projects' });
			expect(projectsLink).toHaveAttribute('href', '/projects');
			// "Redline" is the last segment — should NOT be a link
			const redline = within(desktopList).getByText('Redline');
			expect(redline.tagName).not.toBe('A');
		});
	});

	describe('mobile breadcrumb collapse', () => {
		it('shows back link to parent for depth > 1', () => {
			setMockPathname('/projects/redline');
			render(Header);
			const nav = screen.getByRole('navigation', { name: 'Breadcrumb' });
			const mobileDiv = nav.querySelector('.breadcrumb-mobile') as HTMLElement;
			const backLink = within(mobileDiv).getByRole('link');
			expect(backLink).toHaveAttribute('href', '/projects');
			expect(backLink).toHaveTextContent('Projects');
		});

		it('shows current page name for depth = 1', () => {
			setMockPathname('/projects');
			render(Header);
			const nav = screen.getByRole('navigation', { name: 'Breadcrumb' });
			const mobileDiv = nav.querySelector('.breadcrumb-mobile') as HTMLElement;
			expect(within(mobileDiv).getByText('Projects')).toBeInTheDocument();
		});
	});

	describe('home page', () => {
		it('renders no breadcrumb text on home page', () => {
			setMockPathname('/');
			render(Header);
			const nav = screen.queryByRole('navigation', { name: 'Breadcrumb' });
			expect(nav).not.toBeInTheDocument();
		});
	});

	describe('breadcrumb nav attributes', () => {
		it('nav element has aria-label="Breadcrumb"', () => {
			setMockPathname('/projects');
			render(Header);
			const nav = screen.getByRole('navigation', { name: 'Breadcrumb' });
			expect(nav).toHaveAttribute('aria-label', 'Breadcrumb');
		});
	});
});
