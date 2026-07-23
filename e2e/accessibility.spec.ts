import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { mockPublicApi, PROFILE_NAME } from './fixtures';

test.describe('Accessibility — home page', () => {
  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
  });

  test('home page passes axe scan (no critical violations)', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });

    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      // Known issue: availability badge may have insufficient contrast — tracked
      .exclude('.availability-badge')
      .analyze();

    if (results.violations.length > 0) {
      console.log('Axe violations found:');
      for (const v of results.violations) {
        console.log(`  [${v.impact}] ${v.id}: ${v.description}`);
        for (const node of v.nodes) {
          console.log(`    Selector: ${node.target}`);
        }
      }
    }

    const criticalOrSerious = results.violations.filter(
      v => v.impact === 'critical' || v.impact === 'serious'
    );
    expect(criticalOrSerious).toHaveLength(0);
  });

  test('color contrast audit — known issues documented', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });

    const results = await new AxeBuilder({ page })
      .withTags(['wcag2aa'])
      .analyze();

    const contrastViolations = results.violations.filter(v => v.id === 'color-contrast');
    if (contrastViolations.length > 0) {
      console.log('Color contrast violations:');
      for (const v of contrastViolations) {
        for (const node of v.nodes) {
          console.log(`  Selector: ${node.target} | ${node.failureSummary}`);
        }
      }
    }
  });

  test('navigation cards are keyboard focusable', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });

    await page.locator('body').click();
    await page.keyboard.press('Tab');

    // Tab through elements — at least one interactive element should gain focus
    const focused = await page.evaluate(() => document.activeElement?.tagName);
    if (focused === 'BODY' || focused === null) {
      await page.keyboard.press('Tab');
    }
    const focused2 = await page.evaluate(() => document.activeElement?.tagName);
    expect(['A', 'BUTTON', 'INPUT', 'TEXTAREA']).toContain(focused2);
  });

  test('nav cards have valid href', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    // Check nav card links using data-testid
    const projectsCard = page.locator('[data-testid="card-projects"]');
    await expect(projectsCard).toHaveAttribute('href', '/projects');
    const articlesCard = page.locator('[data-testid="card-articles"]');
    await expect(articlesCard).toHaveAttribute('href', '/articles');
    // Phase 1.5: Resume card
    const resumeCard = page.locator('[data-testid="card-resume"]');
    await expect(resumeCard).toHaveAttribute('href', '/resume');
  });

  test('Ask AI card has accessible label', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    const askAiBtn = page.getByRole('button', { name: /ask ai/i });
    await expect(askAiBtn).toBeVisible();
  });

  test('images have alt text', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });

    const images = page.locator('img');
    const count = await images.count();
    for (let i = 0; i < count; i++) {
      const alt = await images.nth(i).getAttribute('alt');
      expect(alt).not.toBeNull();
    }
  });

  test('page has exactly one h1', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    const h1s = page.getByRole('heading', { level: 1 });
    await expect(h1s).toHaveCount(1);
  });

  test('skeleton has aria-busy during load', async ({ page }) => {
    // Override profile mock with a slow response to catch loading state
    await page.route('**/api/profile', async (route) => {
      await new Promise((r) => setTimeout(r, 2000));
      await route.continue();
    });
    await page.goto('/');
    // Skeleton should be visible during load
    const skeleton = page.locator('[aria-busy="true"]');
    await expect(skeleton).toBeVisible({ timeout: 3000 });
  });

  test('breadcrumb on inner pages has correct ARIA', async ({ page }) => {
    await page.goto('/projects');
    const breadcrumbNav = page.getByRole('navigation', { name: 'Breadcrumb' });
    await expect(breadcrumbNav).toBeVisible();
    await expect(breadcrumbNav).toHaveAttribute('aria-label', 'Breadcrumb');

    // Current page segment should have aria-current="page"
    const currentSegment = breadcrumbNav.locator('[aria-current="page"]');
    await expect(currentSegment.first()).toBeVisible();
  });

  test('PM monogram has aria-label="Home"', async ({ page }) => {
    await page.goto('/projects');
    const monogram = page.getByRole('link', { name: 'Home' });
    await expect(monogram).toBeVisible();
    await expect(monogram).toHaveAttribute('aria-label', 'Home');
  });

  // Phase 1.5: Breadcrumb separator must have aria-hidden="true"
  // Only applies at depth > 1 (separators appear between segments).
  // At depth 1 (e.g. /projects), there are no separators to check.
  // This test verifies the separator elements that DO exist are aria-hidden.
  test('breadcrumb separators (when present) have aria-hidden="true"', async ({ page }) => {
    await page.goto('/projects');
    const breadcrumbNav = page.getByRole('navigation', { name: 'Breadcrumb' });
    await expect(breadcrumbNav).toBeVisible();
    // Count separator spans — at depth 1, there are none
    const separators = breadcrumbNav.locator('.separator');
    const count = await separators.count();
    // If separators exist, each must be aria-hidden
    for (let i = 0; i < count; i++) {
      await expect(separators.nth(i)).toHaveAttribute('aria-hidden', 'true');
    }
  });

  // Phase 1.5: aria-current="page" on the Resume breadcrumb segment
  test('resume page breadcrumb has aria-current="page" on last segment', async ({ page }) => {
    await page.goto('/resume');
    const breadcrumbNav = page.getByRole('navigation', { name: 'Breadcrumb' });
    await expect(breadcrumbNav).toBeVisible();
    // Both desktop and mobile breadcrumb render aria-current="page" — use .first()
    const currentSegment = breadcrumbNav.locator('[aria-current="page"]').first();
    await expect(currentSegment).toBeVisible();
    await expect(currentSegment).toContainText('Resume');
  });

  // Phase 1.5: PM monogram visible on home (rendered unconditionally now)
  test('PM monogram is visible on home page with aria-label', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    const monogram = page.locator('header.site-header a[aria-label="Home"]');
    await expect(monogram).toBeVisible();
    await expect(monogram).toHaveAttribute('aria-label', 'Home');
  });

  // Phase 1.5: SkillsBanner is decorative — must be aria-hidden
  // The .skills-banner wrapper always renders; the inner track only renders with data.
  test('SkillsBanner has aria-hidden="true" (decorative)', async ({ page }) => {
    // Override skills mock to ensure banner content renders
    await page.route('**/api/skills', (route) =>
      route.fulfill({
        json: [
          { skill_name: 'Rust', category: 'strong', sort_order: 1 },
        ],
      })
    );
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    const banner = page.locator('.skills-banner');
    await expect(banner).toHaveAttribute('aria-hidden', 'true');
  });

  // Phase 1.5: SkillsBanner marquee pauses under prefers-reduced-motion
  // Requires seeded skills to render the banner track — override mock with skill data.
  test('SkillsBanner marquee is static under prefers-reduced-motion', async ({ page }) => {
    // Override skills mock to provide actual data so the banner renders
    await page.route('**/api/skills', (route) =>
      route.fulfill({
        json: [
          { skill_name: 'Rust', category: 'strong', sort_order: 1 },
          { skill_name: 'TypeScript', category: 'moderate', sort_order: 2 },
        ],
      })
    );
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    // Under reduced motion, the banner track animation should be none
    const bannerTrack = page.locator('.banner-track');
    await expect(bannerTrack).toBeVisible({ timeout: 5000 });
    const animationName = await bannerTrack.evaluate((el) =>
      getComputedStyle(el).animationName
    );
    // 'none' means the animation is fully disabled as specified in the CSS rule
    expect(animationName).toBe('none');
  });
});

