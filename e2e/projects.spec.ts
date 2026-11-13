/**
 * R-13 through the browser (docs/specs/2026-11-11-projects-from-github.md). Red phase for #4726 T7.
 *
 * Reads, and why:
 *  - frontend/scripts/projects-fixtures/list-expected.json: the expected list file (spec R-8),
 *    served as the route-mocked `/projects.json` and used as the oracle for the hrefs.
 *  - e2e/fixtures.ts: `mockPublicApi`, the shared mock for the layout's own API calls.
 * Mocked like the other public specs: no backend needed. Runs under `just e2e`; no CI job runs it.
 */
import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { mockPublicApi } from './fixtures';

type ListFile = { fetched_at: string; groups: { owner: string; repos: { name: string; description: string }[] }[] };

const EXPECTED: ListFile = JSON.parse(
  readFileSync(resolve(__dirname, '../frontend/scripts/projects-fixtures/list-expected.json'), 'utf8'),
);

test.describe('/projects renders the list file (R-13)', () => {
  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
    await page.route('**/projects.json', (route) => route.fulfill({ json: EXPECTED }));
  });

  test('one link per repo with the exact github.com href, and nothing extra', async ({ page }) => {
    await page.goto('/projects');

    const total = EXPECTED.groups.reduce((n, g) => n + g.repos.length, 0);
    const repoLinks = page.locator(
      EXPECTED.groups.map((g) => `a[href^="https://github.com/${g.owner}/"]`).join(', '),
    );
    await expect(repoLinks).toHaveCount(total, { timeout: 10000 });

    for (const g of EXPECTED.groups) {
      for (const r of g.repos) {
        const link = page.locator(`a[href="https://github.com/${g.owner}/${r.name}"]`);
        await expect(link).toHaveCount(1);
        await expect(link).toHaveText(r.name);
        await expect(link).toHaveAttribute('target', '_blank');
        await expect(link).toHaveAttribute('rel', 'noopener noreferrer');
      }
    }
  });
});
