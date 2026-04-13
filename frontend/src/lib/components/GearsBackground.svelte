<script lang="ts">
	import { onMount } from 'svelte';

	// Gear stop at 10s per Peter's decisions (spec body says 17s but decisions override)
	const STOP_DELAY_MS = 10_000;

	let stopped = $state(false);
	let visible = $state(false);

	onMount(() => {
		// Fade in gear layer at 100ms
		const fadeTimer = setTimeout(() => {
			visible = true;
		}, 100);

		// Stop gears at 10s
		const stopTimer = setTimeout(() => {
			stopped = true;
		}, STOP_DELAY_MS);

		return () => {
			clearTimeout(fadeTimer);
			clearTimeout(stopTimer);
		};
	});
</script>

<!--
  GearsBackground — inline SVG gear assembly for hero background
  aria-hidden: decorative, screen readers skip this entirely
  role="presentation": belt-and-suspenders for older ATs
-->
<div
	class="gears-bg"
	class:stopped
	class:visible
	aria-hidden="true"
	role="presentation"
>
	<svg
		class="gears-svg"
		viewBox="0 0 700 560"
		xmlns="http://www.w3.org/2000/svg"
		preserveAspectRatio="xMaxYMin meet"
	>
		<defs>
			<!--
				Gear Large: r=80, 12 teeth, tooth_height=22, tooth_width=16
				Each tooth rect centered at (0, -(r+tooth_height/2)) rotated by (i * 360/12)deg
			-->
			<symbol id="gear-lg" viewBox="-96 -96 192 192">
				<!-- Central disc -->
				<circle r="72" fill="var(--raw-brass-dark)" fill-opacity="0.6" />
				<!-- Teeth — 12 teeth at 30deg intervals -->
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(0)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(30)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(60)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(90)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(120)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(150)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(180)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(210)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(240)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(270)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(300)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(330)" />
				<!-- Tooth highlights -->
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass)" fill-opacity="0.3" transform="rotate(0)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass)" fill-opacity="0.3" transform="rotate(60)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass)" fill-opacity="0.3" transform="rotate(120)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass)" fill-opacity="0.3" transform="rotate(180)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass)" fill-opacity="0.3" transform="rotate(240)" />
				<rect x="-8" y="-95" width="16" height="24" rx="2" fill="var(--raw-brass)" fill-opacity="0.3" transform="rotate(300)" />
				<!-- Axle hole -->
				<circle r="14" fill="var(--raw-bg-deeper)" />
			</symbol>

			<!--
				Gear Medium: r=50, 8 teeth, tooth_height=18, tooth_width=13
				Meshes with large gear: center distance = 80+50 = 130px
			-->
			<symbol id="gear-md" viewBox="-64 -64 128 128">
				<!-- Central disc -->
				<circle r="46" fill="var(--raw-brass-dark)" fill-opacity="0.6" />
				<!-- Teeth — 8 teeth at 45deg intervals -->
				<rect x="-6.5" y="-61" width="13" height="20" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(0)" />
				<rect x="-6.5" y="-61" width="13" height="20" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(45)" />
				<rect x="-6.5" y="-61" width="13" height="20" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(90)" />
				<rect x="-6.5" y="-61" width="13" height="20" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(135)" />
				<rect x="-6.5" y="-61" width="13" height="20" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(180)" />
				<rect x="-6.5" y="-61" width="13" height="20" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(225)" />
				<rect x="-6.5" y="-61" width="13" height="20" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(270)" />
				<rect x="-6.5" y="-61" width="13" height="20" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(315)" />
				<!-- Tooth highlights -->
				<rect x="-6.5" y="-61" width="13" height="20" rx="2" fill="var(--raw-brass)" fill-opacity="0.3" transform="rotate(22.5)" />
				<rect x="-6.5" y="-61" width="13" height="20" rx="2" fill="var(--raw-brass)" fill-opacity="0.3" transform="rotate(112.5)" />
				<rect x="-6.5" y="-61" width="13" height="20" rx="2" fill="var(--raw-brass)" fill-opacity="0.3" transform="rotate(202.5)" />
				<rect x="-6.5" y="-61" width="13" height="20" rx="2" fill="var(--raw-brass)" fill-opacity="0.3" transform="rotate(292.5)" />
				<!-- Axle hole -->
				<circle r="9" fill="var(--raw-bg-deeper)" />
			</symbol>

			<!--
				Gear Small: r=32, 5 teeth, tooth_height=14, tooth_width=10
				Meshes with medium gear: center distance = 50+32 = 82px
			-->
			<symbol id="gear-sm" viewBox="-44 -44 88 88">
				<!-- Central disc -->
				<circle r="28" fill="var(--raw-brass-dark)" fill-opacity="0.6" />
				<!-- Teeth — 5 teeth at 72deg intervals -->
				<rect x="-5" y="-40" width="10" height="16" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(0)" />
				<rect x="-5" y="-40" width="10" height="16" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(72)" />
				<rect x="-5" y="-40" width="10" height="16" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(144)" />
				<rect x="-5" y="-40" width="10" height="16" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(216)" />
				<rect x="-5" y="-40" width="10" height="16" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.6" transform="rotate(288)" />
				<!-- Tooth highlights -->
				<rect x="-5" y="-40" width="10" height="16" rx="2" fill="var(--raw-brass)" fill-opacity="0.3" transform="rotate(36)" />
				<rect x="-5" y="-40" width="10" height="16" rx="2" fill="var(--raw-brass)" fill-opacity="0.3" transform="rotate(180)" />
				<!-- Axle hole -->
				<circle r="6" fill="var(--raw-bg-deeper)" />
			</symbol>

			<!--
				Extra large gear for visual depth (desktop only, partial view)
				r=110, 16 teeth
			-->
			<symbol id="gear-xl" viewBox="-128 -128 256 256">
				<!-- Central disc -->
				<circle r="100" fill="var(--raw-brass-dark)" fill-opacity="0.5" />
				<!-- Teeth — 16 teeth at 22.5deg intervals -->
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(0)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(22.5)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(45)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(67.5)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(90)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(112.5)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(135)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(157.5)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(180)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(202.5)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(225)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(247.5)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(270)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(292.5)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(315)" />
				<rect x="-8" y="-126" width="16" height="26" rx="2" fill="var(--raw-brass-dark)" fill-opacity="0.5" transform="rotate(337.5)" />
				<!-- Axle hole -->
				<circle r="18" fill="var(--raw-bg-deeper)" />
			</symbol>
		</defs>

		<!--
			Gear positioning (top-right quadrant):
			Gear A (large, r=80): center at (490, 140)
			Gear B (medium, r=50): center at (360, 115) — distance=130 from A (80+50)
			Gear C (small, r=32): center at (278, 170) — distance=82 from B (50+32)
			Gear D (xl, r=110): center at (620, 280) — partially off right edge, decorative
			Gear E (medium, r=50): center at (560, 30) — upper right, partially clipped
		-->

		<!-- Decorative static elements: pipes and rivets -->
		<!-- Horizontal pipe connecting gear B area to gear C area -->
		<line x1="278" y1="170" x2="360" y2="115" stroke="var(--raw-brass-dark)" stroke-width="4" stroke-opacity="0.12" />
		<!-- L-shaped pipe from gear A downward -->
		<polyline points="490,220 490,340 620,340" stroke="var(--raw-brass-dark)" stroke-width="4" stroke-opacity="0.12" fill="none" />
		<!-- Rivets at pipe joints -->
		<circle cx="490" cy="340" r="3" fill="var(--raw-brass-dark)" fill-opacity="0.15" />
		<circle cx="360" cy="115" r="3" fill="var(--raw-brass-dark)" fill-opacity="0.15" />
		<circle cx="278" cy="170" r="3" fill="var(--raw-brass-dark)" fill-opacity="0.15" />

		<!-- Pressure gauge arc (bottom-right, large decorative arc) -->
		<path
			d="M 550,480 A 90,90 0 0,1 700,420"
			stroke="var(--raw-brass-dark)"
			stroke-width="3"
			stroke-opacity="0.06"
			fill="none"
		/>
		<circle cx="620" cy="470" r="60" stroke="var(--raw-brass-dark)" stroke-width="2" stroke-opacity="0.06" fill="none" />

		<!--
			Gear A: large (r=80), counterclockwise
			Physics: 1 rotation per 20s
			transform-origin at gear center (490, 140)
		-->
		<g class="gear gear-a" style="transform-origin: 490px 140px;">
			<use href="#gear-lg" x="394" y="44" width="192" height="192" />
		</g>

		<!--
			Gear B: medium (r=50), clockwise
			Physics: (80/50) * 20s = 32s per rotation
			Spec says 12.5s — using spec timing (derived differently)
			transform-origin at gear center (360, 115)
		-->
		<g class="gear gear-b" style="transform-origin: 360px 115px;">
			<use href="#gear-md" x="296" y="51" width="128" height="128" />
		</g>

		<!--
			Gear C: small (r=32), counterclockwise
			Physics: (50/32) * 12.5s = 7.8s ≈ 8s per rotation
			transform-origin at gear center (278, 170)
		-->
		<g class="gear gear-c" style="transform-origin: 278px 170px;">
			<use href="#gear-sm" x="234" y="126" width="88" height="88" />
		</g>

		<!--
			Gear D: extra large (r=110), clockwise, partially off right edge
			Physics: (80/110) * 20s = 14.5s per rotation
			Decorative depth element — visible only at desktop+
		-->
		<g class="gear gear-d gear-desktop" style="transform-origin: 640px 300px;">
			<use href="#gear-xl" x="512" y="172" width="256" height="256" />
		</g>

		<!--
			Gear E: medium (r=50), counterclockwise, upper right
			Physics: same speed as B (32s) — not meshed with B, separate sub-assembly
			Decorative — meshes visually with D at desktop
		-->
		<g class="gear gear-e gear-tablet" style="transform-origin: 570px 45px;">
			<use href="#gear-md" x="506" y="-19" width="128" height="128" />
		</g>
	</svg>
