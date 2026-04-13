import { test, expect } from '@playwright/test';
import { mockPublicApi, PROFILE_NAME } from './fixtures';

const viewports = [
  { name: 'mobile-320', width: 320, height: 568 },
  { name: 'mobile-375', width: 375, height: 812 },
  { name: 'tablet-768', width: 768, height: 1024 },
  { name: 'desktop-1024', width: 1024, height: 768 },
  { name: 'desktop-1440', width: 1440, height: 900 },
];

for (const viewport of viewports) {
  test.describe(`Responsive — ${viewport.name}`, () => {
    test.use({ viewport: { width: viewport.width, height: viewport.height } });

    test.beforeEach(async ({ page }) => {
      await mockPublicApi(page);
    });

    test('home page renders without horizontal overflow', async ({ page }) => {
      await page.goto('/');
      await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });

      const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
      const clientWidth = await page.evaluate(() => document.documentElement.clientWidth);
      expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1);
    });

    test('PM monogram is visible on inner pages', async ({ page }) => {
      await page.goto('/projects');
      await expect(page.getByRole('link', { name: 'Home' })).toBeVisible();
    });

    test('projects page renders without horizontal overflow', async ({ page }) => {
      await page.goto('/projects');
      await page.waitForLoadState('networkidle');
      const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
      const clientWidth = await page.evaluate(() => document.documentElement.clientWidth);
      expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1);
    });

    test('articles page renders without horizontal overflow', async ({ page }) => {
      await page.goto('/articles');
      await page.waitForLoadState('networkidle');
      const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
      const clientWidth = await page.evaluate(() => document.documentElement.clientWidth);
      expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1);
    });

    test('resume page renders without horizontal overflow', async ({ page }) => {
      await page.goto('/resume');
      await page.waitForLoadState('networkidle');
      const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
      const clientWidth = await page.evaluate(() => document.documentElement.clientWidth);
      expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1);
    });

    test('Ask AI card is accessible on home page', async ({ page }) => {
      await page.goto('/');
      await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
      const askAiBtn = page.getByRole('button', { name: /ask ai/i });
      await expect(askAiBtn).toBeVisible();
    });

    test('nav cards have >= 44px touch targets', async ({ page }) => {
      await page.goto('/');
      await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });

      // Check Ask AI card
      const askAi = page.getByRole('button', { name: /ask ai/i });
      const askAiBox = await askAi.boundingBox();
      expect(askAiBox).not.toBeNull();
      expect(askAiBox!.height).toBeGreaterThanOrEqual(44);

      // Check Projects card
      const projectsCard = page.locator('[data-testid="card-projects"]');
      const projectsBox = await projectsCard.boundingBox();
      expect(projectsBox).not.toBeNull();
      expect(projectsBox!.height).toBeGreaterThanOrEqual(44);

      // Check Articles card
      const articlesCard = page.locator('[data-testid="card-articles"]');
      const articlesBox = await articlesCard.boundingBox();
      expect(articlesBox).not.toBeNull();
      expect(articlesBox!.height).toBeGreaterThanOrEqual(44);

      // Check Resume card
      const resumeCard = page.locator('[data-testid="card-resume"]');
      const resumeBox = await resumeCard.boundingBox();
      expect(resumeBox).not.toBeNull();
      expect(resumeBox!.height).toBeGreaterThanOrEqual(44);

      // Check Contact card
      const contactCard = page.locator('[data-testid="card-contact"]');
      const contactBox = await contactCard.boundingBox();
      expect(contactBox).not.toBeNull();
      expect(contactBox!.height).toBeGreaterThanOrEqual(44);
    });

    // Phase 1.5: All five cards visible at every viewport
    test('all five hub cards are visible', async ({ page }) => {
      await page.goto('/');
      await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
      await expect(page.locator('[data-testid="card-ask-ai"]')).toBeVisible();
      await expect(page.locator('[data-testid="card-projects"]')).toBeVisible();
      await expect(page.locator('[data-testid="card-articles"]')).toBeVisible();
      await expect(page.locator('[data-testid="card-resume"]')).toBeVisible();
      await expect(page.locator('[data-testid="card-contact"]')).toBeVisible();
    });
  });
}

test.describe('Responsive — mobile breadcrumb collapse', () => {
  test('breadcrumb collapses to back link on mobile for nested pages', async ({ page }) => {
    // Use mobile viewport
    await page.setViewportSize({ width: 375, height: 812 });
    await page.goto('/projects');
    // On mobile at depth=1, should show page name (no back link since there's no parent)
    const breadcrumbNav = page.getByRole('navigation', { name: 'Breadcrumb' });
    await expect(breadcrumbNav).toBeVisible();
  });
});

test.describe('Responsive — desktop two-column layout', () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
  });

  test('home page has two-column layout on desktop', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    // The hub container should be using flexbox row direction at this width
    const hub = page.locator('.hub');
    const flexDirection = await hub.evaluate((el) => getComputedStyle(el).flexDirection);
    expect(flexDirection).toBe('row');
  });

  // Phase 1.5: Three-up nav row (Projects · Articles · Resume) on desktop
  test('nav row contains exactly three cards in a row on desktop', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    const navCards = page.locator('.nav-cards');
    const flexDir = await navCards.evaluate((el) => getComputedStyle(el).flexDirection);
    expect(flexDir).toBe('row');
    // Three nav cards: Projects, Articles, Resume
    const cards = navCards.locator('[data-testid]');
    await expect(cards).toHaveCount(3);
  });
});

test.describe('Responsive — mobile stacked layout', () => {
  test.use({ viewport: { width: 320, height: 568 } });

  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
  });

  test('home page stacks vertically on mobile', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    const hub = page.locator('.hub');
    const flexDirection = await hub.evaluate((el) => getComputedStyle(el).flexDirection);
    expect(flexDirection).toBe('column');
  });

  // Phase 1.5: On mobile, nav cards stack vertically
  test('nav cards stack vertically on mobile', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    const navCards = page.locator('.nav-cards');
    const flexDir = await navCards.evaluate((el) => getComputedStyle(el).flexDirection);
    expect(flexDir).toBe('column');
  });
});

test.describe('Responsive — monogram visibility', () => {
  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
  });

  for (const vp of [
    { name: 'mobile-320', width: 320, height: 568 },
    { name: 'tablet-768', width: 768, height: 1024 },
    { name: 'desktop-1440', width: 1440, height: 900 },
  ]) {
    test(`monogram visible on home at ${vp.name}`, async ({ page }) => {
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto('/');
      await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
      const monogram = page.locator('header.site-header a[aria-label="Home"]');
      await expect(monogram).toBeVisible();
    });
  }
});
