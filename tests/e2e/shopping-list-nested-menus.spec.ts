import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';

// Seed directory used by the dev server started by Playwright's `webServer`.
const SEED_DIR = path.resolve(__dirname, '../../seed');
const DIR = path.join(SEED_DIR, 'E2E Ring');
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

// Every amount of `name` in a `POST /api/shopping_list` answer, as text.
function amounts(list: any, name: string): string[] {
  const groups = [...(list.categories ?? []).map((c: any) => c.items), list.other?.items ?? list.other ?? []];
  return groups
    .flat()
    .filter((item: any) => item.name === name)
    .flatMap((item: any) => item.quantities.map((q: any) => `${q.value.value.value} ${q.unit ?? ''}`.trim()));
}

// Three menus in a ring: A → B → C → A. Added to the list, A's meals are
// stored as entries of their own, B among them; B used to be expanded on its
// own, through C and back into A, counting A's oats twice.
test.describe('Menus used as meals of each other', () => {
  let originalList: string | null | undefined;
  let originalChecked: string | null | undefined;

  test.beforeEach(() => {
    originalList = backup(LIST_FILE);
    originalChecked = backup(CHECKED_FILE);
    fs.rmSync(DIR, { recursive: true, force: true });
    fs.mkdirSync(DIR, { recursive: true });
    fs.writeFileSync(path.join(DIR, 'Oats.cook'), 'Soak @ring oats{100%g}.\n');
    fs.writeFileSync(path.join(DIR, 'Ring A.menu'), '- @./E2E Ring/Ring B.menu{} \\\n- @./E2E Ring/Oats{}\n');
    fs.writeFileSync(path.join(DIR, 'Ring B.menu'), '- @./E2E Ring/Ring C.menu{} \\\n- @ring rice{1%cup}\n');
    fs.writeFileSync(path.join(DIR, 'Ring C.menu'), '- @./E2E Ring/Ring A.menu{} \\\n- @ring beans{1%cup}\n');
    fs.writeFileSync(LIST_FILE, '');
    fs.writeFileSync(CHECKED_FILE, '');
  });

  test.afterEach(() => {
    fs.rmSync(DIR, { recursive: true, force: true });
    restore(LIST_FILE, originalList);
    restore(CHECKED_FILE, originalChecked);
  });

  test('count each recipe of the ring once on the shopping list', async ({ page }) => {
    const added = await page.request.post('/api/shopping_list/add_menu', {
      data: { path: 'E2E Ring/Ring A.menu', scale: 1 },
    });
    expect(added.ok()).toBe(true);

    const listed = page.waitForResponse(
      resp => resp.url().endsWith('/api/shopping_list') && resp.request().method() === 'POST',
    );
    await page.goto('/shopping-list');
    const response = await listed;

    // The page sends each meal as part of its menu.
    const sent = response.request().postDataJSON();
    const meals = sent.filter((entry: any) => entry.recipe !== 'E2E Ring/Ring A.menu');
    expect(meals.length).toBeGreaterThan(0);
    for (const meal of meals) {
      expect(meal.menu, JSON.stringify(meal)).toBe('E2E Ring/Ring A.menu');
    }

    const list = await response.json();
    expect(amounts(list, 'ring oats')).toEqual(['100 g']);
    expect(amounts(list, 'ring rice')).toEqual(['1 cup']);
    expect(amounts(list, 'ring beans')).toEqual(['1 cup']);
    await expect(page.locator('input[data-ingredient-name="ring oats"]')).toHaveCount(1);
  });
});
