<footer>
	<div class="container">
		<p><span class="gear" aria-hidden="true"></span>&copy; {new Date().getFullYear()} Peter Manahan</p>
	</div>
</footer>

<style>
	footer {
		border-top: 1px solid var(--color-border);
		padding: 2rem 0;
		margin-top: 4rem;
		text-align: center;
	}

	p {
		font-family: var(--font-heading);
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.12em;
		color: var(--color-text-ghost);
		display: inline-flex;
		align-items: center;
	}

	.gear {
		display: inline-block;
		width: 12px;
		height: 12px;
		border-radius: 50%;
		background: conic-gradient(
			var(--raw-brass) 0deg 30deg, transparent 30deg 60deg,
			var(--raw-brass) 60deg 90deg, transparent 90deg 120deg,
			var(--raw-brass) 120deg 150deg, transparent 150deg 180deg,
			var(--raw-brass) 180deg 210deg, transparent 210deg 240deg,
			var(--raw-brass) 240deg 270deg, transparent 270deg 300deg,
			var(--raw-brass) 300deg 330deg, transparent 330deg 360deg
		);
		opacity: 0.4;
		margin-right: 0.5rem;
	}
</style>
