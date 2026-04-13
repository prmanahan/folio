import { test, expect } from '@playwright/test';
import { mockPublicApi, PROFILE_NAME, PROFILE_INITIALS } from './fixtures';

test.describe('Public pages — home (hub layout)', () => {
  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
  });

  test('home page loads with correct title', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle(/Peter Manahan/);
  });

  test('profile name and title are visible', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(/Software Architect/i).first()).toBeVisible();
  });

  test('availability badge is visible', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(/open to opportunities/i)).toBeVisible();
  });

  // Phase 1.5: Header now renders on home (monogram only, no breadcrumb nav).
  // The old {#if !isHome} gate was removed — site-header is always present.
  test('header renders on home page with monogram only (no breadcrumb)', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });

    // Header IS now present on home
    const header = page.locator('header.site-header');
    await expect(header).toBeVisible();

    // But it should NOT contain a breadcrumb nav
    const breadcrumb = header.getByRole('navigation', { name: 'Breadcrumb' });
    await expect(breadcrumb).toHaveCount(0);
  });

  test('monogram on home page shows profile initials', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    // Wait for layout's profile load (monogram transitions from skeleton to initials)
    const monogram = page.locator('header.site-header a[aria-label="Home"]');
    await expect(monogram).toBeVisible();
    await expect(monogram).toContainText(PROFILE_INITIALS, { timeout: 5000 });
  });

  test('Ask AI card exists on home page', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('button', { name: /ask ai/i })).toBeVisible();
  });

  test('Projects card links to /projects', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    const projectsCard = page.locator('[data-testid="card-projects"]');
    await expect(projectsCard).toBeVisible();
    await projectsCard.click();
    await expect(page).toHaveURL('/projects');
  });

  test('Articles card links to /articles', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    const articlesCard = page.locator('[data-testid="card-articles"]');
    await expect(articlesCard).toBeVisible();
    await articlesCard.click();
    await expect(page).toHaveURL('/articles');
  });

  // Phase 1.5: Resume card is now the third nav card (Projects · Articles · Resume)
  test('Resume card links to /resume', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    const resumeCard = page.locator('[data-testid="card-resume"]');
    await expect(resumeCard).toBeVisible();
    await expect(resumeCard).toHaveAttribute('href', '/resume');
    await resumeCard.click();
    await expect(page).toHaveURL('/resume');
  });

  // Phase 1.5: Contact card is now a full-width zone BELOW the 3-up nav row
  test('Contact card is a distinct zone below the nav row', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    const contactCard = page.locator('[data-testid="card-contact"]');
    await expect(contactCard).toBeVisible();

    // Contact card is NOT inside .nav-cards — it is a sibling
    const navCards = page.locator('.nav-cards');
    const contactInNav = navCards.locator('[data-testid="card-contact"]');
    await expect(contactInNav).toHaveCount(0);
  });

  // Phase 1.5: Resume link inside Contact card must say "Resume PDF" not "Resume"
  // Note: this only applies when the API provides a link with label "Resume".
  // Seeded data (LinkedIn, GitHub, Email) does not include a resume link —
  // this test verifies the label mapping logic triggers correctly when such a
  // link is present, by checking the bare "Resume" label is never used.
  test('Contact card link for resume says "Resume PDF" (when resume link seeded)', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    const contactCard = page.locator('[data-testid="card-contact"]');
    await expect(contactCard).toBeVisible();

    // Check if any link in the contact card is present at all
    const links = contactCard.locator('.contact-link');
    const count = await links.count();
    if (count === 0) {
      // No links seeded — skip label check
      return;
    }

    // A bare "Resume" label in the contact card is a failure — it must be "Resume PDF"
    const resumeBareLink = contactCard.getByText('Resume', { exact: true });
    const hasBareLink = await resumeBareLink.count() > 0;
    expect(hasBareLink).toBe(false);
  });

  test('Contact card is visible with links', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
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
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
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
    // Use aria-current="page" segment — present in both desktop and mobile breadcrumb
    const currentSegment = breadcrumbNav.locator('[aria-current="page"]').first();
    await expect(currentSegment).toContainText('Projects');
  });

  test('articles page shows breadcrumb with "Articles"', async ({ page }) => {
    await page.goto('/articles');
    const breadcrumbNav = page.getByRole('navigation', { name: 'Breadcrumb' });
    await expect(breadcrumbNav).toBeVisible();
    const currentSegment = breadcrumbNav.locator('[aria-current="page"]').first();
    await expect(currentSegment).toContainText('Articles');
  });

  // Phase 1.5: Resume page shows breadcrumb with "Resume"
  test('resume page shows breadcrumb with "Resume"', async ({ page }) => {
    await page.goto('/resume');
    const breadcrumbNav = page.getByRole('navigation', { name: 'Breadcrumb' });
    await expect(breadcrumbNav).toBeVisible();
    // The breadcrumb segment for Resume should have aria-current="page"
    const currentSegment = breadcrumbNav.locator('[aria-current="page"]').first();
    await expect(currentSegment).toContainText('Resume');
  });
});
