import { test, expect } from '@playwright/test';

// Top-level `let` in templates/edit.html: a global binding, not a window property.
declare const editorView: any;

// Ingredient names are written in the cook's own language. Completion used to
// match ASCII only: it never opened for `@œ` or a Greek or Chinese name. It
// also named the document `file://` + the recipe path, which the language
// server refused as a URL whenever the name held a space, so it listed nothing
// in most recipes.

test.describe('Editor completion', () => {
  const name = `E2E Completion ${Date.now()}`;

  test.beforeEach(async ({ request }) => {
    const source = '---\ntitle: Completion\n---\n\n'
      + 'Stir @crème fraîche{100%g}, @μέλι{1%tbsp}, @豆腐{200%g}, @œufs{2} and @salt{}.\n';
    const resp = await request.put(`/api/recipes/${encodeURIComponent(name)}.cook`, {
      headers: { 'Content-Type': 'text/plain' },
      data: source,
    });
    expect(resp.ok()).toBe(true);
  });

  test.afterEach(async ({ request }) => {
    await request.delete(`/api/recipes/${encodeURIComponent(name)}.cook`);
  });

  for (const [typed, expected] of [
    ['@sal', 'salt'],
    ['@crè', 'crème fraîche'],
    ['@œu', 'œufs'],
    ['@μέ', 'μέλι'],
    ['@豆', '豆腐'],
  ]) {
    test(`suggests ${expected} for ${typed}`, async ({ page }) => {
      await page.goto(`/edit/${encodeURIComponent(name)}.cook`);
      await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
      await page.evaluate(() => {
        const end = editorView.state.doc.length;
        editorView.dispatch({ changes: { from: end, insert: '\n' }, selection: { anchor: end + 1 } });
        editorView.focus();
      });
      // Slower than the page's 150 ms `didChange` debounce: typed faster, each
      // completion request reaches the server before the text that asked it.
      await page.keyboard.type(typed, { delay: 300 });

      const options = page.locator('.cm-tooltip-autocomplete li');
      await expect(options.filter({ hasText: expected }).first()).toBeVisible();
    });
  }
});
