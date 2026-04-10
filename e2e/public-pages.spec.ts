import { test, expect } from '@playwright/test';

test.describe('Public pages — home (hub layout)', () => {
  test('home page loads with correct title', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle(/Peter Manahan/);
  });

  test('profile name and title are visible', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(/Software Architect/i).first()).toBeVisible();
  });

  test('availability badge is visible', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(/open to opportunities/i)).toBeVisible();
  });

  test('no traditional nav bar on home page', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });
    // The old Nav component had links like Home, Projects, Agents, Articles inline
    // Those should NOT be present as nav links anymore
    const header = page.locator('header.site-header');
    await expect(header).not.toBeVisible();
  });

  test('Ask AI card exists on home page', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('button', { name: /ask ai/i })).toBeVisible();
  });

  test('Projects card links to /projects', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });
    const projectsCard = page.getByRole('link', { name: /projects/i }).first();
    await expect(projectsCard).toBeVisible();
    await projectsCard.click();
    await expect(page).toHaveURL('/projects');
  });

  test('Articles card links to /articles', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });
    const articlesCard = page.getByRole('link', { name: /articles/i }).first();
    await expect(articlesCard).toBeVisible();
    await articlesCard.click();
    await expect(page).toHaveURL('/articles');
  });

  test('Contact card is visible with links', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Contact')).toBeVisible();
  });

  test('footer is visible', async ({ page }) => {
    await page.goto('/');
    const footer = page.locator('footer');
    await expect(footer).toBeVisible({ timeout: 10000 });
  });

  test('home page has no console errors during load', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') errors.push(msg.text());
    });
    await page.goto('/');
    await expect(page.getByRole('heading', { name: /Peter Manahan/i })).toBeVisible({ timeout: 10000 });
    const criticalErrors = errors.filter(e => !e.includes('favicon'));
    expect(criticalErrors).toHaveLength(0);
  });
});

test.describe('Public pages — inner pages breadcrumb', () => {
  test('PM monogram links to home from inner page', async ({ page }) => {
    await page.goto('/projects');
    const monogram = page.getByRole('link', { name: 'Home' });
    await expect(monogram).toBeVisible();
    await monogram.click();
    await expect(page).toHaveURL('/');
  });

  test('projects page shows breadcrumb with "Projects"', async ({ page }) => {
    await page.goto('/projects');
    const breadcrumbNav = page.getByRole('navigation', { name: 'Breadcrumb' });
    await expect(breadcrumbNav).toBeVisible();
    await expect(breadcrumbNav.getByText('Projects')).toBeVisible();
  });

  test('articles page shows breadcrumb with "Articles"', async ({ page }) => {
    await page.goto('/articles');
    const breadcrumbNav = page.getByRole('navigation', { name: 'Breadcrumb' });
    await expect(breadcrumbNav).toBeVisible();
    await expect(breadcrumbNav.getByText('Articles')).toBeVisible();
  });
});
