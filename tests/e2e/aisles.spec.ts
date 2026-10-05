import { test, expect, Page } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';

// The seed's aisle file, which the dev server started by Playwright's
// `webServer` reads. Each test changes it through the page and the file is put
// back afterwards.
const AISLE_FILE = path.resolve(__dirname, '../../seed/config/aisle.conf');

test.describe('Aisles', () => {
  test.describe.configure({ mode: 'serial' });

  let original: string;

  test.beforeAll(() => {
    original = fs.readFileSync(AISLE_FILE, 'utf8');
  });

  test.afterEach(() => {
    fs.writeFileSync(AISLE_FILE, original);
  });

  const card = (page: Page, aisle: string) =>
    page.locator(`.aisle-card[data-aisle="${aisle}"]`);

  test('lists the aisles in file order', async ({ page }) => {
    await page.goto('/aisles');

    await expect(page.locator('h1')).toHaveText('Aisles');
    const names = await page.locator('.aisle-card').evaluateAll(cards =>
      cards.map(card => card.getAttribute('data-aisle')));
    expect(names.slice(0, 2)).toEqual(['fruit and veg', 'milk and dairy']);
    // Other names show beside the first one.
    await expect(card(page, 'fruit and veg').locator('.aisle-ingredient[data-name="avocado"]'))
      .toContainText('avocados');
  });

  test('adds, moves and removes an ingredient, touching only its line', async ({ page }) => {
    await page.goto('/aisles');

    const fruit = card(page, 'fruit and veg');
    await fruit.getByPlaceholder(/spring onion/).fill('e2e leek | e2e leeks');
    await fruit.getByRole('button', { name: 'Add ingredient' }).click();
    await expect(fruit.locator('.aisle-ingredient[data-name="e2e leek"]')).toBeVisible();
    expect(fs.readFileSync(AISLE_FILE, 'utf8')).toContain('\ne2e leek | e2e leeks\n');

    // Open it, move it to another aisle.
    await page.locator('.aisle-ingredient[data-name="e2e leek"]').click();
    const editor = page.locator('.aisle-editor');
    await editor.getByRole('combobox').selectOption('milk and dairy');
    await editor.getByRole('button', { name: 'Save' }).click();
    await expect(card(page, 'milk and dairy').locator('.aisle-ingredient[data-name="e2e leek"]'))
      .toBeVisible();
    await expect(fruit.locator('.aisle-ingredient[data-name="e2e leek"]')).toHaveCount(0);

    page.once('dialog', dialog => dialog.accept());
    await page.locator('.aisle-ingredient[data-name="e2e leek"]').click();
    await page.locator('.aisle-editor').getByRole('button', { name: 'Remove' }).click();
    await expect(page.locator('.aisle-ingredient[data-name="e2e leek"]')).toHaveCount(0);

    // Every line it did not touch, blank ones included, is as it was.
    expect(fs.readFileSync(AISLE_FILE, 'utf8')).toBe(original);
  });

  test('refuses a name some aisle already has', async ({ page }) => {
    await page.goto('/aisles');

    const dairy = card(page, 'milk and dairy');
    await dairy.getByPlaceholder(/spring onion/).fill('Apples');
    await dairy.getByRole('button', { name: 'Add ingredient' }).click();

    await expect(page.locator('#aisles-error-message'))
      .toHaveText('"Apples" is already in the "fruit and veg" aisle');
    expect(fs.readFileSync(AISLE_FILE, 'utf8')).toBe(original);
  });

  test('puts an uncategorized ingredient in an aisle', async ({ page }) => {
    await page.goto('/aisles#uncategorized');

    const row = page.locator('.uncategorized-ingredient[data-name="cold butter"]');
    await expect(row).toBeVisible();
    await expect(row.getByRole('link', { name: 'Risotto.cook' })).toBeVisible();

    await row.getByRole('combobox').selectOption('milk and dairy');
    await row.getByRole('button', { name: 'Add' }).click();
    await expect(row).toHaveCount(0);
    await expect(card(page, 'milk and dairy').locator('.aisle-ingredient[data-name="cold butter"]'))
      .toHaveCount(1);
  });

  test('shows the file as text and refuses text the parser rejects', async ({ page }) => {
    await page.goto('/aisles#text');

    const text = page.locator('#aisles-text');
    await expect(text).toHaveValue(original);

    await text.fill('[a]\nx\n[b]\nx\n');
    await page.getByRole('button', { name: 'Save file' }).click();
    await expect(page.locator('#aisles-error-message')).toContainText('Line 4');
    expect(fs.readFileSync(AISLE_FILE, 'utf8')).toBe(original);
  });

  test('assigns an aisle from the shopping list', async ({ page }) => {
    // The list itself is shared with every other spec running alongside, so
    // this one is given its own; grouping it is still the server's work.
    await page.route('**/api/shopping_list/items', route => route.fulfill({
      json: [{ path: 'Risotto.cook', name: 'Risotto', scale: 1, included_references: [] }],
    }));
    await page.goto('/shopping-list');

    await expect(page.getByRole('link', { name: 'Manage aisles' })).toBeVisible();
    const select = page.locator('[data-action="assign-aisle"][data-ingredient-name="dry white wine"]');
    await select.selectOption('oils and dressings');

    const aisle = page.locator('.aisle-section', { hasText: 'oils and dressings' });
    await expect(aisle.locator('.item-name', { hasText: 'dry white wine' })).toBeVisible();
    await expect(select).toHaveCount(0);
    expect(fs.readFileSync(AISLE_FILE, 'utf8')).toContain('\ndry white wine\n');
  });
});
