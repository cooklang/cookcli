import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { TestHelpers } from '../fixtures/test-helpers';

// Seed directory used by the dev server started by Playwright's `webServer`.
// Kept in sync with `playwright.config.ts`'s `cwd`/command.
const SEED_DIR = path.resolve(__dirname, '../../seed');
const LIST_FILE = path.join(SEED_DIR, '.shopping-list');
const CHECKED_FILE = path.join(SEED_DIR, '.shopping-checked');

// Every test here rewrites the same two seed files and then waits on the
// server to notice, so they must not run alongside each other.
test.describe.configure({ mode: 'default' });

function backup(file: string): string | null {
  return fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : null;
}

function restore(file: string, content: string | null | undefined) {
  // `undefined` means beforeEach never got as far as taking a backup (e.g. the
  // browser failed to launch) — leave the file alone rather than deleting it.
  if (content === undefined) return;
  if (content === null) {
    if (fs.existsSync(file)) fs.unlinkSync(file);
  } else {
    fs.writeFileSync(file, content);
  }
}

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

// Ticking an item off writes `.shopping-checked`, which the server announces
// to every open page — this one included. That echo used to regenerate the
// whole list and replace every checkbox, often while the user was pressing
// the next one. These pin that a tick, from here or from elsewhere, now
// changes the box and nothing else.
test.describe('Shopping list ticks', () => {
  let originalList: string | null | undefined;
  let originalChecked: string | null | undefined;

  test.beforeEach(async ({ page }) => {
    originalList = backup(LIST_FILE);
    originalChecked = backup(CHECKED_FILE);
    // Recipe paths in the .shopping-list format require a "./" prefix.
    fs.writeFileSync(LIST_FILE, './Breakfast/Easy Pancakes\n');
    fs.writeFileSync(CHECKED_FILE, '');
    await new TestHelpers(page).goToShoppingList();
  });

  test.afterEach(async () => {
    restore(LIST_FILE, originalList);
    restore(CHECKED_FILE, originalChecked);
  });

  test('ticking an item leaves the list in place', async ({ page }) => {
    // `egg` is the aisle common name for the recipe's `eggs`; the checkbox is
    // keyed on the displayed name.
    const egg = page.locator('input[data-ingredient-name="egg"]');
    await expect(egg).toBeVisible({ timeout: 10_000 });
    const box = await egg.elementHandle();

    const ticked = page.waitForResponse(
      (response) => new URL(response.url()).pathname === '/api/shopping_list/check',
    );
    await egg.check();
    await ticked;

    // The server announces the tick back after its 200 ms debounce and the
    // page waits another 100 ms before acting on it; this is well past both.
    await page.waitForTimeout(1_500);

    expect(await box!.evaluate((element) => element.isConnected)).toBe(true);
    await expect(egg).toBeChecked();
    await expect(page.locator('li', { has: egg }).locator('.item-name')).toHaveCSS(
      'text-decoration-line',
      'line-through',
    );
  });

  test('a tick made elsewhere is shown in place', async ({ page }) => {
    const flour = page.locator('input[data-ingredient-name="flour"]');
    await expect(flour).toBeVisible({ timeout: 10_000 });
    const box = await flour.elementHandle();

    // Another device ticking flour off, as far as this page can tell.
    fs.appendFileSync(CHECKED_FILE, '+ flour\n');
    await expect(flour).toBeChecked({ timeout: 10_000 });
    expect(await box!.evaluate((element) => element.isConnected)).toBe(true);

    // And unticking it again.
    fs.appendFileSync(CHECKED_FILE, '- flour\n');
    await expect(flour).not.toBeChecked({ timeout: 10_000 });
    expect(await box!.evaluate((element) => element.isConnected)).toBe(true);
  });
});
