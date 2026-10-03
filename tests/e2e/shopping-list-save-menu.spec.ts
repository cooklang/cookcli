import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import * as fs from 'node:fs';
import * as path from 'node:path';

// "Save as Menu" on the shopping list (#385, the request from #380): the
// recipes on the list, with their amounts, written to a new .menu.

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

test.describe('Save the shopping list as a menu', () => {
  let originalList: string | null | undefined;
  let originalChecked: string | null | undefined;
  let name: string;

  test.beforeEach(async ({}, testInfo) => {
    originalList = backup(LIST_FILE);
    originalChecked = backup(CHECKED_FILE);
    name = `E2E Saved ${testInfo.workerIndex} ${Date.now()}`;
    fs.writeFileSync(LIST_FILE, './Breakfast/Easy Pancakes{2}\n./Risotto\n');
    fs.writeFileSync(CHECKED_FILE, '');
  });

  test.afterEach(async () => {
    restore(LIST_FILE, originalList);
    restore(CHECKED_FILE, originalChecked);
    const saved = path.join(SEED_DIR, `${name}.menu`);
    if (fs.existsSync(saved)) fs.unlinkSync(saved);
  });

  test('writes the list to a menu and opens it', async ({ page }) => {
    await page.goto('/shopping-list');
    const open = page.getByRole('button', { name: 'Save as Menu' });
    await expect(open).toBeVisible({ timeout: 10_000 });
    await open.click();

    const dialog = page.getByRole('dialog', { name: 'Save the list as a menu' });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByLabel('Menu path')).toBeFocused();
    await dialog.getByLabel('Menu path').fill(name);
    await dialog.getByRole('button', { name: 'Save Menu' }).click();

    await expect(page).toHaveURL(new RegExp(`/recipe/${encodeURIComponent(name)}\\.menu$`));
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(name);
    await expect(page.getByRole('link', { name: /Easy Pancakes/ })).toHaveAttribute('href', /servings=4$/); // it serves 2, so ×2 is 4 servings
    await expect(page.getByRole('link', { name: 'Risotto' })).toBeVisible();
    expect(fs.readFileSync(path.join(SEED_DIR, `${name}.menu`), 'utf8')).toBe(
      `---\ntitle: ${name}\n---\n\n- @./Breakfast/Easy Pancakes{2} \\\n- @./Risotto{}\n`
    );
  });

  test('says so when the name is taken, and Escape puts focus back', async ({ page }) => {
    fs.writeFileSync(path.join(SEED_DIR, `${name}.menu`), 'kept\n');
    await page.goto('/shopping-list');
    const open = page.getByRole('button', { name: 'Save as Menu' });
    await expect(open).toBeVisible({ timeout: 10_000 });
    await open.click();

    const dialog = page.getByRole('dialog', { name: 'Save the list as a menu' });
    await dialog.getByLabel('Menu path').fill(name);
    await dialog.getByRole('button', { name: 'Save Menu' }).click();
    await expect(dialog.getByRole('alert')).toHaveText('A menu with this name already exists');
    await expect(dialog.getByLabel('Menu path')).toHaveAttribute('aria-invalid', 'true');
    expect(fs.readFileSync(path.join(SEED_DIR, `${name}.menu`), 'utf8')).toBe('kept\n');

    const results = await new AxeBuilder({ page })
      .include('#save-menu-dialog')
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(results.violations).toEqual([]);

    await page.keyboard.press('Escape');
    await expect(dialog).toBeHidden();
    await expect(open).toBeFocused();
  });

  test('is not offered for an empty list', async ({ page }) => {
    fs.writeFileSync(LIST_FILE, '');
    await page.goto('/shopping-list');
    await expect(page.getByText(/no recipes/i).first()).toBeVisible({ timeout: 10_000 });
    await expect(page.getByRole('button', { name: 'Save as Menu' })).toBeHidden();
  });
});
