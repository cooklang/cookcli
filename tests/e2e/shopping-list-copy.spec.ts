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
  // `undefined` means beforeEach never got as far as taking a backup (e.g. the
  // browser failed to launch) — leave the file alone rather than deleting it.
  if (content === undefined) return;
  if (content === null) {
    if (fs.existsSync(file)) fs.unlinkSync(file);
  } else {
    fs.writeFileSync(file, content);
  }
}

test.describe('Copy shopping list to clipboard', () => {
  let originalList: string | null | undefined;
  let originalChecked: string | null | undefined;

  test.beforeEach(async ({ context }) => {
    originalList = backup(LIST_FILE);
    originalChecked = backup(CHECKED_FILE);
    // baseURL is http://localhost, which counts as a secure context, so the
    // async clipboard API is the path exercised here.
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  });

  test.afterEach(async () => {
    restore(LIST_FILE, originalList);
    restore(CHECKED_FILE, originalChecked);
  });

  test('copies the list grouped by aisle, one item per line', async ({ page }) => {
    // Recipe paths in the .shopping-list format require a "./" prefix.
    fs.writeFileSync(LIST_FILE, './Breakfast/Easy Pancakes\n');
    fs.writeFileSync(CHECKED_FILE, '');

    const helpers = new TestHelpers(page);
    await helpers.goToShoppingList();

    const copyButton = page.locator('#copy-list-button');
    await expect(copyButton).toBeVisible({ timeout: 10_000 });
    await copyButton.click();
    await expect(copyButton).toHaveText(/copied/i);

    const text = await page.evaluate(() => navigator.clipboard.readText());
    const lines = text.split('\n');

    // Easy Pancakes contributes eggs, flour, milk, sea salt and butter. Each
    // is a bare line — no bullet, no checkbox — under its aisle heading.
    //
    // Names are the aisle's *common* names, not the ones written in the .cook
    // file: aisle.conf maps `egg | eggs` and `salt | sea salt`, and the copy
    // reuses the same aggregated payload the page renders from.
    expect(text).toMatch(/^egg 3$/m);
    expect(text).toMatch(/^flour 125 g$/m);
    expect(text).toMatch(/^milk 250 ml$/m);
    // Two quantities of the same ingredient stay on one line, ordered by unit
    // name with the unitless component first — so `pinch` precedes `1 tbsp`.
    //
    // This used to expect the other order. `GET /api/shopping-list` served the
    // components straight out of a HashMap, so it was really in a random order
    // per request that happened to come out that way here, and it disagreed
    // with what `cook shopping-list` printed for the same recipe. Both go
    // through the one ordering helper now (#441).
    expect(text).toMatch(/^salt pinch, 1 tbsp$/m);

    // Aisle headings are present and are not themselves item lines. A heading
    // opens the text or follows the blank line that closes the previous group.
    const headings = lines.filter((line, i) => line !== '' && (i === 0 || lines[i - 1] === ''));
    expect(headings).toContain('milk and dairy');
    expect(headings).not.toContain('egg 3');

    // The list is what it starts with — no title line to delete after pasting.
    expect(lines[0]).not.toMatch(/shopping list/i);
  });

  test('leaves out items that are already ticked off', async ({ page }) => {
    fs.writeFileSync(LIST_FILE, './Breakfast/Easy Pancakes\n');
    fs.writeFileSync(CHECKED_FILE, '');

    const helpers = new TestHelpers(page);
    await helpers.goToShoppingList();

    const copyButton = page.locator('#copy-list-button');
    await expect(copyButton).toBeVisible({ timeout: 10_000 });

    // Establish that the item IS in the copy before ticking it off — otherwise
    // the negative assertion below would also pass if the name were simply
    // wrong (which is how an earlier revision of this test fooled itself).
    await copyButton.click();
    await expect(copyButton).toHaveText(/copied/i);
    expect(await page.evaluate(() => navigator.clipboard.readText())).toMatch(/^egg 3$/m);

    // `egg` is the aisle common name for the recipe's `eggs`; the checkbox is
    // keyed on the displayed name.
    const egg = page.locator('input[data-ingredient-name="egg"]');
    await expect(egg).toBeVisible();
    await egg.check();

    await expect(copyButton).toHaveText(/^copy$/i);
    await copyButton.click();
    await expect(copyButton).toHaveText(/copied/i);

    const text = await page.evaluate(() => navigator.clipboard.readText());
    expect(text).not.toMatch(/^egg 3$/m);
    // Unchecked items are untouched.
    expect(text).toMatch(/^flour 125 g$/m);
  });

  test('drops the amounts when the amounts option is off', async ({ page }) => {
    fs.writeFileSync(LIST_FILE, './Breakfast/Easy Pancakes\n');
    fs.writeFileSync(CHECKED_FILE, '');

    const helpers = new TestHelpers(page);
    await helpers.goToShoppingList();

    const copyButton = page.locator('#copy-list-button');
    await expect(copyButton).toBeVisible({ timeout: 10_000 });

    await page.locator('#copy-options-toggle').click();
    await page.locator('#copy-option-amounts').uncheck();
    await copyButton.click();
    await expect(copyButton).toHaveText(/copied/i);

    const text = await page.evaluate(() => navigator.clipboard.readText());
    // Bare names, and no trailing space where the quantity used to sit.
    expect(text).toMatch(/^egg$/m);
    expect(text).toMatch(/^flour$/m);
    expect(text).not.toMatch(/125 g/);
    // Aisle headings are a separate option and stay put.
    expect(text).toMatch(/^milk and dairy$/m);
  });

  test('drops the aisle headings when the aisle option is off', async ({ page }) => {
    fs.writeFileSync(LIST_FILE, './Breakfast/Easy Pancakes\n');
    fs.writeFileSync(CHECKED_FILE, '');

    const helpers = new TestHelpers(page);
    await helpers.goToShoppingList();

    const copyButton = page.locator('#copy-list-button');
    await expect(copyButton).toBeVisible({ timeout: 10_000 });

    await page.locator('#copy-options-toggle').click();
    await page.locator('#copy-option-aisles').uncheck();
    await copyButton.click();
    await expect(copyButton).toHaveText(/copied/i);

    const text = await page.evaluate(() => navigator.clipboard.readText());
    expect(text).not.toMatch(/^milk and dairy$/m);
    // Items keep their amounts, and run together as one block: with no headings
    // left there is nothing for a blank line to separate.
    expect(text).toMatch(/^flour 125 g$/m);
    expect(text.trimEnd()).not.toContain('\n\n');
  });

  test('remembers the copy options across a reload', async ({ page }) => {
    fs.writeFileSync(LIST_FILE, './Breakfast/Easy Pancakes\n');
    fs.writeFileSync(CHECKED_FILE, '');

    const helpers = new TestHelpers(page);
    await helpers.goToShoppingList();

    await expect(page.locator('#copy-list-button')).toBeVisible({ timeout: 10_000 });

    // Both options default on for a browser that has never set them.
    await page.locator('#copy-options-toggle').click();
    await expect(page.locator('#copy-option-aisles')).toBeChecked();
    await expect(page.locator('#copy-option-amounts')).toBeChecked();

    await page.locator('#copy-option-amounts').uncheck();

    await helpers.goToShoppingList();
    await expect(page.locator('#copy-list-button')).toBeVisible({ timeout: 10_000 });
    await page.locator('#copy-options-toggle').click();
    await expect(page.locator('#copy-option-amounts')).not.toBeChecked();
    await expect(page.locator('#copy-option-aisles')).toBeChecked();
  });

  test('hides the copy button when there is nothing to buy', async ({ page }) => {
    fs.writeFileSync(LIST_FILE, '');
    fs.writeFileSync(CHECKED_FILE, '');

    const helpers = new TestHelpers(page);
    await helpers.goToShoppingList();

    await expect(page.locator('#selected-recipes').getByText(/no recipes/i)).toBeVisible({
      timeout: 10_000,
    });
    await expect(page.locator('#copy-list-button')).toBeHidden();
  });
});
