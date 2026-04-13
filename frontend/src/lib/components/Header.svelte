<script lang="ts">
	import { page } from '$app/state';

	interface Props {
		/** Monogram initials derived from the profile name (e.g. "PM", "AR").
		 *  Undefined while profile is loading — shows skeleton placeholder.
		 *  Null on error — shows "?" fallback. */
		initials?: string | null;
	}

	let { initials = undefined }: Props = $props();

	// Derive breadcrumb segments from the current URL path
	let segments = $derived(
		page.url.pathname === '/'
			? []
			: page.url.pathname
					.split('/')
					.filter(Boolean)
					.map((seg, i, arr) => {
						const path = '/' + arr.slice(0, i + 1).join('/');
						const label = seg
							.split('-')
							.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
							.join(' ');
						return { path, label, isLast: i === arr.length - 1 };
					})
	);

	let isHome = $derived(page.url.pathname === '/');

	// For mobile collapse: show only parent as back link when depth > 1
	let parentSegment = $derived(
		segments.length > 1 ? segments[segments.length - 2] : null
	);

	// Resolved display text: skeleton (loading), fallback (error), or initials
	let monogramText = $derived(
		initials === undefined ? '' : (initials === null ? '?' : initials)
	);
	let isLoadingMonogram = $derived(initials === undefined);
</script>

<header class="site-header">
	<div class="header-inner container">
		<a href="/" class="monogram" class:skeleton={isLoadingMonogram} aria-label="Home">
			{#if !isLoadingMonogram}{monogramText}{/if}
		</a>

		{#if !isHome && segments.length > 0}
			<nav aria-label="Breadcrumb" class="breadcrumb">
				<!-- Desktop breadcrumb: full trail -->
				<ol class="breadcrumb-desktop">
					{#each segments as segment}
						{#if !segment.isLast}
							<li>
								<a href={segment.path}>{segment.label}</a>
								<span class="separator" aria-hidden="true">/</span>
							</li>
						{:else}
							<li>
								<span aria-current="page">{segment.label}</span>
							</li>
						{/if}
					{/each}
				</ol>

				<!-- Mobile breadcrumb: back link to parent (depth > 1) or current page name -->
				<div class="breadcrumb-mobile">
					{#if parentSegment}
						<a href={parentSegment.path} class="back-link">
							<span aria-hidden="true">&larr;</span> {parentSegment.label}
						</a>
					{:else}
						<span aria-current="page">{segments[0].label}</span>
					{/if}
				</div>
			</nav>
		{/if}
	</div>
</header>

<style>
	.site-header {
		position: sticky;
		top: 0;
		z-index: 100;
		background: var(--color-bg);
		border-bottom: 1px solid var(--color-border);
		height: var(--nav-height);
	}

	.header-inner {
		display: flex;
		align-items: center;
		gap: 1rem;
		height: 100%;
	}

	.monogram {
		font-family: var(--font-heading);
		font-weight: 900;
		font-size: 1.25rem;
		letter-spacing: 0.08em;
		color: var(--color-gold);
		text-decoration: none;
		flex-shrink: 0;
		min-width: 2ch;
	}

	.monogram:hover {
		color: var(--color-gold);
		text-decoration: none;
	}

	/* Loading state: skeleton placeholder matching monogram dimensions */
	.monogram.skeleton {
		display: inline-block;
		width: 2.2rem;
		height: 1.2rem;
		background: rgba(176, 141, 87, 0.20);
		border-radius: var(--radius-sm);
		animation: skeleton-pulse 1.4s ease-in-out infinite;
		vertical-align: middle;
	}

	@keyframes skeleton-pulse {
		0%, 100% { opacity: 0.5; }
		50%       { opacity: 1; }
	}

	.breadcrumb {
		min-width: 0;
	}

	.breadcrumb-desktop {
		display: flex;
		align-items: center;
		list-style: none;
		gap: 0;
		margin: 0;
		padding: 0;
	}

	.breadcrumb-desktop li {
		display: flex;
		align-items: center;
	}

	.breadcrumb-desktop a {
		font-family: var(--font-heading);
		font-size: 0.75rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.10em;
		color: var(--color-text-muted);
		text-decoration: none;
		padding: 0.25rem 0;
	}

	.breadcrumb-desktop a:hover {
		color: var(--color-accent-light);
	}

	.breadcrumb-desktop span[aria-current="page"] {
		font-family: var(--font-heading);
		font-size: 0.75rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.10em;
		color: var(--color-accent-light);
	}

	.separator {
		margin: 0 0.5rem;
		color: var(--color-text-ghost);
		font-size: 0.75rem;
	}

	/* Mobile: hide desktop breadcrumb, show mobile version */
	.breadcrumb-mobile {
		display: none;
	}

	.back-link {
		font-family: var(--font-heading);
		font-size: 0.75rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.10em;
		color: var(--color-text-muted);
		text-decoration: none;
	}

	.back-link:hover {
		color: var(--color-accent-light);
	}

	.breadcrumb-mobile span[aria-current="page"] {
		font-family: var(--font-heading);
		font-size: 0.75rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.10em;
		color: var(--color-accent-light);
	}

	@media (max-width: 767px) {
		.breadcrumb-desktop {
			display: none;
		}

		.breadcrumb-mobile {
			display: block;
		}
	}
</style>
