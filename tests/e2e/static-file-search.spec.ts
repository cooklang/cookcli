import { test, expect } from '@playwright/test';
import { execFileSync } from 'child_process';
import { mkdtempSync, rmSync } from 'fs';
import { tmpdir } from 'os';
import path from 'path';
import { pathToFileURL } from 'url';

/**
 * Search over a site opened straight from disk.
 *
 * The rest of the suite runs against `cook server`, which takes the /api/search
 * branch and never exercises the static search path. Covers the file:// promise
 * in docs/build.md.
 */
test.describe('Static site search over file://', () => {
  let outDir: string;
  let indexUrl: string;

  test.beforeAll(() => {
    outDir = mkdtempSync(path.join(tmpdir(), 'cookcli-static-'));
    const repoRoot = path.resolve(__dirname, '../..');
    execFileSync(
      path.join(repoRoot, 'target/debug/cook'),
      ['build', 'web', outDir, '--base-path', path.join(repoRoot, 'seed')],
      { cwd: repoRoot },
    );
    indexUrl = pathToFileURL(path.join(outDir, 'index.html')).href;
  });

  test.afterAll(() => {
    rmSync(outDir, { recursive: true, force: true });
  });

  test('returns results for a recipe title', async ({ page }) => {
    const failures: string[] = [];
    page.on('requestfailed', (r) => failures.push(r.url()));

    await page.goto(indexUrl);
    await page.getByPlaceholder('Search recipes...').fill('Risotto');

    // Risotto also matches two menus that list it as an ingredient; the
    // title match scores higher and must come first.
    const results = page.locator('#search-results');
    await expect(results).toBeVisible();
    await expect(results.locator('a').first()).toContainText(
      'Classic Risotto alla Milanese',
    );

    expect(failures, 'no request should be blocked on file://').toEqual([]);
  });

  test('result link opens the recipe page', async ({ page }) => {
    await page.goto(indexUrl);
    await page.getByPlaceholder('Search recipes...').fill('Risotto');
    await page.locator('#search-results a').first().click();

    await expect(page).toHaveURL(/Risotto\.html$/);
    await expect(page.locator('h1')).toContainText('Risotto');
  });

  test('reports no matches for an absent term', async ({ page }) => {
    await page.goto(indexUrl);
    const input = page.getByPlaceholder('Search recipes...');

    // An index that failed to load renders the same empty state, so the
    // absent-term assertion below only means something once we have seen
    // a real match come back.
    await input.fill('Risotto');
    await expect(page.locator('#search-results a').first()).toBeVisible();

    await input.fill('zzzznotarecipe');
    await expect(page.locator('#search-results')).toContainText('No recipes found');
  });
});
