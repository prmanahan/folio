// Mock for $app/navigation (SvelteKit)
export const goto = async () => {};
export const invalidate = async () => {};
export const invalidateAll = async () => {};
export const preloadData = async () => ({ type: 'loaded', status: 200, data: {} });
export const preloadCode = async () => {};
export const beforeNavigate = () => {};
export const afterNavigate = () => {};
export const pushState = () => {};
export const replaceState = () => {};
