import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

export default defineConfig({
	plugins: [
		svelte({ hot: !process.env.VITEST }),
	],
	resolve: {
		conditions: ['browser'],
		alias: {
			$lib: path.resolve(__dirname, './src/lib'),
			$app: path.resolve(__dirname, './src/test/mocks/app'),
		},
	},
	test: {
		include: ['src/**/*.{test,spec}.{js,ts}'],
		globals: true,
		environment: 'jsdom',
		setupFiles: ['src/test/setup.ts'],
		alias: {
			$lib: path.resolve(__dirname, './src/lib'),
			$app: path.resolve(__dirname, './src/test/mocks/app'),
		},
		coverage: {
			provider: 'v8',
			reporter: ['text', 'html', 'lcov'],
			reportsDirectory: './coverage',
			include: ['src/**/*.{ts,svelte}'],
			// `src/test/**` is load-bearing: setup.ts and the $app mocks are
			// helpers, not tests, so Vitest's own defaults do not drop them.
			// There is deliberately NO `src/lib/__tests__/**` entry. MEASURED
			// 2026-09-04: no *.test.ts row appears in the report either way,
			// and adding that entry moved the total by 0.00 points, so it
			// would be inert. Positive control, so that is a real absence and
			// not a dead exclude list: adding `src/routes/**` moves the total
			// 11.53% -> 37.93%. Why Vitest 4 drops them unasked is not
			// something this measured, so it is not claimed here.
			// `**/*.server.ts` matches no file today. Carried forward from the
			// original branch as a guard for SvelteKit server modules, which a
			// jsdom unit test cannot reach.
			exclude: [
				'**/*.server.ts',
				'src/test/**',
			],
			// No thresholds on purpose (#2750): these recipes measure and
			// report, they do not gate. A threshold set below current coverage
			// passes unconditionally while reading like enforcement — the exact
			// defect this task removed from the backend recipes.
		},
	},
});
