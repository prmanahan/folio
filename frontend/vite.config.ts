import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, loadEnv } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import path from 'node:path';

export default defineConfig(({ mode }) => {
	// Task #3558: the backend rejects any request without the origin
	// lock's shared secret. Read it from the repo-root `.env` (not
	// `frontend/.env` — `loadEnv`'s third argument, an empty prefix,
	// is what exposes a non-`VITE_`-prefixed var here). This value is
	// used only inside this Node-side config, never passed to
	// `define`/`import.meta.env`, so it never reaches client bundle code.
	const env = loadEnv(mode, path.resolve(import.meta.dirname, '..'), '');

	return {
		plugins: [
			tailwindcss(),
			sveltekit({
				preprocess: vitePreprocess(),
				adapter: adapter({ fallback: 'index.html' }),
				// HTML pages get their script-hash CSP here, as a <meta http-equiv> tag.
				// The guard (`npm run check:csp`) validates the built page's meta policy
				// and writes it to build/csp-header.txt. The server sends that file as the
				// Content-Security-Policy header with `; frame-ancestors 'none'` appended.
				// STATIC_CSP is only the fallback when the file is unusable (see
				// docs/RISK-REGISTER.md RISK-2).
				// frame-ancestors / report-uri / sandbox are ignored in a meta tag.
				csp: {
					mode: 'hash',
					directives: {
						'default-src': ['self'],
						'script-src': ['self'],
						'style-src': ['self', 'unsafe-inline', 'https://fonts.googleapis.com'],
						'font-src': ['self', 'https://fonts.gstatic.com'],
						'img-src': ['self', 'data:'],
						'connect-src': ['self'],
						'object-src': ['none'],
						'base-uri': ['none'],
						'form-action': ['self']
					}
				}
			})
		],
		server: {
			proxy: {
				'/api': {
					target: 'http://localhost:3000',
					changeOrigin: true,
					configure: (proxy) => {
						proxy.on('proxyReq', (proxyReq) => {
							if (env.EDGE_AUTH_TOKEN) {
								proxyReq.setHeader('X-Folio-Edge-Auth', env.EDGE_AUTH_TOKEN);
							}
						});
					}
				}
			}
		}
	};
});
