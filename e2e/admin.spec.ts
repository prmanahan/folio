import { test, expect, type Page } from '@playwright/test';

const ADMIN_PASSWORD = 'changeme';

/**
 * Login via API and set token in localStorage, then navigate.
 * This avoids UI login dependencies for tests that just need authenticated state.
 */
async function loginViaApi(page: Page, goto = '/admin') {
  const resp = await page.request.post('/api/admin/login', {
    data: { password: ADMIN_PASSWORD },
    headers: { 'Content-Type': 'application/json' },
  });
  expect(resp.ok()).toBeTruthy();
  const { token, expires_at } = await resp.json();

  // Navigate first so we have the right origin, then set localStorage
  await page.goto('/admin/login');
  await page.evaluate(({ token, expires_at }) => {
    localStorage.setItem('admin_token', token);
    localStorage.setItem('admin_token_expires', expires_at);
  }, { token, expires_at });

  await page.goto(goto);
}

test.describe('Admin — authentication', () => {
  test('login page loads', async ({ page }) => {
    await page.goto('/admin/login');
    await expect(page.getByRole('heading', { name: /admin login/i })).toBeVisible();
    await expect(page.getByPlaceholder(/password/i)).toBeVisible();
    await expect(page.getByRole('button', { name: /login/i })).toBeVisible();
  });

  test('login with correct password succeeds and redirects to dashboard', async ({ page }) => {
    await page.goto('/admin/login');
    await page.getByPlaceholder(/password/i).fill(ADMIN_PASSWORD);
    await page.getByRole('button', { name: /login/i }).click();
    await expect(page).toHaveURL('/admin', { timeout: 10000 });
    await expect(page.getByRole('heading', { name: /dashboard/i })).toBeVisible({ timeout: 10000 });
  });

  test('login with wrong password shows error', async ({ page }) => {
    await page.goto('/admin/login');
    await page.getByPlaceholder(/password/i).fill('wrongpassword');
    await page.getByRole('button', { name: /login/i }).click();
    // Should stay on login page and show an error
    await expect(page).toHaveURL('/admin/login');
    // Error alert should appear
    const errorEl = page.locator('.alert-error');
    await expect(errorEl).toBeVisible({ timeout: 5000 });
  });

  test('accessing /admin without auth redirects to login', async ({ page }) => {
    // Navigate without setting any token
    await page.goto('/admin');
    // The layout should redirect to /admin/login
    await expect(page).toHaveURL(/\/admin\/login/, { timeout: 10000 });
  });

  test('logout works', async ({ page }) => {
    await loginViaApi(page, '/admin');
    await expect(page.getByRole('heading', { name: /dashboard/i })).toBeVisible({ timeout: 10000 });
    await page.getByRole('button', { name: /logout/i }).click();
    await expect(page).toHaveURL(/\/admin\/login/, { timeout: 10000 });
  });
});

test.describe('Admin — dashboard', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaApi(page, '/admin');
    await expect(page.getByRole('heading', { name: /dashboard/i })).toBeVisible({ timeout: 10000 });
  });

  test('dashboard shows content count cards', async ({ page }) => {
    // Wait for counts to load (spinner disappears)
    await expect(page.locator('.loading-spinner')).toHaveCount(0, { timeout: 10000 });
    // Should have multiple stat cards
    const cards = page.locator('.card');
    await expect(cards).not.toHaveCount(0);
  });

  test('can navigate to skills management', async ({ page }) => {
    // The sidebar has a "Skills" nav link — use the sidebar nav specifically
    await page.locator('aside').getByRole('link', { name: 'Skills' }).click();
    await expect(page).toHaveURL('/admin/skills');
  });
});

test.describe('Admin — skills CRUD', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaApi(page, '/admin/skills');
    // Wait for skills list to load
    await expect(page.getByRole('heading', { name: 'Skills' })).toBeVisible({ timeout: 10000 });
    await expect(page.locator('.loading-spinner')).toHaveCount(0, { timeout: 10000 });
  });

  test('skills list page loads', async ({ page }) => {
    await expect(page.getByRole('button', { name: '+ Add New' })).toBeVisible();
    // Should have a table with skills
    await expect(page.locator('table')).toBeVisible();
  });

  test('can create a new skill', async ({ page }) => {
    await page.getByRole('button', { name: '+ Add New' }).click();

    // Form should appear with heading "New Skill"
    await expect(page.getByRole('heading', { name: 'New Skill' })).toBeVisible({ timeout: 5000 });

    // Fill in the skill name field
    await page.locator('#field-skill-name').fill('E2E Test Skill');
    await page.locator('#field-category').fill('Testing');

    // Save
    await page.getByRole('button', { name: 'Save' }).click();

    // Should return to list and show the new skill
    await expect(page.getByRole('heading', { name: 'Skills' })).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('cell', { name: 'E2E Test Skill' })).toBeVisible({ timeout: 5000 });
  });

  test('can edit a skill', async ({ page }) => {
    // Find the E2E test skill row and click Edit
    const row = page.locator('tr').filter({ hasText: 'E2E Test Skill' }).first();
    await expect(row).toBeVisible({ timeout: 10000 });
    await row.getByRole('button', { name: 'Edit' }).click();

    await expect(page.getByRole('heading', { name: 'Edit Skill' })).toBeVisible({ timeout: 5000 });

    // Modify the name
    await page.locator('#field-skill-name').fill('E2E Test Skill Updated');
    await page.getByRole('button', { name: 'Save' }).click();

    // Should return to list with updated name
    await expect(page.getByRole('cell', { name: 'E2E Test Skill Updated' })).toBeVisible({ timeout: 10000 });
  });

  test('can delete a skill', async ({ page }) => {
    // Find the updated test skill row and click Delete
    const row = page.locator('tr').filter({ hasText: 'E2E Test Skill Updated' }).first();
    await expect(row).toBeVisible({ timeout: 10000 });

    // The delete button uses window.confirm — accept it
    page.on('dialog', dialog => dialog.accept());
    await row.getByRole('button', { name: 'Delete' }).click();

    // Skill should no longer appear
    await expect(page.getByRole('cell', { name: 'E2E Test Skill Updated' })).toHaveCount(0, { timeout: 10000 });
  });
});