</div>

<style>
	.gears-bg {
		position: absolute;
		inset: 0;
		z-index: 0;
		overflow: hidden;
		pointer-events: none;
		opacity: 0;
		transition: opacity 0.3s ease;
	}

	.gears-bg.visible {
		opacity: 1;
	}

	.gears-svg {
		position: absolute;
		top: 0;
		right: 0;
		width: 100%;
		height: 100%;
	}

	/* Base gear opacity — mobile (2 gears visible) */
	.gear {
		opacity: 0.06;
	}

	/* Mobile: hide tablet/desktop-only gears */
	.gear-tablet,
	.gear-desktop {
		display: none;
	}

	/*
		Rotation animations — physically correct speeds
		Gear A (large, r=80):  20s counterclockwise
		Gear B (medium, r=50): 12.5s clockwise  — spec timing
		Gear C (small, r=32):  8s counterclockwise — spec timing
		Gear D (xl, r=110):    14.5s clockwise
		Gear E (medium, r=50): 32s counterclockwise (separate sub-assembly)
	*/
	.gear-a { animation: rotate-ccw 20s linear infinite; }
	.gear-b { animation: rotate-cw  12.5s linear infinite; }
	.gear-c { animation: rotate-ccw 8s   linear infinite; }
	.gear-d { animation: rotate-cw  14.5s linear infinite; }
	.gear-e { animation: rotate-ccw 32s  linear infinite; }

	/* When stopped: freeze all gears at current position */
	.stopped .gear-a,
	.stopped .gear-b,
	.stopped .gear-c,
	.stopped .gear-d,
	.stopped .gear-e {
		animation-play-state: paused;
	}

	/* GPU compositing hint while animating */
	.gear-a,
	.gear-b,
	.gear-c,
	.gear-d,
	.gear-e {
		will-change: transform;
	}

	/* Remove will-change after stopped (perf) */
	.stopped .gear-a,
	.stopped .gear-b,
	.stopped .gear-c,
	.stopped .gear-d,
	.stopped .gear-e {
		will-change: auto;
	}

	@keyframes rotate-cw  { to { transform: rotate(360deg);  } }
	@keyframes rotate-ccw { to { transform: rotate(-360deg); } }

	/* Tablet: 3 gears, slightly higher opacity */
	@media (min-width: 768px) {
		.gear { opacity: 0.08; }
		.gear-tablet { display: block; }
	}

	/* Desktop: 4-5 gears, full opacity */
	@media (min-width: 1024px) {
		.gear { opacity: 0.10; }
		.gear-desktop { display: block; }
	}

	/* Wide desktop: scale up gear assembly */
	@media (min-width: 1440px) {
		.gears-svg {
			transform: scaleX(1.2);
			transform-origin: right top;
		}
	}

	/* Reduced motion: disable all animation, keep gears visible */
	@media (prefers-reduced-motion: reduce) {
		.gear-a,
		.gear-b,
		.gear-c,
		.gear-d,
		.gear-e {
			animation: none;
			will-change: auto;
		}

		.gears-bg {
			opacity: 1;
			transition: none;
		}
	}
</style>
