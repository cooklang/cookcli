import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';

// Seed directory used by the dev server started by Playwright's `webServer`.
const SEED_DIR = path.resolve(__dirname, '../../seed');
const RECIPE_FILE = path.join(SEED_DIR, 'Neapolitan Pizza.cook');

test.describe('Recipe editor', () => {
  test('does not autosave when the page is merely opened', async ({ page }) => {
    const before = fs.statSync(RECIPE_FILE);
    const originalContent = fs.readFileSync(RECIPE_FILE, 'utf8');

    const saveRequests: string[] = [];
    page.on('request', request => {
      if (request.method() === 'PUT' && request.url().includes('/api/recipes/')) {
        saveRequests.push(request.url());
      }
    });

    await page.goto('/edit/Neapolitan Pizza.cook');
    await page.waitForLoadState('networkidle');

    // Wait past the autosave debounce so a spurious change would have fired.
    await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
    await page.waitForTimeout(2000);

    await expect(page.locator('#save-status')).toHaveText('');
    expect(saveRequests).toEqual([]);

    const after = fs.statSync(RECIPE_FILE);
    expect(after.mtimeMs).toBe(before.mtimeMs);
    expect(fs.readFileSync(RECIPE_FILE, 'utf8')).toBe(originalContent);
  });
});
