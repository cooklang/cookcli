import { test, expect } from '@playwright/test';

// Recipe names are typed in the reader's own language. The form's `pattern`
// used to allow ASCII only, so "Crêpes" never reached the server, and the
// server dropped the combining marks Thai or Hindi words are written with.

test.describe('New recipe names', () => {
  test('the form accepts letters of any script and refuses punctuation', async ({ page }) => {
    await page.goto('/new');
    const field = page.locator('#filename');

    const valid = async (name: string) => {
      await field.fill(name);
      return field.evaluate((input: HTMLInputElement) => input.validity.valid);
    };

    for (const name of [
      'Desserts/Crêpes Suzette',
      'Bœuf bourguignon',
      'Μουσακάς',
      'Борщ',
      '麻婆豆腐',
      'カレーライス',
      '김치찌개',
      'كبسة',
      'שקשוקה',
      'ต้มยำกุ้ง',
      'मक्खन चिकन',
      'قورمه\u200cسبزی',
    ]) {
      expect(await valid(name), name).toBe(true);
    }
    // A pattern the browser cannot compile is ignored, so these also prove
    // the one in the template is valid.
    for (const name of ['Pasta.cook', "Mom's Pie", 'Fig & Honey']) {
      expect(await valid(name), name).toBe(false);
    }
  });

  for (const [script, word] of [
    ['Greek', 'Μουσακάς'],
    ['Japanese', 'カレーライス'],
    ['Thai', 'ต้มยำกุ้ง'],
  ]) {
    test.describe(script, () => {
      const name = `E2E ${word} ${Date.now()}`;

      test.afterEach(async ({ request }) => {
        await request.delete(`/api/recipes/${encodeURIComponent(name)}.cook`);
      });

      test('is created under the name typed and opens in the editor', async ({ page }) => {
        await page.goto('/new');
        await page.getByLabel('Recipe path').fill(name);
        await page.getByRole('button', { name: 'Create Recipe' }).click();

        await expect(page).toHaveURL(new RegExp(`/edit/${encodeURIComponent(name)}\\.cook$`));
        await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
        await expect(page.locator('#editor-container .cm-content')).toContainText(`title: ${name}`);

        await page.goto(`/recipe/${encodeURIComponent(name)}`);
        await expect(page.getByRole('heading', { level: 1 })).toHaveText(name);
      });
    });
  }
});
