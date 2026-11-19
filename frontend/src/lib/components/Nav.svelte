<script lang="ts">
	import { page } from '$app/state';

	let { onToggleAi }: { onToggleAi: () => void } = $props();

	const links = [
		{ href: '/', label: 'Home', external: false },
		{ href: '/projects', label: 'Projects', external: false },
		{ href: '/agents', label: 'Agents', external: false },
		{ href: 'https://mnemra.dev/blog/', label: 'Articles', external: true },
	];

	function isActive(link: (typeof links)[number]): boolean {
		return link.href === '/' ? page.url.pathname === '/' : page.url.pathname.startsWith(link.href);
	}
</script>

<nav>
	<div class="nav-inner container">
		<a href="/" class="nav-brand">PM</a>
		<div class="nav-links">
			{#each links as link}
				<a
					href={link.href}
					target={link.external ? '_blank' : undefined}
					rel={link.external ? 'noopener noreferrer' : undefined}
					class:active={isActive(link)}
					aria-current={isActive(link) ? 'page' : undefined}
				>
					{link.label}
				</a>
			{/each}
		</div>
		<button class="ask-ai-btn" onclick={onToggleAi}>Ask AI</button>
	</div>
</nav>

<style>
	nav {
		border-bottom: 1px solid rgba(176, 141, 87, 0.40);
		background: var(--color-bg);
		position: sticky;
		top: 0;
		z-index: 100;
	}

	.nav-inner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: var(--nav-height);
	}

	.nav-brand {
		font-family: var(--font-heading);
		font-weight: 900;
		font-size: 1.25rem;
		letter-spacing: 0.08em;
		color: var(--color-gold);
	}

	.nav-brand:hover {
		color: var(--color-gold);
		text-decoration: none;
	}

	.nav-links {
		display: flex;
		gap: 1.5rem;
	}

	.nav-links a {
		font-family: var(--font-heading);
		font-size: 0.75rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		color: var(--color-text-muted);
		padding: 0.375rem 0.25rem;
		border-bottom: 2px solid transparent;
		transition: color 0.2s ease, border-color 0.2s ease;
	}

	.nav-links a:hover {
		color: var(--color-accent-light);
		border-bottom-color: rgba(176, 141, 87, 0.6);
		text-decoration: none;
	}

	.nav-links a.active {
		color: var(--color-accent-light);
		border-bottom-color: var(--color-accent);
		font-weight: 400;
	}

	.ask-ai-btn {
		font-family: var(--font-heading);
		font-size: 0.75rem;
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.10em;
		padding: 0.375rem 0.875rem;
		border: 1px solid rgba(176, 141, 87, 0.40);
		border-radius: var(--radius-btn);
		background: var(--color-surface);
		color: var(--color-text-muted);
		cursor: pointer;
		transition: color 0.2s ease, border-color 0.2s ease, background 0.2s ease;
		min-height: 44px;
		display: flex;
		align-items: center;
	}

	.ask-ai-btn:hover {
		color: var(--color-accent-light);
		border-color: var(--color-accent);
		background: rgba(176, 141, 87, 0.10);
	}

	@media (max-width: 767px) {
		.nav-inner { flex-wrap: wrap; height: auto; padding: 0.5rem 0; gap: 0.5rem; }
		.nav-links {
			display: flex; width: 100%; overflow-x: auto;
			-webkit-overflow-scrolling: touch; gap: 0; padding-bottom: 0.25rem;
			scrollbar-width: none;
		}
		.nav-links::-webkit-scrollbar { display: none; }
		.nav-links a { padding: 0.5rem 0.875rem; white-space: nowrap; flex-shrink: 0; }
	}
</style>
