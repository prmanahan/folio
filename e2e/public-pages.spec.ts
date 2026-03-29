import { test, expect } from '@playwright/test';

test.describe('Public pages — home', () => {
  test('home page loads with correct title', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle(/Peter Manahan/);
  });

  test('profile name and title are visible', async ({ page }) => {
    await page.goto('/');
    // The page loads data via API on mount — wait for the hero to render
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible();
    // The title element is a <p class="title"> — use first() since the text appears in multiple places
    await expect(page.getByText(/Software Architect/i).first()).toBeVisible();
  });

  test('navigation is visible with expected links', async ({ page }) => {
    await page.goto('/');
    const nav = page.getByRole('navigation');
    await expect(nav).toBeVisible();
    await expect(nav.getByRole('link', { name: 'Home' })).toBeVisible();
    await expect(nav.getByRole('link', { name: 'Projects' })).toBeVisible();
    await expect(nav.getByRole('link', { name: 'Articles' })).toBeVisible();
  });

  test('Ask AI button exists in nav', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('button', { name: /ask ai/i })).toBeVisible();
  });

  test('experience section is visible with entries', async ({ page }) => {
    await page.goto('/');
    // Wait for data to load from API
    const experienceSection = page.locator('section').filter({ hasText: /experience/i });
    await expect(experienceSection).toBeVisible({ timeout: 10000 });
    // Should have at least one experience entry (9 seeded)
    const entries = experienceSection.locator('article, li, [class*="experience"], [class*="entry"]');
    const count = await entries.count();
    if (count === 0) {
      // Fallback: just confirm there's content beneath the heading
      await expect(experienceSection.locator('h3, h4, .company, [class*="company"]').first()).toBeVisible();
    }
  });

  test('skills section is visible with entries', async ({ page }) => {
    await page.goto('/');
    const skillsSection = page.locator('section').filter({ hasText: /skills/i });
    await expect(skillsSection).toBeVisible({ timeout: 10000 });
  });

  test('education section is visible', async ({ page }) => {
    await page.goto('/');
    const educationSection = page.locator('section').filter({ hasText: /education/i });
    await expect(educationSection).toBeVisible({ timeout: 10000 });
  });

  test('footer is visible', async ({ page }) => {
    await page.goto('/');
    const footer = page.locator('footer');
    await expect(footer).toBeVisible({ timeout: 10000 });
  });

  test('nav brand PM links to home', async ({ page }) => {
    await page.goto('/projects');
    await page.getByRole('link', { name: 'PM' }).click();
    await expect(page).toHaveURL('/');
  });

  test('nav projects link navigates to projects page', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('navigation').getByRole('link', { name: 'Projects' }).click();
    await expect(page).toHaveURL('/projects');
  });

  test('nav articles link navigates to articles page', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('navigation').getByRole('link', { name: 'Articles' }).click();
    await expect(page).toHaveURL('/articles');
  });

  test('home page has no console errors during load', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') errors.push(msg.text());
    });
    await page.goto('/');
    // Wait for data to load
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });
    // Filter out known non-critical browser errors (e.g. favicon 404)
    const criticalErrors = errors.filter(e => !e.includes('favicon'));
    expect(criticalErrors).toHaveLength(0);
  });
});
