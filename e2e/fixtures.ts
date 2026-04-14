/**
 * Shared test fixtures for E2E tests.
 *
 * Provides mock API responses that match the seeded DB data ("Alex Rivera" profile).
 * Used by home-page tests to avoid hitting the backend rate limiter (60 req/min)
 * when the full Playwright suite runs many sequential navigations.
 *
 * The mock data is intentionally pinned to the seed values — changes to seed data
 * require updating these fixtures.
 */

import type { Page } from '@playwright/test';

export const SEEDED_PROFILE = {
  name: 'Alex Rivera',
  email: 'alex@example.com',
  title: 'Software Architect / Engineering Manager',
  location: 'Vancouver, BC',
  phone: '604-555-0199',
  linkedin_url: 'https://linkedin.com/in/alex-rivera-example',
  github_url: 'https://github.com/alex-rivera-example',
  twitter_url: '',
  pitch_short:
    'Software architect with 12 years building distributed systems. Backend Java to architecture and team leadership.',
  pitch_long:
    'Software architect with 12 years building distributed systems. Started in backend Java, evolved into architecture and team leadership. Track record of shipping reliable systems at scale while growing engineering teams.',
  availability_status: 'open',
  availability_date: '',
  remote_preference: 'remote',
};

export const SEEDED_LINKS = [
  { id: 3, label: 'LinkedIn', url: 'https://linkedin.com/in/alex-rivera-example', icon: 'linkedin', sort_order: 1 },
  { id: 4, label: 'GitHub', url: 'https://github.com/alex-rivera-example', icon: 'github', sort_order: 2 },
  { id: 5, label: 'Email', url: 'mailto:alex@example.com', icon: 'mail', sort_order: 3 },
];

export const SEEDED_SKILLS: { skill_name: string; category: string; sort_order: number }[] = [];

/**
 * Mock all public API routes with seeded fixture data.
 * Call this in beforeEach for test suites that navigate to `/`.
 * Prevents rate-limit exhaustion when many tests run in sequence.
 */
export async function mockPublicApi(page: Page): Promise<void> {
  await page.route('**/api/profile', (route) =>
    route.fulfill({ json: SEEDED_PROFILE })
  );
  await page.route('**/api/links', (route) =>
    route.fulfill({ json: SEEDED_LINKS })
  );
  await page.route('**/api/skills', (route) =>
    route.fulfill({ json: SEEDED_SKILLS })
  );
}

/** Profile name regex for assertions. */
export const PROFILE_NAME = /Alex Rivera/i;
/** Expected monogram initials for seeded profile. */
export const PROFILE_INITIALS = 'AR';