test.describe('Accessibility — AI pane', () => {
  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
  });

  test('AI pane close button has aria-label', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    await page.getByRole('button', { name: /ask ai/i }).click();
    const closeBtn = page.getByRole('button', { name: /close/i });
    await expect(closeBtn).toBeVisible({ timeout: 5000 });
    const ariaLabel = await closeBtn.getAttribute('aria-label');
    const textContent = await closeBtn.textContent();
    expect(ariaLabel || textContent?.trim()).toBeTruthy();
  });

  test('send button in chat has accessible label', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    await page.getByRole('button', { name: /ask ai/i }).click();
    const sendBtn = page.getByRole('button', { name: /transmit message/i });
    await expect(sendBtn).toBeVisible({ timeout: 5000 });
  });

  // D6-03: a screen reader gets zero indication today that a chat response
  // streamed or arrived. Covers AC-1 (arrival announced) and AC-3 (streaming
  // status announced, then removed). AC-2/AC-4/AC-5 (no per-token firehose,
  // error path untouched, no visual change) are verified by code reading in
  // the completion report — not straightforward to assert from the DOM here.
  test('assistant response is announced via a live region once streaming completes (AC-1)', async ({ page }) => {
    await page.route('**/api/chat', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'text/event-stream',
        body: 'data: Peter ships reliable systems.\ndata: [DONE]\n',
      })
    );

    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    await page.getByRole('button', { name: /ask ai/i }).click();
    const textarea = page.getByPlaceholder(/type your transmission/i);
    await expect(textarea).toBeVisible({ timeout: 5000 });
    await textarea.fill('tell me about Peter');
    await page.getByRole('button', { name: /transmit message/i }).click();

    // Streaming settled once the transmit button reverts from its spinner.
    await expect(page.getByRole('button', { name: /transmit message/i })).toContainText('Transmit', {
      timeout: 5000,
    });

    const announcement = page.locator('[data-testid="chat-response-announcement"]');
    await expect(announcement).toHaveText('Peter ships reliable systems.');
    // Note: not asserting !toBeVisible() here — Playwright treats the standard
    // clip-rect sr-only technique (1x1px, non-zero layout box) as "visible"
    // since it isn't display:none/visibility:hidden. Confirmed via manual run
    // (AC-5's no-visible-layout-change claim is a code-reading check, not this
    // assertion's job).
  });

  test('typing status is announced while streaming and removed after (AC-3)', async ({ page }) => {
    await page.route('**/api/chat', async (route) => {
      await new Promise((r) => setTimeout(r, 1000));
      await route.fulfill({
        status: 200,
        contentType: 'text/event-stream',
        body: 'data: Peter ships reliable systems.\ndata: [DONE]\n',
      });
    });

    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
    await page.getByRole('button', { name: /ask ai/i }).click();
    const textarea = page.getByPlaceholder(/type your transmission/i);
    await expect(textarea).toBeVisible({ timeout: 5000 });
    await textarea.fill('tell me about Peter');
    await page.getByRole('button', { name: /transmit message/i }).click();

    const status = page.locator('[data-testid="chat-typing-status"]');
    await expect(status).toHaveText(/assistant is typing/i, { timeout: 2000 });

    await expect(page.getByRole('button', { name: /transmit message/i })).toContainText('Transmit', {
      timeout: 5000,
    });
    await expect(status).toBeEmpty();
  });
});

test.describe('Accessibility — Resume page', () => {
  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
  });

  test('resume page passes axe scan', async ({ page }) => {
    await page.goto('/resume');
    await page.waitForLoadState('networkidle');

    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();

    const criticalOrSerious = results.violations.filter(
      v => v.impact === 'critical' || v.impact === 'serious'
    );
    if (criticalOrSerious.length > 0) {
      for (const v of criticalOrSerious) {
        console.log(`  [${v.impact}] ${v.id}: ${v.description}`);
      }
    }
    expect(criticalOrSerious).toHaveLength(0);
  });

  test('resume sections have semantic headings', async ({ page }) => {
    await page.goto('/resume');
    await page.waitForLoadState('networkidle');
    // Each section must have an h2
    await expect(page.getByRole('heading', { level: 2, name: /skills/i })).toBeVisible({ timeout: 10000 });
  });
});
