import { test, expect, Page } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';

// The seed's pantry file, which the dev server started by Playwright's
// `webServer` reads. Each test changes it through the page and the file is put
// back afterwards.
const PANTRY_FILE = path.resolve(__dirname, '../../seed/config/pantry.conf');

test.describe('Pantry file', () => {
  test.describe.configure({ mode: 'serial' });

  let original: string;

  test.beforeAll(() => {
    original = fs.readFileSync(PANTRY_FILE, 'utf8');
  });

  test.afterEach(() => {
    fs.writeFileSync(PANTRY_FILE, original);
  });

  const head = (page: Page, section: string) =>
    page.locator(`.pantry-section-head[data-section="${section}"]`);

  test('renames a section in place', async ({ page }) => {
    await page.goto('/pantry');

    await head(page, 'garden').getByRole('button', { name: 'Rename section' }).click();
    const input = page.getByRole('textbox', { name: 'Section name' });
    await expect(input).toHaveValue('garden');
    await input.fill('herb garden');
    await input.press('Enter');

    await expect(head(page, 'herb garden')).toHaveCount(1);
    await expect(head(page, 'garden')).toHaveCount(0);
    const written = fs.readFileSync(PANTRY_FILE, 'utf8');
    expect(written).toBe(original.replace('[garden]', '["herb garden"]'));
  });

  test('refuses a name another section has', async ({ page }) => {
    await page.goto('/pantry');

    await head(page, 'garden').getByRole('button', { name: 'Rename section' }).click();
    const input = page.getByRole('textbox', { name: 'Section name' });
    await input.fill('Fridge');
    await page.getByRole('button', { name: 'Save', exact: true }).click();

    await expect(page.locator('#pantry-error-message')).toContainText('fridge');
    expect(fs.readFileSync(PANTRY_FILE, 'utf8')).toBe(original);
  });

  test('closes a rename that keeps the name, without asking the server', async ({ page }) => {
    await page.goto('/pantry');

    await head(page, 'garden').getByRole('button', { name: 'Rename section' }).click();
    const input = page.getByRole('textbox', { name: 'Section name' });
    await input.fill(' garden ');
    await input.press('Enter');

    await expect(input).toHaveCount(0);
    await expect(head(page, 'garden').locator('.pantry-section-title')).toHaveText(/^garden \d+$/);
    await expect(page.locator('#pantry-error-banner')).toBeHidden();
    expect(fs.readFileSync(PANTRY_FILE, 'utf8')).toBe(original);
  });

  const row = (page: Page, section: string, name: string) =>
    page.locator(`.pantry-row[data-section="${section}"][data-name="${name}"]`);

  test('writes a date picked in the item editor', async ({ page }) => {
    await page.goto('/pantry');

    await row(page, 'fridge', 'milk').getByRole('button', { name: /Edit item/ }).click();
    const editor = page.locator('.pantry-editor');
    await editor.getByLabel('Expiry Date').fill('2026-11-02');
    await editor.getByRole('button', { name: 'Save' }).click();

    await expect(editor).toHaveCount(0);
    await expect(row(page, 'fridge', 'milk').locator('.pantry-meta')).toContainText('2026-11-02');
    expect(fs.readFileSync(PANTRY_FILE, 'utf8')).toContain('milk = { quantity = "2%l", expire = "2026-11-02" }');
  });

  test('shows a date in the picker and keeps its spelling unless it changes', async ({ page }) => {
    fs.writeFileSync(PANTRY_FILE, original.replace('bought = "2026-03-07"', 'bought = "07.03.2026"'));
    await page.goto('/pantry');

    const eggs = row(page, 'fridge', 'eggs');
    await eggs.getByRole('button', { name: /Edit item/ }).click();
    const editor = page.locator('.pantry-editor');
    await expect(editor.getByLabel('Bought Date')).toHaveValue('2026-03-07');
    await editor.getByLabel('Quantity').fill('10');
    await editor.getByLabel('Quantity').press('Enter');

    await expect(eggs.locator('.pantry-quantity')).toHaveText('10');
    const written = fs.readFileSync(PANTRY_FILE, 'utf8');
    expect(written).toContain('eggs = { quantity = "10", bought = "07.03.2026" }');
  });

  test('adds an item from its section, and keeps the field for the next', async ({ page }) => {
    await page.goto('/pantry');

    const add = page.locator('.pantry-add[data-section="fridge"]');
    await add.getByLabel('Item Name').fill('cream');
    await add.getByLabel('Quantity').fill('200%ml');
    await add.getByRole('button', { name: 'Add' }).click();

    await expect(row(page, 'fridge', 'cream').locator('.pantry-quantity')).toHaveText('200 ml');
    await expect(page.locator('.pantry-add[data-section="fridge"]').getByLabel('Item Name')).toBeFocused();
    expect(fs.readFileSync(PANTRY_FILE, 'utf8')).toMatch(/\[fridge\][^[]*cream = "200%ml"/);
  });

  test('refuses an item the section already has, before asking the server', async ({ page }) => {
    await page.goto('/pantry');

    const add = page.locator('.pantry-add[data-section="fridge"]');
    await add.getByLabel('Item Name').fill('milk');
    await add.getByRole('button', { name: 'Add' }).click();

    await expect(page.locator('#pantry-error-message')).toHaveText('milk is already in fridge');
    expect(fs.readFileSync(PANTRY_FILE, 'utf8')).toBe(original);
  });

  test('adds an item to a new section, or to one named the same', async ({ page }) => {
    await page.goto('/pantry');

    const form = page.locator('#pantry-new-item');
    await form.getByLabel('Section').fill('cellar');
    await form.getByLabel('Item Name').fill('wine');
    await form.getByRole('button', { name: 'Add Item' }).click();
    await expect(head(page, 'cellar')).toHaveCount(1);
    await expect(row(page, 'cellar', 'wine')).toHaveCount(1);

    // `Fridge` reads as the fridge already there, not a second one.
    await form.getByLabel('Section').fill('Fridge');
    await form.getByLabel('Item Name').fill('yoghurt');
    await form.getByRole('button', { name: 'Add Item' }).click();
    await expect(row(page, 'fridge', 'yoghurt')).toHaveCount(1);
    await expect(head(page, 'Fridge')).toHaveCount(0);

    const written = fs.readFileSync(PANTRY_FILE, 'utf8');
    expect(written).toMatch(/\[cellar\]\s*wine = ""/);
  });

  test('filters the list by stock', async ({ page }) => {
    await page.goto('/pantry');

    await page.locator('.pantry-filter[data-filter="out"]').click();
    await expect(page.locator('.pantry-filter[data-filter="out"]')).toHaveAttribute('aria-pressed', 'true');
    // The seed's tinned tomatoes are at 0.
    await expect(row(page, 'pantry', 'tinned tomatoes')).toBeVisible();
    await expect(row(page, 'fridge', 'milk')).toBeHidden();
    await expect(head(page, 'garden')).toBeHidden();
    await expect(page.locator('#pantry-new-item')).toBeHidden();

    await page.locator('.pantry-filter[data-filter="all"]').click();
    await expect(row(page, 'fridge', 'milk')).toBeVisible();
  });

  test('speaks the page language for its own words', async ({ context, page }) => {
    await context.addCookies([{ name: 'lang', value: 'fr-FR', url: 'http://localhost:9080' }]);
    await page.goto('/pantry');

    await expect(head(page, 'general').locator('.pantry-section-title')).toHaveText(/^Général \d+$/);
    await expect(head(page, 'garden').locator('.pantry-section-title')).toHaveText(/^garden \d+$/);
    await expect(page.locator('.pantry-filter[data-filter="out"]')).toContainText('En rupture de stock');
    await expect(page.getByText('Qté')).toHaveCount(0);
  });

  test('shows the file as text and refuses text that is not a pantry', async ({ page }) => {
    await page.goto('/pantry#text');

    const text = page.locator('#pantry-text');
    await expect(text).toHaveValue(original);

    await text.fill('[fridge]\nmilk = "1%l"\nmilk = "2%l"\n');
    await page.getByRole('button', { name: 'Save file' }).click();
    await expect(page.locator('#pantry-error-message')).toContainText('line 3');
    expect(fs.readFileSync(PANTRY_FILE, 'utf8')).toBe(original);
  });

  test('saves the text, and the items follow', async ({ page }) => {
    await page.goto('/pantry');
    await page.getByRole('tab', { name: 'Text' }).click();

    const text = page.locator('#pantry-text');
    await expect(text).toHaveValue(original);
    const changed = `${original}\n# by hand\n[freezer]\npeas = "1%kg"\n`;
    await text.fill(changed);
    await page.getByRole('button', { name: 'Save file' }).click();
    await expect(page.locator('#pantry-text-status')).toHaveText('Saved');
    expect(fs.readFileSync(PANTRY_FILE, 'utf8')).toBe(changed);

    await page.getByRole('tab', { name: 'Items' }).click();
    await expect(head(page, 'freezer')).toHaveCount(1);
  });
});
