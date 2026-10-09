import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { TestHelpers } from '../fixtures/test-helpers';

// Seed directory used by the dev server started by Playwright's `webServer`.
// Kept in sync with `playwright.config.ts`'s `cwd`/command.
const SEED_DIR = path.resolve(__dirname, '../../seed');
const LIST_FILE = path.join(SEED_DIR, '.shopping-list');
const CHECKED_FILE = path.join(SEED_DIR, '.shopping-checked');

function backup(file: string): string | null {
  return fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : null;
}

function restore(file: string, content: string | null | undefined) {
  // `undefined` means beforeEach never got as far as taking a backup.
  if (content === undefined) return;
  if (content === null) {
    if (fs.existsSync(file)) fs.unlinkSync(file);
  } else {
    fs.writeFileSync(file, content);
  }
}

// The same recipe can be on the list twice — here once with the vinaigrette
// it references and once without — and Remove on the second copy used to take
// the first, because the page sent only the recipe's path.
test.describe('Removing one of two copies of a recipe', () => {
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

  test('takes the copy that was clicked', async ({ page }) => {
    for (const [scale, refs] of [
      [1, ['./Shared/Vinaigrette.cook']],
      [2, []],
    ] as const) {
      const response = await page.request.post('/api/shopping_list/add', {
        data: { path: 'Salads/Caprese.cook', scale, included_references: refs },
      });
      expect(response.ok()).toBeTruthy();
    }

    const helpers = new TestHelpers(page);
    await helpers.goToShoppingList();

    const selected = page.locator('#selected-recipes');
    const remove = selected.getByRole('button', { name: /Remove/i });
    await expect(remove).toHaveCount(2);

    await remove.nth(1).click();

    await expect(remove).toHaveCount(1);
    await expect(selected).toContainText('×1');
    await expect(selected).not.toContainText('×2');
    await expect(selected.getByRole('link', { name: /Vinaigrette/i })).toBeVisible();
  });
});
