/**
 * BM-PW-5 (test plan docs/test-plans/2026-11-18-blog-move-test-plan.md), spec R-27:
 * the admin UI shows no articles surface after the blog move.
 *
 * Needs the backend on :3000 started with ADMIN_PASSWORD equal to the E2E_ADMIN_PASSWORD
 * this run exports. The password is generated per run and never committed. The test FAILS,
 * not skips, when E2E_ADMIN_PASSWORD is unset.
 */
import { test, expect } from '@playwright/test';

test.describe('Blog move — admin UI has no articles surface (R-27)', () => {
  test('dashboard and sidebar show no articles link, label, count or quick action', async ({ page }) => {
    const password = process.env.E2E_ADMIN_PASSWORD;
    expect(password, 'E2E_ADMIN_PASSWORD must be set to the backend ADMIN_PASSWORD').toBeTruthy();

    // Given a logged-in admin session
    const resp = await page.request.post('/api/admin/login', {
      data: { password },
      headers: { 'Content-Type': 'application/json' },
    });
    expect(resp.ok()).toBeTruthy();
    const { token, expires_at } = await resp.json();
    await page.goto('/admin/login');
    await page.evaluate(({ token, expires_at }) => {
      localStorage.setItem('admin_token', token);
      localStorage.setItem('admin_token_expires', expires_at);
    }, { token, expires_at });

    // When the dashboard renders
    await page.goto('/admin');
    await expect(page.getByRole('heading', { name: /dashboard/i })).toBeVisible({ timeout: 10000 });
    // The counts have loaded: a known card is present (control, so absence below is not a blank page).
    await expect(page.getByText('Experience', { exact: true }).first()).toBeVisible({ timeout: 10000 });

    // Then nothing points at, names or counts articles
    await expect(page.locator('a[href^="/admin/articles"]')).toHaveCount(0);
    await expect(page.getByText(/articles/i)).toHaveCount(0);
    await expect(page.getByRole('link', { name: /new article/i })).toHaveCount(0);
  });
});
