import { test, expect, Page } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';

declare const editorView: any;
declare function hasUnsavedChanges(): boolean;

// Seed directory used by the dev server started by Playwright's `webServer`.
const SEED_DIR = path.resolve(__dirname, '../../seed');
const DIR = path.join(SEED_DIR, 'E2E Rename');
const file = (name: string) => path.join(DIR, name);
const USES = '---\ntitle: Uses Dough\n---\n\nStretch @./E2E Rename/Dough{1}.\n';

test.describe('Renaming from the editor', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeAll(() => {
    fs.rmSync(DIR, { recursive: true, force: true });
    fs.mkdirSync(DIR, { recursive: true });
    fs.writeFileSync(file('Dough.cook'), '---\ntitle: Dough\n---\n\nMix @flour{500%g}.\n');
    fs.writeFileSync(file('Dough.jpg'), 'not really a picture');
    fs.writeFileSync(file('Uses Dough.cook'), USES);
    fs.writeFileSync(file('Taken.cook'), 'Taken.\n');
  });

  test.afterAll(() => {
    fs.rmSync(DIR, { recursive: true, force: true });
  });

  test('renames the file, its picture and the references to it', async ({ page }) => {
    await page.goto('/edit/E2E Rename/Dough.cook');
    await page.getByRole('button', { name: 'Rename', exact: true }).click();

    const dialog = page.getByRole('dialog', { name: 'Rename file' });
    const input = dialog.getByLabel('New name');
    await expect(input).toHaveValue('Dough');
    await expect(dialog.getByRole('button', { name: 'Rename' })).toBeDisabled();

    await input.fill('Pizza Dough');
    await dialog.getByRole('button', { name: 'Rename' }).click();

    await expect(page).toHaveURL(/\/edit\/E2E%20Rename\/Pizza%20Dough\.cook$/);
    expect(fs.existsSync(file('Dough.cook'))).toBe(false);
    expect(fs.readFileSync(file('Pizza Dough.cook'), 'utf8')).toContain('@flour{500%g}');
    expect(fs.existsSync(file('Dough.jpg'))).toBe(false);
    expect(fs.readFileSync(file('Pizza Dough.jpg'), 'utf8')).toBe('not really a picture');
    expect(fs.readFileSync(file('Uses Dough.cook'), 'utf8')).toBe(
      USES.replace('@./E2E Rename/Dough{1}', '@./E2E Rename/Pizza Dough{1}'),
    );
  });

  test('saves what was typed under the old name, and nothing writes that name again', async ({ page }) => {
    const saves: string[] = [];
    page.on('request', request => {
      if (request.method() === 'PUT') saves.push(decodeURIComponent(request.url()));
    });

    await page.goto('/edit/E2E Rename/Pizza Dough.cook');
    await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
    await page.evaluate(() => {
      editorView.dispatch({
        changes: { from: editorView.state.doc.length, insert: '\nRest for ~{1%hour}.\n' },
      });
    });
    // Before the one-second autosave fires.
    await page.getByRole('button', { name: 'Rename', exact: true }).click();
    await page.getByLabel('New name').fill('Dough');
    await page.getByLabel('New name').press('Enter');

    await expect(page).toHaveURL(/\/edit\/E2E%20Rename\/Dough\.cook$/);
    expect(fs.readFileSync(file('Dough.cook'), 'utf8')).toContain('Rest for ~{1%hour}.');
    // Past when the autosave would have fired.
    await page.waitForTimeout(1500);
    expect(fs.existsSync(file('Pizza Dough.cook'))).toBe(false);
    expect(saves.every(url => url.endsWith('/api/recipes/E2E Rename/Pizza Dough.cook'))).toBe(true);
  });

  test('explains a refused name and changes nothing', async ({ page }) => {
    const before = fs.readdirSync(DIR).sort();
    await page.goto('/edit/E2E Rename/Dough.cook');
    await page.getByRole('button', { name: 'Rename', exact: true }).click();

    const dialog = page.getByRole('dialog', { name: 'Rename file' });
    for (const [name, message] of [
      ['../Escaped', 'Invalid name'],
      ['Taken', 'already exists'],
    ]) {
      await dialog.getByLabel('New name').fill(name);
      await dialog.getByRole('button', { name: 'Rename' }).click();
      await expect(dialog.locator('#rename-error')).toContainText(message);
      await expect(page).toHaveURL(/\/edit\/E2E%20Rename\/Dough\.cook$/);
    }
    expect(fs.readdirSync(DIR).sort()).toEqual(before);

    await page.keyboard.press('Escape');
    await expect(dialog).toBeHidden();
  });

  // Another tab still open on the old name used to autosave to it, and the
  // save endpoint created the missing file: the old name came back beside
  // the new one. Saves now ask for an existing file (`If-Match: *`).
  for (const [how, takeAway] of [
    ['renamed', async (page: Page) => {
      await page.getByRole('button', { name: 'Rename', exact: true }).click();
      await page.getByLabel('New name').fill('Race Renamed');
      await page.getByLabel('New name').press('Enter');
      await expect(page).toHaveURL(/\/edit\/E2E%20Rename\/Race%20Renamed\.cook$/);
    }],
    ['deleted', async (page: Page) => {
      const resp = await page.request.delete('/api/recipes/E2E Rename/Race.cook');
      expect(resp.ok()).toBe(true);
    }],
  ] as const) {
    test(`a tab open on a file ${how} in another does not write it back`, async ({ page, context }) => {
      fs.writeFileSync(file('Race.cook'), 'Stir.\n');
      fs.rmSync(file('Race Renamed.cook'), { force: true });
      const other = await context.newPage();
      for (const tab of [page, other]) {
        await tab.goto('/edit/E2E Rename/Race.cook');
        await expect(tab.locator('#editor-container .cm-editor')).toBeVisible();
      }
      const saves: string[] = [];
      other.on('request', request => {
        if (request.method() === 'PUT') saves.push(request.url());
      });

      await takeAway(page);
      await other.evaluate(() => {
        editorView.dispatch({ changes: { from: editorView.state.doc.length, insert: 'Typed here.\n' } });
      });

      await expect(other.locator('#save-status')).toHaveText('Not saved: renamed or deleted elsewhere');
      expect(fs.existsSync(file('Race.cook'))).toBe(false);
      if (how === 'renamed') {
        expect(fs.readFileSync(file('Race Renamed.cook'), 'utf8')).toBe('Stir.\n');
      }

      // No more saves after the first refusal, and the text is still unsaved.
      await other.evaluate(() => {
        editorView.dispatch({ changes: { from: editorView.state.doc.length, insert: 'More.\n' } });
      });
      await other.waitForTimeout(1500);
      expect(saves).toHaveLength(1);
      expect(fs.existsSync(file('Race.cook'))).toBe(false);
      await expect(other.locator('#save-status')).toHaveText('Not saved: renamed or deleted elsewhere');
      expect(await other.evaluate(() => hasUnsavedChanges())).toBe(true);
    });
  }

  test('says so in the language of the page', async ({ page, context }) => {
    await context.addCookies([{ name: 'cook_lang', value: 'fr-FR', url: 'http://localhost:9080' }]);
    fs.writeFileSync(file('Race.cook'), 'Stir.\n');
    await page.goto('/edit/E2E Rename/Race.cook');
    await expect(page.locator('#editor-container .cm-editor')).toBeVisible();

    await page.evaluate(() => {
      editorView.dispatch({ changes: { from: editorView.state.doc.length, insert: 'Typed here.\n' } });
    });
    await expect(page.locator('#save-status')).toHaveText('Enregistré');

    const resp = await page.request.delete('/api/recipes/E2E Rename/Race.cook');
    expect(resp.ok()).toBe(true);
    await page.evaluate(() => {
      editorView.dispatch({ changes: { from: editorView.state.doc.length, insert: 'More.\n' } });
    });
    await expect(page.locator('#save-status')).toHaveText('Non enregistré : renommé ou supprimé ailleurs');
    await expect(page.getByText('Ce fichier a été renommé ou supprimé')).toBeVisible();
    expect(fs.existsSync(file('Race.cook'))).toBe(false);
  });
});
