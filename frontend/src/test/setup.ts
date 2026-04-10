import '@testing-library/jest-dom';

// Mock IntersectionObserver for jsdom (used by SkillsBanner)
class MockIntersectionObserver {
	readonly root: Element | null = null;
	readonly rootMargin: string = '';
	readonly thresholds: ReadonlyArray<number> = [];
	observe() {}
	unobserve() {}
	disconnect() {}
	takeRecords(): IntersectionObserverEntry[] { return []; }
}

(globalThis as Record<string, unknown>).IntersectionObserver = MockIntersectionObserver;
