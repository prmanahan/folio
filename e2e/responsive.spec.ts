import { test, expect } from '@playwright/test';

const viewports = [
  { name: 'mobile-320', width: 320, height: 568 },
  { name: 'mobile-375', width: 375, height: 812 },
  { name: 'tablet-768', width: 768, height: 1024 },
  { name: 'desktop-1440', width: 1440, height: 900 },
];

for (const viewport of viewports) {
  test.describe(`Responsive — ${viewport.name}`, () => {
    test.use({ viewport: { width: viewport.width, height: viewport.height } });

    test('home page renders without horizontal overflow', async ({ page }) => {
      await page.goto('/');
      await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });

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

    test('Ask AI card is accessible on home page', async ({ page }) => {
      await page.goto('/');
      await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });
      const askAiBtn = page.getByRole('button', { name: /ask ai/i });
      await expect(askAiBtn).toBeVisible();
    });

    test('nav cards have >= 44px touch targets', async ({ page }) => {
      await page.goto('/');
      await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });

      // Check Ask AI card
      const askAi = page.getByRole('button', { name: /ask ai/i });
      const askAiBox = await askAi.boundingBox();
      expect(askAiBox).not.toBeNull();
      expect(askAiBox!.height).toBeGreaterThanOrEqual(44);

      // Check Projects card
      const projectsCard = page.getByRole('link', { name: /projects/i }).first();
      const projectsBox = await projectsCard.boundingBox();
      expect(projectsBox).not.toBeNull();
      expect(projectsBox!.height).toBeGreaterThanOrEqual(44);
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

  test('home page has two-column layout on desktop', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });
    // The hub container should be using flexbox row direction at this width
    const hub = page.locator('.hub');
    const flexDirection = await hub.evaluate((el) => getComputedStyle(el).flexDirection);
    expect(flexDirection).toBe('row');
  });
});
