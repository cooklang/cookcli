import { test, expect, type Page } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';

// Seed directory used by the dev server started by Playwright's `webServer`.
const SEED_DIR = path.resolve(__dirname, '../../seed');
const DIR = path.join(SEED_DIR, 'E2E Unsaved');
const RECIPE = path.join(DIR, 'Soup.cook');
const RECIPE_TEXT = '---\ntitle: Soup\n---\n\nSimmer @water{1%l}.\n';
const EDIT_URL = '/edit/E2E Unsaved/Soup.cook';

// Closes the tab the way a person would, and tells whether the page asked
// first ('beforeunload', then dismissed: the page stays) or just closed.
async function closeTab(page: Page): Promise<string> {
  const outcome = Promise.race([
    page.waitForEvent('dialog').then(async dialog => {
      const type = dialog.type();
      await dialog.dismiss();
      return type;
    }),
    page.waitForEvent('close').then(() => 'closed'),
  ]);
  await page.close({ runBeforeUnload: true });
  return outcome;
}

// Types at the end of the recipe, as a person would: the browser only asks
// on a page that has been interacted with.
async function typeInEditor(page: Page, text: string) {
  await page.locator('#editor-container .cm-content').click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type(text);
}

test.describe('Leaving the editor', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(() => {
    fs.rmSync(DIR, { recursive: true, force: true });
    fs.mkdirSync(DIR, { recursive: true });
    fs.writeFileSync(RECIPE, RECIPE_TEXT);
  });

  test.afterAll(() => {
    fs.rmSync(DIR, { recursive: true, force: true });
  });

  test('Back saves what the autosave has not yet', async ({ page }) => {
    await page.goto(EDIT_URL);
    await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
    await typeInEditor(page, 'Serve hot.');
    // Well inside the autosave's second.
    await page.getByRole('link', { name: 'Back' }).click();

    await expect(page).toHaveURL(/\/recipe\/E2E%20Unsaved\/Soup\.cook$/);
    expect(fs.readFileSync(RECIPE, 'utf8')).toContain('Serve hot.');
  });

  test('Back stays on the page when the save fails', async ({ page }) => {
    await page.route('**/api/recipes/**', route =>
      route.request().method() === 'PUT'
        ? route.fulfill({ status: 500, contentType: 'application/json', body: '{"error":"disk full"}' })
        : route.continue());
    await page.goto(EDIT_URL);
    await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
    await typeInEditor(page, 'Serve hot.');
    await page.getByRole('link', { name: 'Back' }).click();

    await expect(page.locator('#save-status')).toHaveText('Save failed');
    await expect(page).toHaveURL(/\/edit\/E2E%20Unsaved\/Soup\.cook$/);
    expect(fs.readFileSync(RECIPE, 'utf8')).toBe(RECIPE_TEXT);
  });

  test('asks before closing the tab while the last change is not saved', async ({ page }) => {
    // The save never answers, so the change stays unsaved.
    await page.route('**/api/recipes/**', route =>
      route.request().method() === 'PUT' ? new Promise(() => {}) : route.continue());
    await page.goto(EDIT_URL);
    await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
    await typeInEditor(page, 'Serve hot.');

    expect(await closeTab(page)).toBe('beforeunload');
    expect(page.isClosed()).toBe(false);
  });

  test('closes without asking once the change is saved', async ({ page }) => {
    await page.goto(EDIT_URL);
    await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
    await typeInEditor(page, 'Serve hot.');
    await expect(page.locator('#save-status')).toHaveText('Saved');

    expect(await closeTab(page)).toBe('closed');
  });

  test('a rename moves on without asking', async ({ page }) => {
    await page.goto(EDIT_URL);
    await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
    await typeInEditor(page, 'Serve hot.');
    await page.getByRole('button', { name: 'Rename', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Rename file' });
    await dialog.getByLabel('New name').fill('Broth');
    await dialog.getByRole('button', { name: 'Rename' }).click();

    await expect(page).toHaveURL(/\/edit\/E2E%20Unsaved\/Broth\.cook$/);
    expect(fs.readFileSync(path.join(DIR, 'Broth.cook'), 'utf8')).toContain('Serve hot.');
    expect(await closeTab(page)).toBe('closed');
  });
});

test.describe('Leaving a page with something typed', () => {
  test('the pantry closes without asking when nothing was typed', async ({ page }) => {
    await page.goto('/pantry');
    await page.locator('#pantry-new-item input[name="name"]').click();

    expect(await closeTab(page)).toBe('closed');
  });

  test('asks while the new pantry item is typed but not added', async ({ page }) => {
    await page.goto('/pantry');
    const form = page.locator('#pantry-new-item');
    await form.locator('input[name="section"]').click();
    await page.keyboard.type('fridge');
    await form.locator('input[name="name"]').click();
    await page.keyboard.type('E2E unsaved butter');

    expect(await closeTab(page)).toBe('beforeunload');
  });

  test('asks while an add is on its way, and not once it is done', async ({ page }) => {
    let answer: () => void = () => {};
    const answered = new Promise<void>(resolve => { answer = resolve; });
    // Answered by the test, so the seed's pantry.conf is never written.
    await page.route('**/api/pantry/add', async route => {
      await answered;
      await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
    });
    await page.goto('/pantry');
    const form = page.locator('#pantry-new-item');
    await form.locator('input[name="section"]').click();
    await page.keyboard.type('E2E section');
    await form.locator('input[name="name"]').click();
    await page.keyboard.type('E2E unsaved butter');
    const added = page.waitForResponse('**/api/pantry/add');
    await form.getByRole('button').click();

    expect(await closeTab(page)).toBe('beforeunload');
    answer();
    await added;
    expect(await closeTab(page)).toBe('closed');
  });

  test('asks while the pantry Text tab holds an unsaved edit', async ({ page }) => {
    await page.goto('/pantry#text');
    const text = page.locator('#pantry-text');
    await expect(text).toBeVisible();
    await text.click();
    await page.keyboard.press('Control+End');
    await page.keyboard.type('\n# unsaved');

    expect(await closeTab(page)).toBe('beforeunload');
  });

  test('the new file form asks once a name is typed, but lets its own submit through', async ({ page }) => {
    // Answered by the test, so no file is created in the seed.
    await page.route('**/new', route =>
      route.request().method() === 'POST'
        ? route.fulfill({ status: 200, contentType: 'text/html', body: '<p>created</p>' })
        : route.continue());
    await page.goto('/new');
    await page.locator('#filename').click();
    await page.keyboard.type('E2E Unsaved Recipe');

    let asked = false;
    page.on('dialog', dialog => {
      asked = true;
      dialog.dismiss();
    });
    await page.locator('#filename').press('Enter');
    await expect(page.locator('text=created')).toBeVisible();
    expect(asked).toBe(false);
  });

  test("asks while an ingredient's names wait for Save, and not once they are put back", async ({ page }) => {
    await page.goto('/aisles');
    await page.locator('button[data-name="avocado"]').click();
    const editor = page.locator('.aisle-editor');
    await editor.getByRole('button', { name: 'Remove this name: avocados' }).click();

    expect(await closeTab(page)).toBe('beforeunload');

    await editor.getByRole('textbox').first().click();
    await page.keyboard.type('avocados');
    await page.keyboard.press('Enter');
    expect(await closeTab(page)).toBe('closed');
  });

  test('asks about the new file form when leaving with a name typed', async ({ page }) => {
    await page.goto('/new');
    await page.locator('#filename').click();
    await page.keyboard.type('E2E Unsaved Recipe');

    expect(await closeTab(page)).toBe('beforeunload');
  });
});
