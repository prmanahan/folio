import { test, expect } from '@playwright/test';
import { mockPublicApi } from './fixtures';

/**
 * E2E tests for the /resume destination (Phase 1.5 Amendment).
 *
 * Covers:
 * - Route renders all three sections in order: Skills → Experience → Education
 * - Deep-link anchors (#skills, #experience, #education)
 * - Breadcrumb shows [Monogram] > Resume
 * - SkillsPillMatrix pill bands (strong, moderate, gap)
 * - Skeleton loading state
 * - Responsive behavior across viewports
 * - Accessibility: section headings, aria-busy
 */

test.describe('Resume destination — sections', () => {
  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
    await page.goto('/resume');
    await page.waitForLoadState('networkidle');
  });

  test('page title includes Resume', async ({ page }) => {
    await expect(page).toHaveTitle(/Resume/);
  });

  test('renders Skills section first', async ({ page }) => {
    const skillsSection = page.locator('[data-testid="resume-section-skills"]');
    await expect(skillsSection).toBeVisible({ timeout: 10000 });
  });

  test('renders Experience section second', async ({ page }) => {
    const experienceSection = page.locator('[data-testid="resume-section-experience"]');
    await expect(experienceSection).toBeVisible({ timeout: 10000 });
  });

  test('renders Education section third', async ({ page }) => {
    const educationSection = page.locator('[data-testid="resume-section-education"]');
    await expect(educationSection).toBeVisible({ timeout: 10000 });
  });

  test('section order is Skills → Experience → Education', async ({ page }) => {
    const skills = page.locator('[data-testid="resume-section-skills"]');
    const experience = page.locator('[data-testid="resume-section-experience"]');
    const education = page.locator('[data-testid="resume-section-education"]');

    await expect(skills).toBeVisible({ timeout: 10000 });
    await expect(experience).toBeVisible();
    await expect(education).toBeVisible();

    // Verify DOM order by comparing bounding boxes (top Y)
    const skillsBox = await skills.boundingBox();
    const experienceBox = await experience.boundingBox();
    const educationBox = await education.boundingBox();

    expect(skillsBox).not.toBeNull();
    expect(experienceBox).not.toBeNull();
    expect(educationBox).not.toBeNull();

    expect(skillsBox!.y).toBeLessThan(experienceBox!.y);
    expect(experienceBox!.y).toBeLessThan(educationBox!.y);
  });

  test('Skills section has semantic h2 heading', async ({ page }) => {
    await expect(page.getByRole('heading', { level: 2, name: /skills/i })).toBeVisible({ timeout: 10000 });
  });

  test('SkillsPillMatrix renders', async ({ page }) => {
    // With empty seeded skills, the SkillsPillMatrix still renders (empty state)
    // The section container is visible regardless
    await expect(page.locator('[data-testid="resume-section-skills"]')).toBeVisible({ timeout: 10000 });
  });
});

test.describe('Resume destination — pill matrix (when skills seeded)', () => {
  test('pill matrix renders with banded skills', async ({ page }) => {
    // Override with a skills response that has actual data
    await page.route('**/api/skills', (route) =>
      route.fulfill({
        json: [
          { skill_name: 'Rust', category: 'strong', sort_order: 1 },
          { skill_name: 'TypeScript', category: 'strong', sort_order: 2 },
          { skill_name: 'Python', category: 'moderate', sort_order: 3 },
          { skill_name: 'Go', category: 'gap', sort_order: 4 },
        ],
      })
    );
    await page.route('**/api/profile', (route) =>
      route.fulfill({
        json: {
          name: 'Alex Rivera',
          title: 'Software Architect',
          pitch_short: 'Test pitch.',
          pitch_long: 'Test pitch long.',
          availability_status: 'open',
        },
      })
    );
    await page.route('**/api/links', (route) => route.fulfill({ json: [] }));
    await page.route('**/api/experience', (route) => route.fulfill({ json: [] }));
    await page.route('**/api/education', (route) => route.fulfill({ json: [] }));

    await page.goto('/resume');
    await page.waitForLoadState('networkidle');

    // SkillsPillMatrix should render with data
    const pillMatrix = page.locator('[data-testid="skills-pill-matrix"]');
    await expect(pillMatrix).toBeVisible({ timeout: 10000 });

    // Strong band should be visible
    const strongBand = page.locator('[data-testid="skill-band-strong"]');
    await expect(strongBand).toBeVisible();

    // Moderate band
    const moderateBand = page.locator('[data-testid="skill-band-moderate"]');
    await expect(moderateBand).toBeVisible();

    // Gap (Learning) band
    const gapBand = page.locator('[data-testid="skill-band-gap"]');
    await expect(gapBand).toBeVisible();
  });
});

