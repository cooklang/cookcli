import { test, expect, type Page } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';

// A throwaway recipe in its own folder, so nothing here touches a seed recipe.
const SEED_DIR = path.resolve(__dirname, '../../seed');
const RECIPE_DIR = path.join(SEED_DIR, 'E2E Highlight');
const RECIPE_FILE = path.join(RECIPE_DIR, 'Highlight Test.cook');
const EDIT_URL = '/edit/E2E Highlight/Highlight Test.cook';

// One construct per line, each read the way cooklang's lexer reads it.
const LINES = [
  'Stir-fry for ~{1%minute}.',
  'Add @oil. -- careful [- not a block',
  'Then @egg{2}.',
  'Season \\[- not a comment',
  'Add @salt -- or {3}',
  'Add @pepper [- or {more} -] to taste',
];

async function openEditor(page: Page, warnings: string[]) {
  page.on('console', msg => {
    if (msg.type() === 'warning') warnings.push(msg.text());
  });
  await page.goto(EDIT_URL);
  await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
  return (n: number) => page.locator('#editor-container .cm-line').nth(n);
}

test.describe('Recipe editor highlighting', () => {
  // One worker, so no afterAll removes the recipe while another test reads it.
  test.describe.configure({ mode: 'serial' });

  test.beforeAll(() => {
    fs.rmSync(RECIPE_DIR, { recursive: true, force: true });
    fs.mkdirSync(RECIPE_DIR, { recursive: true });
    fs.writeFileSync(RECIPE_FILE, LINES.join('\n') + '\n');
  });

  test.afterAll(() => {
    fs.rmSync(RECIPE_DIR, { recursive: true, force: true });
  });

  test('colours the quantity and unit of a timer without a name', async ({ page }) => {
    const warnings: string[] = [];
    const line = await openEditor(page, warnings);

    await expect(line(0).locator('.cm-cook-timer')).toHaveText('~');
    await expect(line(0).locator('.cm-cook-quantity')).toHaveText('1');
    await expect(line(0).locator('.cm-cook-unit')).toHaveText('minute');
    expect(warnings.filter(w => w.includes('Unknown highlighting tag'))).toEqual([]);
  });

  test('ends a line comment at the end of its line, even with "[-" in it', async ({ page }) => {
    const line = await openEditor(page, []);

    await expect(line(1).locator('.cm-cook-comment')).toHaveText('-- careful [- not a block');
    await expect(line(2).locator('.cm-cook-comment')).toHaveCount(0);
    await expect(line(2).locator('.cm-cook-ingredient')).toHaveText('@egg');
    await expect(line(2).locator('.cm-cook-quantity')).toHaveText('2');
  });

  test('does not open a comment on an escaped "[-"', async ({ page }) => {
    const line = await openEditor(page, []);

    await expect(line(3).locator('.cm-cook-comment')).toHaveCount(0);
  });

  test('stops an ingredient name at a comment', async ({ page }) => {
    const line = await openEditor(page, []);

    await expect(line(4).locator('.cm-cook-ingredient')).toHaveText('@salt');
    await expect(line(4).locator('.cm-cook-comment')).toHaveText('-- or {3}');
    await expect(line(4).locator('.cm-cook-quantity')).toHaveCount(0);

    await expect(line(5).locator('.cm-cook-ingredient')).toHaveText('@pepper');
    await expect(line(5).locator('.cm-cook-comment')).toHaveText('[- or {more} -]');
    await expect(line(5).locator('.cm-cook-quantity')).toHaveCount(0);
  });
});
