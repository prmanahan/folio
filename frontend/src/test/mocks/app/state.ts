// Mock for $app/state (SvelteKit)
// Mutable so tests can override pathname before rendering
export const page = {
	url: { pathname: '/' },
	params: {},
	route: { id: '/' },
	status: 200,
	error: null,
	data: {},
	form: undefined,
	state: {},
};

/** Set the mock page pathname for a test. Resets to '/' if not called. */
export function setMockPathname(pathname: string) {
	page.url.pathname = pathname;
}
