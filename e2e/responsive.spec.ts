import { test, expect } from '@playwright/test';

const viewports = [
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
      expect(scrollWidth).toBeLessThanOrEqual(clientWidth + 1); // +1 for rounding
    });

    test('navigation is visible and accessible', async ({ page }) => {
      await page.goto('/');
      await expect(page.getByRole('navigation')).toBeVisible();
      // Nav brand should always be visible
      await expect(page.getByRole('link', { name: 'PM' })).toBeVisible();
    });

    test('projects page renders without horizontal overflow', async ({ page }) => {
      await page.goto('/projects');
      // Wait for page to settle
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

    test('Ask AI button is accessible', async ({ page }) => {
      await page.goto('/');
      const askAiBtn = page.getByRole('button', { name: /ask ai/i });
      await expect(askAiBtn).toBeVisible();
    });
  });
}

test.describe('Responsive — mobile nav links reachable', () => {
  test.use({ viewport: { width: 375, height: 812 } });

  test('all nav links are still reachable at mobile width', async ({ page }) => {
    await page.goto('/');
    // The nav has no hamburger menu - links are inline. Verify they exist even if small.
    const nav = page.getByRole('navigation');
    await expect(nav.getByRole('link', { name: 'Home' })).toBeAttached();
    await expect(nav.getByRole('link', { name: 'Projects' })).toBeAttached();
    await expect(nav.getByRole('link', { name: 'Articles' })).toBeAttached();
  });
});
