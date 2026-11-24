/**
 * Edge tests for the /projects page, merge 2 of #4853 (docs/test-plans/2026-11-18-live-lists-test-plan.md,
 * ids LL-PW-1..5). Spec: docs/specs/2026-11-18-live-lists-and-blog-move.md, R-24.
 *
 * Reads, and why:
 *  - frontend/scripts/projects-fixtures/list-expected.json: the projects document, read as data. It is the mocked
 *    `/api/lists/projects` body and the oracle for the hrefs.
 *  - e2e/fixtures.ts: `mockPublicApi`, the shared mock for the layout's own API calls.
 * The API is mocked (only `**\/api/lists/projects`, never `projects.json`): no backend needed. Runs under
 * `just e2e`; no CI job runs it.
 *
 * LL-PW-2 and LL-PW-4 drive the page's 15 s abort with `page.clock`: the clock is installed before navigation at a
 * fixed instant T0, and once the page has requested the route it is paused at T0 + 10 s, then advanced by an
 * explicit amount. The page's timer is armed at or after T0, so at T0 + 14 s it has not fired and at T0 + 25 s it
 * has, whatever the real elapsed time. This relies on the page using setTimeout + AbortController (the plan's
 * LL-PW-4 timer precondition, met by the build); no real 16 s wait fallback is used.
 */
import { test, expect, type Page, type Route } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { mockPublicApi } from './fixtures';

type ListFile = { fetched_at: string; groups: { owner: string; repos: { name: string; description: string }[] }[] };

const EXPECTED: ListFile = JSON.parse(
  readFileSync(resolve(__dirname, '../frontend/scripts/projects-fixtures/list-expected.json'), 'utf8'),
);

const ROUTE = '**/api/lists/projects';
const UNAVAILABLE_LINE = 'The project list is temporarily unavailable; try again shortly.';
const NO_PROJECTS = 'No projects yet.';
const T0 = new Date('2026-06-01T12:00:00.000Z');

/** All repo links the list renders, by owner prefix. */
function repoLinks(page: Page) {
  return page.locator(EXPECTED.groups.map((g) => `a[href^="https://github.com/${g.owner}/"]`).join(', '));
}

/** Hold `/api/lists/projects`: the request is recorded and never answered until the test says so. */
async function holdRoute(page: Page): Promise<{ held: () => Route | undefined }> {
  let held: Route | undefined;
  await page.route(ROUTE, (route) => {
    held = route;
  });
  return { held: () => held };
}

