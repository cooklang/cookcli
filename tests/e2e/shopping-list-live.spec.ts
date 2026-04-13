import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { TestHelpers } from '../fixtures/test-helpers';

// Seed directory used by the dev server started by Playwright's `webServer`.
// Kept in sync with `playwright.config.ts`'s `cwd`/command.
const SEED_DIR = path.resolve(__dirname, '../../seed');
const LIST_FILE = path.join(SEED_DIR, '.shopping-list');

test.describe('Shopping list live updates', () => {
  let originalContent: string | null;

  test.beforeEach(async ({ page }) => {
    originalContent = fs.existsSync(LIST_FILE)
      ? fs.readFileSync(LIST_FILE, 'utf8')
      : null;
    // Start from an empty list so assertions are deterministic.
    fs.writeFileSync(LIST_FILE, '');
    const helpers = new TestHelpers(page);
    await helpers.goToShoppingList();
  });

  test.afterEach(async () => {
    if (originalContent === null) {
      if (fs.existsSync(LIST_FILE)) fs.unlinkSync(LIST_FILE);
    } else {
      fs.writeFileSync(LIST_FILE, originalContent);
    }
  });

  test('updates the sidebar when .shopping-list changes on disk', async ({ page }) => {
    // Baseline: empty state visible.
    // The "shopping-no-recipes" i18n key resolves to:
    // "No recipes selected. Add recipes from the recipe page."
    // The JS renders this text asynchronously after the API call, so wait.
    await expect(page.locator('#selected-recipes').getByText(/no recipes/i)).toBeVisible({ timeout: 5_000 });

    // Out-of-band write: add a seed recipe.
    // Recipe paths in the .shopping-list format require a "./" prefix.
    fs.writeFileSync(LIST_FILE, './Breakfast/Easy Pancakes\n');

    // The selected-recipes sidebar should pick it up via SSE + re-fetch.
    // The watcher has a 200ms debounce; allow generous headroom.
    await expect(
      page.locator('#selected-recipes').getByText(/Easy Pancakes/i),
    ).toBeVisible({ timeout: 10_000 });

    // Remove it out-of-band → back to empty.
    fs.writeFileSync(LIST_FILE, '');
    await expect(
      page.locator('#selected-recipes').getByText(/no recipes/i),
    ).toBeVisible({ timeout: 10_000 });
  });
});
