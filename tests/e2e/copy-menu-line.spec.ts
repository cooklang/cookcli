import { test, expect, Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// The recipe page's "Copy for a Menu" button (#385): a `.menu` bullet for the
// recipe, at the servings or scale the page shows, in the form the menu
// editor's recipe picker writes.

test.use({ permissions: ['clipboard-read', 'clipboard-write'] });

async function copyMenuLine(page: Page): Promise<string> {
  await page.getByRole('button', { name: 'Copy for a Menu' }).click();
  await expect(page.getByRole('status').filter({ hasText: 'Menu line copied' })).toBeVisible();
  return page.evaluate(() => navigator.clipboard.readText());
}

test.describe('Copy for a Menu', () => {
  test('counts servings when the recipe declares them', async ({ page }) => {
    await page.goto('/recipe/Risotto');
    expect(await copyMenuLine(page)).toBe('- @./Risotto{4%servings}');
  });

  test('follows the servings stepper', async ({ page }) => {
    await page.goto('/recipe/Risotto');
    await page.getByRole('button', { name: /increase servings/i }).click();
    await expect(page).toHaveURL(/servings=4\.5/);
    expect(await copyMenuLine(page)).toBe('- @./Risotto{4.5%servings}');
  });

  test('writes the folder and the factor of a recipe without servings', async ({ page }) => {
    await page.goto('/recipe/Breakfast/Chocolate%20Toast%20Delight');
    expect(await copyMenuLine(page)).toBe('- @./Breakfast/Chocolate Toast Delight{}');

    await page.goto('/recipe/Breakfast/Chocolate%20Toast%20Delight?scale=2');
    expect(await copyMenuLine(page)).toBe('- @./Breakfast/Chocolate Toast Delight{2}');
  });

  test('the confirmation goes away', async ({ page }) => {
    await page.goto('/recipe/Risotto');
    await copyMenuLine(page);
    await expect(page.locator('#copy-menu-line-status')).toHaveText('', { timeout: 5000 });
  });

  test('has no accessibility violations', async ({ page }) => {
    await page.goto('/recipe/Risotto');
    await copyMenuLine(page);
    const results = await new AxeBuilder({ page })
      .include('#copy-menu-line')
      .include('#copy-menu-line-status')
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(results.violations).toEqual([]);
  });
});
