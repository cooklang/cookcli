import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { TestHelpers } from '../fixtures/test-helpers';

// A sub-recipe, or a recipe on the shopping list, opens at the size it is made
// at — as a menu's recipes do (#560).

const SEED_DIR = path.resolve(__dirname, '../../seed');
const LIST_FILE = path.join(SEED_DIR, '.shopping-list');
const CHECKED_FILE = path.join(SEED_DIR, '.shopping-checked');

function backup(file: string): string | null {
  return fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : null;
}

function restore(file: string, content: string | null | undefined) {
  if (content === undefined) return;
  if (content === null) {
    if (fs.existsSync(file)) fs.unlinkSync(file);
  } else {
    fs.writeFileSync(file, content);
  }
}

// Caprese serves 2 and drizzles `@./Shared/Vinaigrette{50%ml}`; the
// vinaigrette yields 200 ml, written as 1 serving.
test.describe('Sub-recipes on a recipe page', () => {
  test('open at the share of their yield the recipe asks for', async ({ page }) => {
    await page.goto('/recipe/Salads/Caprese');
    const links = page.locator('a[href*="Shared/Vinaigrette"]');
    await expect(links.first()).toBeVisible();
    // The ingredient list and the step both link to it.
    for (const link of await links.all()) {
      await expect(link).toHaveAttribute('href', /\/recipe\/(\.\/)?Shared\/Vinaigrette\?servings=0\.25$/);
    }
  });

  test('follow the recipe when it is scaled', async ({ page }) => {
    // 4 servings of Caprese drizzle 100 ml: half the vinaigrette.
    await page.goto('/recipe/Salads/Caprese?servings=4');
    const link = page.locator('a[href*="Shared/Vinaigrette"]').first();
    await expect(link).toHaveAttribute('href', /\?servings=0\.5$/);
    await link.click();
    await expect(page).toHaveURL(/\/recipe\/Shared\/Vinaigrette\?servings=0\.5$/);
    await expect(page.locator('#servings')).toHaveValue('0.5');

    // 8 servings drizzle 200 ml, the vinaigrette as written: a plain link.
    await page.goto('/recipe/Salads/Caprese?servings=8');
    await expect(page.locator('a[href*="Shared/Vinaigrette"]').first())
      .toHaveAttribute('href', /Shared\/Vinaigrette$/);
  });
});

test.describe('Recipes on the shopping list', () => {
  // Both tests empty and restore the one list file.
  test.describe.configure({ mode: 'serial' });
  let originalList: string | null | undefined;
  let originalChecked: string | null | undefined;

  test.beforeEach(() => {
    originalList = backup(LIST_FILE);
    originalChecked = backup(CHECKED_FILE);
    fs.writeFileSync(LIST_FILE, '');
    fs.writeFileSync(CHECKED_FILE, '');
  });

  test.afterEach(() => {
    restore(LIST_FILE, originalList);
    restore(CHECKED_FILE, originalChecked);
  });

  test('open at the size they were added at, sub-recipes too', async ({ page }) => {
    const response = await page.request.post('/api/shopping_list/add', {
      data: { path: 'Salads/Caprese.cook', scale: 2, included_references: ['./Shared/Vinaigrette'] },
    });
    expect(response.ok()).toBeTruthy();

    const helpers = new TestHelpers(page);
    await helpers.goToShoppingList();

    const selected = page.locator('#selected-recipes');
    await expect(selected.getByRole('link', { name: /Caprese/ }))
      .toHaveAttribute('href', /\/recipe\/Salads\/Caprese\.cook\?scale=2$/);
    // 100 ml of a 200 ml vinaigrette.
    const vinaigrette = selected.getByRole('link', { name: /Vinaigrette/ });
    await expect(vinaigrette).toHaveAttribute('href', /\/recipe\/(\.\/)?Shared\/Vinaigrette\?scale=0\.5$/);

    await vinaigrette.click();
    await expect(page.locator('#servings')).toHaveValue('0.5');
  });

  test('a recipe at its own size links plainly', async ({ page }) => {
    const response = await page.request.post('/api/shopping_list/add', {
      data: { path: 'Salads/Caprese.cook', scale: 1, included_references: [] },
    });
    expect(response.ok()).toBeTruthy();

    const helpers = new TestHelpers(page);
    await helpers.goToShoppingList();

    await expect(page.locator('#selected-recipes').getByRole('link', { name: /Caprese/ }))
      .toHaveAttribute('href', /\/recipe\/Salads\/Caprese\.cook$/);
  });
});
