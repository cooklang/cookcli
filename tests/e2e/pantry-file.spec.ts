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
    await expect(head(page, 'garden').locator('.pantry-section-title')).toHaveText('garden');
    await expect(page.locator('#pantry-error-banner')).toBeHidden();
    expect(fs.readFileSync(PANTRY_FILE, 'utf8')).toBe(original);
  });

  test('writes a date picked in the Add item dialog', async ({ page }) => {
    await page.goto('/pantry');

    await page.getByRole('button', { name: 'Add Item' }).click();
    await page.locator('#add-section').selectOption('fridge');
    await page.locator('#add-name').fill('cream');
    await page.locator('#add-expire').fill('2026-11-02');
    await page.locator('#add-form').getByRole('button', { name: 'Save' }).click();

    await expect(page.locator('.pantry-item[data-name="cream"]')).toHaveCount(1);
    expect(fs.readFileSync(PANTRY_FILE, 'utf8')).toMatch(/cream = \{[^}]*expire = "2026-11-02"/);
  });

  test('shows a date in the picker and keeps its spelling unless it changes', async ({ page }) => {
    fs.writeFileSync(PANTRY_FILE, original.replace('bought = "2026-03-07"', 'bought = "07.03.2026"'));
    await page.goto('/pantry');

    const eggs = page.locator('.pantry-item[data-section="fridge"][data-name="eggs"]');
    await eggs.locator('.edit-btn').click();
    await expect(eggs.locator('.edit-bought')).toHaveValue('2026-03-07');
    await eggs.locator('.edit-quantity').fill('10');
    await eggs.locator('.save-btn').click();

    await expect(eggs.locator('.item-quantity')).toHaveText('10');
    const written = fs.readFileSync(PANTRY_FILE, 'utf8');
    expect(written).toContain('eggs = { quantity = "10", bought = "07.03.2026" }');
  });

  test('speaks the page language for its own words', async ({ context, page }) => {
    await context.addCookies([{ name: 'lang', value: 'fr-FR', url: 'http://localhost:9080' }]);
    await page.goto('/pantry');

    await expect(head(page, 'general').locator('.pantry-section-title')).toHaveText('Général');
    await expect(head(page, 'garden').locator('.pantry-section-title')).toHaveText('garden');
    await expect(page.locator('#out-of-stock-count')).toHaveText(/^En rupture de stock : \d+ sur \d+$/);
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