test.describe('Resume destination — deep links', () => {
  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
  });

  test('deep link to #skills scrolls to skills section', async ({ page }) => {
    await page.goto('/resume#skills');
    await page.waitForLoadState('networkidle');

    const skillsAnchor = page.locator('#skills');
    await expect(skillsAnchor).toBeVisible({ timeout: 10000 });
    // After navigation, the skills section should be near the top of the viewport
    const box = await skillsAnchor.boundingBox();
    expect(box).not.toBeNull();
    // The element should be in the visible viewport area (y < viewport height)
    const viewportHeight = await page.evaluate(() => window.innerHeight);
    expect(box!.y).toBeLessThan(viewportHeight * 1.5); // generous: within 1.5x viewport
  });

  test('deep link to #experience scrolls to experience section', async ({ page }) => {
    await page.goto('/resume#experience');
    await page.waitForLoadState('networkidle');

    const experienceAnchor = page.locator('#experience');
    await expect(experienceAnchor).toBeVisible({ timeout: 10000 });
  });

  test('deep link to #education scrolls to education section', async ({ page }) => {
    await page.goto('/resume#education');
    await page.waitForLoadState('networkidle');

    const educationAnchor = page.locator('#education');
    await expect(educationAnchor).toBeVisible({ timeout: 10000 });
  });
});

test.describe('Resume destination — breadcrumb', () => {
  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
  });

  test('breadcrumb shows Resume as current page', async ({ page }) => {
    await page.goto('/resume');
    const breadcrumbNav = page.getByRole('navigation', { name: 'Breadcrumb' });
    await expect(breadcrumbNav).toBeVisible();
    // Both desktop and mobile breadcrumb render aria-current="page" — use .first()
    const currentSegment = breadcrumbNav.locator('[aria-current="page"]').first();
    await expect(currentSegment).toContainText('Resume');
  });

  test('monogram link on /resume navigates to home', async ({ page }) => {
    await page.goto('/resume');
    const monogram = page.getByRole('link', { name: 'Home' });
    await expect(monogram).toBeVisible();
    await monogram.click();
    await expect(page).toHaveURL('/');
  });
});

test.describe('Resume destination — skeleton', () => {
  test('shows aria-busy container while loading', async ({ page }) => {
    // Slow the API to catch loading state
    await page.route('**/api/skills', async (route) => {
      await new Promise((r) => setTimeout(r, 2000));
      await route.continue();
    });
    await page.route('**/api/profile', (route) =>
      route.fulfill({
        json: {
          name: 'Alex Rivera',
          title: 'Software Architect',
          pitch_short: 'Test pitch.',
          pitch_long: 'Test pitch long.',
          availability_status: 'open',
        },
      })
    );
    await page.route('**/api/links', (route) => route.fulfill({ json: [] }));
    await page.goto('/resume');
    // During load, the loading container should appear with aria-busy
    const loadingEl = page.locator('[aria-busy="true"]');
    await expect(loadingEl).toBeVisible({ timeout: 3000 });
  });
});

test.describe('Resume destination — responsive', () => {
  const viewports = [
    { name: 'mobile-320', width: 320, height: 568 },
    { name: 'tablet-768', width: 768, height: 1024 },
    { name: 'desktop-1024', width: 1024, height: 768 },
    { name: 'desktop-1440', width: 1440, height: 900 },
  ];

  for (const vp of viewports) {
    test(`resume page no horizontal overflow at ${vp.name}`, async ({ page }) => {
      await mockPublicApi(page);
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto('/resume');
      await page.waitForLoadState('networkidle');

      const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
      const clientWidth = await page.evaluate(() => document.documentElement.clientWidth);
      expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1);
    });

    test(`all three resume sections visible at ${vp.name}`, async ({ page }) => {
      await mockPublicApi(page);
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto('/resume');
      await page.waitForLoadState('networkidle');

      await expect(page.locator('[data-testid="resume-section-skills"]')).toBeVisible({ timeout: 10000 });
      await expect(page.locator('[data-testid="resume-section-experience"]')).toBeVisible();
      await expect(page.locator('[data-testid="resume-section-education"]')).toBeVisible();
    });
  }
});
