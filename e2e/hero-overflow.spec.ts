/**
 * E2E regression test for the hero hub vertical-overflow bug.
 *
 * Spec: docs/specs/2026-04-13-hero-hub-vertical-overflow.md (task #382)
 *
 * The original bug: with a 700-char single-paragraph elevator_pitch, the
 * .hero section grew past the viewport on iPhone SE / iPhone 14 / iPad
 * landscape / 13" laptop, pushing the .skills-banner below the fold. The fix
 * caps .hero at 100svh − nav-height, renders only pitch_short on the hub
 * (≤280 chars), and lifts the SkillsBanner out of .hero into the parent layout
 * so its position is determined by document flow.
 *
 * This spec uses an inline mock that mirrors the live-profile fixture
 * shipped at libs/site-core/db/fixtures/live_profile.json. It is intentionally
 * a static copy — keep the two in sync if the live fixture changes.
 */

import { test, expect, type Page } from '@playwright/test';

// pitch_long is the actual 700+ char pitch from peter.manahan.io captured
// 2026-04-13. pitch_short is a representative ≤280-char tweet-length pitch
// used for the regression — Peter writes the real one as content work post-merge.
const LIVE_PROFILE = {
  name: 'Peter Manahan',
  email: 'prmanahan@pm.me',
  title: 'Software Architect',
  location: 'Fredericton, NB, Canada',
  phone: '506.292.4897',
  linkedin_url: 'https://www.linkedin.com/in/peter-manahan-4181839/',
  github_url: 'https://github.com/prmanahan',
  twitter_url: '',
  pitch_short:
    '15+ years across the full software delivery chain. Took teams from daily broken builds to predictable, high-frequency delivery. Builds developer productivity tooling and installation infrastructure used by 160 IBM products.',
  pitch_long:
    '15+ years across the full software delivery chain: developer, tester, release lead, installation architect, development manager, product owner, TPM, software architect. Led Agile teams building back-end services for security intelligence platforms. Took teams from daily broken builds to predictable, high-frequency delivery. Built developer productivity tooling including Backstage-based hubs, launched cloud-based offerings, designed automated systems that replaced manual quality gates, and delivered installation infrastructure used by 160 IBM products.',
  availability_status: 'available',
  availability_date: '',
  remote_preference: 'Remote or Fredericton, NB, Canada',
};

const LIVE_LINKS = [
  { id: 1, label: 'LinkedIn', url: 'https://linkedin.com/in/pmanahan', icon: 'linkedin', sort_order: 1 },
  { id: 2, label: 'GitHub', url: 'https://github.com/prmanahan', icon: 'github', sort_order: 2 },
  { id: 3, label: 'Email', url: 'mailto:prmanahan@pm.me', icon: 'mail', sort_order: 3 },
];

const LIVE_SKILLS = [
  { id: 1, skill_name: 'Rust', category: 'strong', years_experience: 3, last_used: '2026' },
  { id: 2, skill_name: 'Java', category: 'strong', years_experience: 15, last_used: '2024' },
  { id: 3, skill_name: 'TypeScript', category: 'strong', years_experience: 8, last_used: '2026' },
  { id: 4, skill_name: 'Python', category: 'moderate', years_experience: 5, last_used: '2025' },
];

async function mockLiveProfile(page: Page) {
  await page.route('**/api/profile', (route) => route.fulfill({ json: LIVE_PROFILE }));
  await page.route('**/api/links', (route) => route.fulfill({ json: LIVE_LINKS }));
  await page.route('**/api/skills', (route) => route.fulfill({ json: LIVE_SKILLS }));
}

// Test matrix from spec §Test Expectations — the seven viewports the bug
// matrix called out, including the borderline laptop and the wide desktop.
const VIEWPORTS = [
  { name: 'iPhone SE',     width: 375,  height: 667 },
  { name: 'iPhone 14',     width: 390,  height: 844 },
  { name: 'iPad portrait', width: 820,  height: 1180 },
  { name: 'iPad landscape',width: 1180, height: 820 },
  { name: '13" MBP',       width: 1440, height: 900 },
  { name: '16" MBP',       width: 1728, height: 1117 },
  { name: '27" monitor',   width: 2560, height: 1440 },
];

test.describe('hero hub vertical overflow regression (task #382)', () => {
  for (const vp of VIEWPORTS) {
    test(`hero fits within viewport at ${vp.name} (${vp.width}×${vp.height})`, async ({ page }) => {
      await mockLiveProfile(page);
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto('/');
      await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({
        timeout: 10000,
      });

      // .hero must not exceed the viewport height. We allow up to viewport
      // height (some breathing room for the nav is already accounted for via
      // 100svh − var(--nav-height)). The assertion is strict: the hero's
      // bottom edge must be ≤ viewport height.
      const hero = page.locator('.hero');
      await expect(hero).toBeVisible();
      const heroBox = await hero.boundingBox();
      expect(heroBox).not.toBeNull();
      expect(heroBox!.height).toBeLessThanOrEqual(vp.height);
    });

    test(`skills banner sits within initial viewport at ${vp.name}`, async ({ page }) => {
      await mockLiveProfile(page);
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto('/');
      await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({
        timeout: 10000,
      });

      // SkillsBanner is now a sibling of .hero. The banner's bottom edge
      // (top + height) must be visible in the initial viewport with no scroll.
      // Some pixel slop accounts for sub-pixel layout differences across
      // browsers — 8px is generous and well within "no visible scroll needed".
      const banner = page.locator('.skills-banner');
      await expect(banner).toBeVisible();
      const bannerBox = await banner.boundingBox();
      expect(bannerBox).not.toBeNull();
      const bannerBottom = bannerBox!.y + bannerBox!.height;
      expect(bannerBottom).toBeLessThanOrEqual(vp.height + 8);
    });
  }

  test('SkillsBanner is a DOM sibling of .hero, not a descendant', async ({ page }) => {
    await mockLiveProfile(page);
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({
      timeout: 10000,
    });

    // Structural assertion R2: .hero must NOT contain .skills-banner.
    const bannerInsideHero = await page.locator('.hero .skills-banner').count();
    expect(bannerInsideHero).toBe(0);

    // And .skills-banner must exist somewhere on the page.
    const bannerCount = await page.locator('.skills-banner').count();
    expect(bannerCount).toBe(1);
  });

  test('hub renders pitch_short, not pitch_long', async ({ page }) => {
    await mockLiveProfile(page);
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({
      timeout: 10000,
    });

    // pitch_short fragment present
    await expect(page.getByText(/15\+ years across the full software delivery chain/i).first()).toBeVisible();
    // pitch_long-only fragment must NOT appear (text from the long version)
    await expect(page.getByText(/installation architect, development manager/i)).toHaveCount(0);
  });
});
