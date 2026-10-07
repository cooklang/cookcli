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
    await editor.getByLabel('Move to aisle').selectOption('milk and dairy');
    await editor.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(card(page, 'milk and dairy').locator('.aisle-ingredient[data-name="e2e leek"]'))
      .toBeVisible();
    await expect(fruit.locator('.aisle-ingredient[data-name="e2e leek"]')).toHaveCount(0);

    page.once('dialog', dialog => dialog.accept());
    await page.locator('.aisle-ingredient[data-name="e2e leek"]').click();
    await page.locator('.aisle-editor').getByRole('button', { name: 'Remove', exact: true }).click();
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
    await row.getByRole('button', { name: 'Add', exact: true }).click();
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

  test('creates an aisle from the shopping list picker', async ({ page }) => {
    await page.route('**/api/shopping_list/items', route => route.fulfill({
      json: [{ path: 'Risotto.cook', name: 'Risotto', scale: 1, included_references: [] }],
    }));
    await page.goto('/shopping-list');

    page.once('dialog', dialog => dialog.accept('E2E drinks'));
    await page.locator('[data-action="assign-aisle"][data-ingredient-name="dry white wine"]')
      .selectOption({ label: '+ New aisle…' });

    const aisle = page.locator('.aisle-section', { hasText: 'E2E drinks' });
    await expect(aisle.locator('.item-name', { hasText: 'dry white wine' })).toBeVisible();
    expect(fs.readFileSync(AISLE_FILE, 'utf8')).toMatch(/\n\[E2E drinks\]\ndry white wine\n$/);
  });

  test('groups several entries under the name chosen', async ({ page, request }) => {
    await request.post('/api/aisles/changes', {
      data: { action: 'add_ingredient', aisle: 'milk and dairy', names: ['e2e salted butter'] },
    });
    await page.goto('/aisles');

    await page.getByRole('button', { name: 'Select', exact: true }).click();
    await page.locator('.aisle-ingredient[data-name="e2e salted butter"]').click();
    await page.locator('.aisle-ingredient[data-name="butter"]').click();
    const bar = page.locator('#aisles-bulk');
    await expect(bar).toContainText('2 selected');
    // Nothing a missing part could print: "undefined", or "false".
    const stray = /undefined|\bfalse\b/;
    await expect(bar).not.toContainText(stray);
    await expect(page.locator('#aisle-list')).not.toContainText(stray);

    // Moving is apart from grouping, and out of the way while grouping.
    const moving = bar.locator('.aisles-move');
    await expect(moving).toBeVisible();
    await expect(moving.getByLabel('Move to aisle')).toBeVisible();
    await bar.getByRole('button', { name: 'Group…' }).click();
    await expect(moving).toHaveCount(0);
    await bar.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(moving).toBeVisible();
    await expect(bar).not.toContainText(stray);

    await bar.getByRole('button', { name: 'Group…' }).click();
    // The shortest main name is offered first.
    await expect(bar.getByRole('radio', { name: 'butter', exact: true })).toBeChecked();
    await bar.getByRole('button', { name: 'Group', exact: true }).click();

    const grouped = page.locator('.aisle-ingredient[data-name="butter"]');
    await expect(grouped).toContainText('e2e salted butter');
    await expect(page.locator('.aisle-ingredient[data-name="e2e salted butter"]')).toHaveCount(0);
    expect(fs.readFileSync(AISLE_FILE, 'utf8')).toContain('\nbutter | e2e salted butter\n');
  });

  test('makes another name the main one', async ({ page }) => {
    await page.goto('/aisles');

    await page.locator('.aisle-ingredient[data-name="avocado"]').click();
    const editor = page.locator('.aisle-editor');
    await editor.getByRole('button', { name: 'Make this the main name: avocados' }).click();
    await editor.getByPlaceholder('Add another name').fill('hass');
    await editor.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(page.locator('.aisle-ingredient[data-name="avocados"]')).toContainText('avocado | hass');
    expect(fs.readFileSync(AISLE_FILE, 'utf8')).toContain('\navocados | avocado | hass\n');
  });

  test('files several uncategorized ingredients at once', async ({ page }) => {
    await page.goto('/aisles#uncategorized');
    await expect(page.locator('.uncategorized-ingredient').first()).toBeVisible();

    // Filtered down to one, then joined to an existing ingredient. A pick
    // the filter hides is left out of what the bar acts on.
    await page.getByRole('checkbox', { name: 'almonds' }).check();
    await page.getByPlaceholder('Filter ingredients').fill('butter');
    await expect(page.locator('.uncategorized-ingredient:not(.hidden)')).toHaveCount(1);
    const bar = page.locator('#uncategorized-bulk');
    await expect(bar).toBeHidden();
    await page.getByLabel('Select all shown').check();
    await expect(bar).toContainText('1 selected');
    await bar.getByRole('combobox', { name: 'Or add as other names of' }).fill('butter');
    await bar.locator('button', { hasText: 'Add' }).last().click();
    await expect(page.locator('.uncategorized-ingredient[data-name="cold butter"]')).toHaveCount(0);
    expect(fs.readFileSync(AISLE_FILE, 'utf8')).toContain('\nbutter | cold butter\n');

    // Two at once, into an aisle made on the spot.
    await page.getByPlaceholder('Filter ingredients').fill('');
    await expect(page.getByRole('checkbox', { name: 'almonds' })).toBeChecked();
    await page.getByRole('checkbox', { name: 'dark chocolate' }).check();
    await expect(bar).toContainText('2 selected');
    page.once('dialog', dialog => dialog.accept('E2E snacks'));
    await bar.getByRole('combobox', { name: 'Choose an aisle…' }).selectOption({ label: '+ New aisle…' });
    await bar.locator('button', { hasText: 'Add' }).first().click();

    await expect(page.locator('.uncategorized-ingredient[data-name="almonds"]')).toHaveCount(0);
    await expect(page.locator('.uncategorized-ingredient[data-name="dark chocolate"]')).toHaveCount(0);
    await expect(bar).toBeHidden();
    expect(fs.readFileSync(AISLE_FILE, 'utf8')).toMatch(/\n\[E2E snacks\]\nalmonds\ndark chocolate\n$/);
  });
});
