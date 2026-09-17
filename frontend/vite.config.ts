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
		plugins: [tailwindcss(), sveltekit()],
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
