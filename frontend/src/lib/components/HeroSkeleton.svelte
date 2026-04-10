<!--
  HeroSkeleton — loading placeholder that mirrors the hub layout structure.
  Shows pulsing skeleton blocks for data-dependent content while rendering
  static nav cards immediately (they don't depend on API data).
-->
<section class="hero-skeleton" role="status" aria-busy="true" aria-label="Loading content">
	<div class="hub container">
		<!-- Left column: Identity block skeleton -->
		<div class="identity">
			<div class="skeleton-block skeleton-pulse name-block"></div>
			<div class="skeleton-block skeleton-pulse title-block"></div>
			<div class="skeleton-block skeleton-pulse badge-block"></div>
		</div>

		<!-- Right column: Content + cards skeleton -->
		<div class="content">
			<!-- Pitch skeleton -->
			<div class="pitch-skeleton">
				<div class="skeleton-block skeleton-pulse pitch-line"></div>
				<div class="skeleton-block skeleton-pulse pitch-line short"></div>
			</div>

			<!-- Meta skeleton -->
			<div class="meta-skeleton">
				<div class="skeleton-block skeleton-pulse meta-block"></div>
				<div class="skeleton-block skeleton-pulse meta-block"></div>
			</div>

			<!-- Ask AI card skeleton -->
			<div class="skeleton-block skeleton-pulse ai-card-block"></div>

			<!-- Static nav cards — rendered immediately, not skeletonized -->
			<div class="nav-cards">
				<a href="/projects" class="card card-nav">
					<span class="card-label">Projects</span>
					<span class="card-desc">Things I've built</span>
				</a>

				<a href="/articles" class="card card-nav">
					<span class="card-label">Articles</span>
					<span class="card-desc">Thoughts and write-ups</span>
				</a>

				<div class="card card-nav">
					<span class="card-label">Contact</span>
					<span class="card-desc">Links loading...</span>
				</div>
			</div>
		</div>
	</div>
</section>

<style>
	.hero-skeleton {
		position: relative;
		min-height: 100svh;
		display: flex;
		flex-direction: column;
		justify-content: center;
		padding-top: 2rem;
		padding-bottom: 5rem;
	}

	.hub {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	.identity {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.content {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	/* ============================================================
	   Skeleton blocks — pulse animation, surface color
	   ============================================================ */
	.skeleton-block {
		background: var(--raw-bg-surface);
		border-radius: var(--radius-sm);
	}

	.skeleton-pulse {
		animation: skeleton-pulse 1.5s ease-in-out infinite;
	}

	@keyframes skeleton-pulse {
		0%, 100% { opacity: 0.6; }
		50% { opacity: 1; }
	}

	/* Block dimensions matching the real layout */
	.name-block {
		height: 2.75rem;
		width: 70%;
		max-width: 280px;
	}

	.title-block {
		height: 1.25rem;
		width: 55%;
		max-width: 220px;
	}

	.badge-block {
		height: 1rem;
		width: 40%;
		max-width: 160px;
	}

	.pitch-skeleton {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.pitch-line {
		height: 1rem;
		width: 100%;
	}

	.pitch-line.short {
		width: 65%;
	}

	.meta-skeleton {
		display: flex;
		gap: 0.875rem;
	}

	.meta-block {
		height: 0.875rem;
		width: 80px;
	}

	.ai-card-block {
		height: 80px;
		width: 100%;
		border-radius: var(--radius-md);
	}

	/* ============================================================
	   Static nav cards (not skeletonized)
	   ============================================================ */
	.nav-cards {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.card {
		border-radius: var(--radius-md);
		padding: 1rem 1.25rem;
		min-height: 44px;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		text-decoration: none;
		color: var(--color-text);
	}

	.card-nav {
		border: 1px solid rgba(176, 141, 87, 0.40);
		background: transparent;
		transition: border-color 0.2s ease, background 0.2s ease;
	}

	.card-nav:hover {
		border-color: var(--raw-brass-light);
		background: rgba(176, 141, 87, 0.08);
		text-decoration: none;
	}

	.card-label {
		font-family: var(--font-heading);
		font-size: 0.8125rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--color-accent-light);
	}

	.card-desc {
		font-size: 0.8125rem;
		color: var(--color-text-muted);
	}

	/* ============================================================
	   Reduced motion — disable pulse
	   ============================================================ */
	@media (prefers-reduced-motion: reduce) {
		.skeleton-pulse {
			animation: none;
			opacity: 0.8;
		}
	}

	/* ============================================================
	   Tablet — two-column
	   ============================================================ */
	@media (min-width: 768px) {
		.hub {
			flex-direction: row;
			align-items: flex-start;
			gap: 3rem;
		}

		.identity {
			flex: 0 0 38%;
			max-width: 360px;
		}

		.content {
			flex: 1;
			min-width: 0;
		}

		.nav-cards {
			flex-direction: row;
		}

		.nav-cards .card-nav {
			flex: 1;
		}
	}

	@media (min-width: 1024px) {
		.hero-skeleton {
			padding-top: 4rem;
			padding-bottom: 6rem;
		}

		.hub {
			gap: 4rem;
		}
	}
</style>