test.describe('/projects reads /api/lists/projects (R-24)', () => {
  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
  });

  test('LL-PW-1 loaded list: one link per repo, from the lists API and never projects.json', async ({ page }) => {
    // Given the route mocked to 200 with the fixture document.
    await page.route(ROUTE, (route) => route.fulfill({ json: EXPECTED }));
    const requested: string[] = [];
    page.on('request', (req) => requested.push(req.url()));

    // When /projects loads.
    await page.goto('/projects');

    // Then one link per repo with its exact github.com href, and nothing extra.
    const total = EXPECTED.groups.reduce((n, g) => n + g.repos.length, 0);
    await expect(repoLinks(page)).toHaveCount(total, { timeout: 10000 });
    for (const g of EXPECTED.groups) {
      for (const r of g.repos) {
        const link = page.locator(`a[href="https://github.com/${g.owner}/${r.name}"]`);
        await expect(link).toHaveCount(1);
        await expect(link).toHaveText(r.name);
        await expect(link).toHaveAttribute('target', '_blank');
        await expect(link).toHaveAttribute('rel', 'noopener noreferrer');
      }
    }
    await expect(page.getByText(NO_PROJECTS)).toHaveCount(0);
    await expect(page.getByText(UNAVAILABLE_LINE)).toHaveCount(0);
    // And the page asked the lists API, and never asked for projects.json.
    expect(requested.some((u) => new URL(u).pathname === '/api/lists/projects')).toBe(true);
    expect(requested.filter((u) => new URL(u).pathname.endsWith('projects.json'))).toEqual([]);
  });

  test('LL-PW-2 pending: only the heading shows while the request is outstanding', async ({ page }) => {
    // Given the route held and the page clock installed before navigation.
    await page.clock.install({ time: T0 });
    const { held } = await holdRoute(page);
    const requestMade = page.waitForRequest((req) => new URL(req.url()).pathname === '/api/lists/projects');

    // When /projects loads, and the request has been made and the heading is visible.
    await page.goto('/projects');
    await requestMade;
    await expect(page.getByRole('heading', { name: 'Projects', level: 1 })).toBeVisible();

    // Then, with the clock advanced to 14 s after the page's timer was armed (still under 15 s), the page shows
    // neither "No projects yet." nor the unavailable line.
    await page.clock.pauseAt(new Date(T0.getTime() + 10_000));
    await page.clock.runFor(4_000);
    await expect(page.getByText(NO_PROJECTS)).toHaveCount(0);
    await expect(page.getByText(UNAVAILABLE_LINE)).toHaveCount(0);
    await expect(repoLinks(page)).toHaveCount(0);

    // Release the held request for a clean teardown.
    await held()?.abort().catch(() => undefined);
  });

  test('LL-PW-3 a 503 shows the unavailable line and the two owner links', async ({ page }) => {
    // Given the route mocked to 503 with the API's own error body.
    await page.route(ROUTE, (route) =>
      route.fulfill({ status: 503, json: { error: 'Projects list unavailable' } }),
    );

    // When /projects loads.
    await page.goto('/projects');

    // Then no repo links, exactly the line in an element with role=alert, and the two GitHub owner links.
    const alert = page.getByRole('alert').filter({ hasText: UNAVAILABLE_LINE });
    await expect(alert).toHaveCount(1, { timeout: 10000 });
    await expect(alert).toHaveText(UNAVAILABLE_LINE);
    await expect(repoLinks(page)).toHaveCount(0);
    await expect(page.getByText(NO_PROJECTS)).toHaveCount(0);

    const mnemra = page.locator('a[href="https://github.com/mnemra"]');
    const peter = page.locator('a[href="https://github.com/prmanahan"]');
    await expect(mnemra).toHaveCount(1);
    await expect(peter).toHaveCount(1);
    for (const link of [mnemra, peter]) {
      await expect(link).toHaveAttribute('target', '_blank');
      await expect(link).toHaveAttribute('rel', 'noopener noreferrer');
    }
    await expect(mnemra).toContainText('mnemra');
    await expect(peter).toContainText('prmanahan');
    expect(await mnemra.innerText()).not.toBe(await peter.innerText());
  });

  test('LL-PW-4 unsettled after 15 s: the request is aborted and a late response is never applied', async ({
    page,
  }) => {
    // Given the route held and the page clock installed before navigation (no real 15 s wait, no sleep).
    await page.clock.install({ time: T0 });
    const { held } = await holdRoute(page);
    const failed: string[] = [];
    page.on('requestfailed', (req) => failed.push(new URL(req.url()).pathname));
    const requestMade = page.waitForRequest((req) => new URL(req.url()).pathname === '/api/lists/projects');
    await page.goto('/projects');
    await requestMade;
    await expect(page.getByRole('heading', { name: 'Projects', level: 1 })).toBeVisible();

    // When the clock is advanced well past 15 s after the timer was armed.
    await page.clock.pauseAt(new Date(T0.getTime() + 10_000));
    await page.clock.runFor(15_000);

    // Then the request was aborted and the unavailable line shows.
    await expect.poll(() => failed.includes('/api/lists/projects')).toBe(true);
    await expect(page.getByText(UNAVAILABLE_LINE)).toHaveCount(1);

    // And a late valid response, if the route can still be fulfilled at all, is never applied. A fulfil that
    // throws because the request is already gone is the expected outcome.
    const route = held();
    expect(route).toBeDefined();
    await route!.fulfill({ json: EXPECTED }).catch(() => undefined);
    await page.clock.runFor(1_000);
    await expect(page.getByText(UNAVAILABLE_LINE)).toHaveCount(1);
    await expect(repoLinks(page)).toHaveCount(0);
    await expect(page.getByText(NO_PROJECTS)).toHaveCount(0);
  });

  test('LL-PW-5 a hostile description renders as text, not markup', async ({ page }) => {
    // Given the route mocked to 200 with one repo whose description is markup.
    const hostile = '<img src=x onerror=alert(1)> **b**';
    const doc: ListFile = JSON.parse(JSON.stringify(EXPECTED));
    doc.groups[0].repos[0].description = hostile;
    await page.route(ROUTE, (route) => route.fulfill({ json: doc }));

    // When /projects loads.
    await page.goto('/projects');

    // Then the description is present verbatim as text and there is no img or strong in the list region.
    const description = page.getByText(hostile, { exact: true });
    await expect(description).toHaveCount(1, { timeout: 10000 });
    const region = page.locator('ul', { has: description });
    await expect(region).toHaveCount(1);
    await expect(region.locator('img')).toHaveCount(0);
    await expect(region.locator('strong')).toHaveCount(0);
  });
});
