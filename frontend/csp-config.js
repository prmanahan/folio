// The single source of the page CSP. Two readers, both import this file:
//  - vite.config.ts passes it to sveltekit({ csp }), which emits the policy;
//  - scripts/csp-lib.mjs reads its directive names for the guard's shape check.
// Never copy this list into either of them.
//
// HTML pages get their script-hash CSP from this object, as a <meta http-equiv> tag.
// The guard (`npm run check:csp`) validates the built page's meta policy
// and writes it to build/csp-header.txt. The server sends that file as the
// Content-Security-Policy header with `; frame-ancestors 'none'` appended.
// STATIC_CSP is only the fallback when the file is unusable (see
// docs/RISK-REGISTER.md RISK-2).
// frame-ancestors / report-uri / sandbox are ignored in a meta tag.

/** @satisfies {NonNullable<import('@sveltejs/kit/vite').Config['csp']>} */
export const csp = {
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
};
