import { test, expect } from '@playwright/test';

// Top-level `let` in templates/edit.html: a global binding, not a window property.
declare const editorView: any;

// The page sends its text to the language server 150 ms after the last
// keystroke, but asks for completions at once. Typed without a pause, `@sal`
// used to be completed against the text from before the `@`, which offers
// nothing: the request now sends the pending text first.

test.describe('Editor completion while typing fast', () => {
  // One word: the name ends up in the document URI the page sends.
  const name = `E2E-Completion-${Date.now()}`;

  test.beforeEach(async ({ request }) => {
    const resp = await request.put(`/api/recipes/${encodeURIComponent(name)}.cook`, {
      headers: { 'Content-Type': 'text/plain' },
      data: '---\ntitle: Completion\n---\n\nSeason with @salt{} and @pepper{}.\n',
    });
    expect(resp.ok()).toBe(true);
  });

  test.afterEach(async ({ request }) => {
    await request.delete(`/api/recipes/${encodeURIComponent(name)}.cook`);
  });

  test('suggests the ingredient for text typed without a pause', async ({ page }) => {
    await page.goto(`/edit/${encodeURIComponent(name)}.cook`);
    await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
    // Shown once the socket opens; completion waits for the handshake after it.
    await expect(page.locator('#lsp-text')).toHaveText('LSP Connected');
    await page.waitForFunction('lspInitialized === true');
    await page.evaluate(() => {
      const end = editorView.state.doc.length;
      editorView.dispatch({ changes: { from: end, insert: '\n' }, selection: { anchor: end + 1 } });
      editorView.focus();
    });
    await page.keyboard.type('@sal');

    await expect(page.locator('.cm-tooltip-autocomplete li', { hasText: 'salt' }).first()).toBeVisible();
  });
});
