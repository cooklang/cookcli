import { test, expect, Page } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';

// Assertions read data-name rather than the card heading so these tests do
// not re-create the markup coupling the sorter itself was fixed to avoid.
const recipeNames = (page: Page) =>
  page.locator('#recipes-grid [data-type="recipe"]').evaluateAll((els) =>
    els.map((el) => el.getAttribute('data-name') ?? ''),
  );

const allNames = (page: Page) =>
  page.locator('#recipes-grid > [data-type]').evaluateAll((els) =>
    els.map((el) => `${el.getAttribute('data-type')}:${el.getAttribute('data-name')}`),
  );

test.describe('Recipes index sorting', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    await helpers.navigateTo('/');
    await page.evaluate(() => sessionStorage.removeItem('recipes-sort'));
    await page.reload();
  });

  test('controls are visible and default to name ascending', async ({ page }) => {
    await expect(page.locator('#sort-controls')).toBeVisible();
    await expect(page.locator('#sort-field')).toHaveValue('name');
    await expect(page.locator('#sort-dir')).toHaveText('↑');

    const names = await recipeNames(page);
    expect(names.length).toBeGreaterThan(1);
    const sorted = [...names].sort((a, b) =>
      a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' }),
    );
    expect(names).toEqual(sorted);
  });

  test('direction toggle reverses the order', async ({ page }) => {
    const asc = await recipeNames(page);
    await page.locator('#sort-dir').click();
    await expect(page.locator('#sort-dir')).toHaveText('↓');

    const desc = await recipeNames(page);
    expect(desc).toEqual([...asc].reverse());
  });

  test('sorting by modified date reorders and defaults to newest first', async ({ page }) => {
    const byName = await recipeNames(page);
    await page.locator('#sort-field').selectOption('modified');
    await expect(page.locator('#sort-dir')).toHaveText('↓');

    const newestFirst = await recipeNames(page);
    expect(newestFirst).not.toEqual(byName);
    expect([...newestFirst].sort()).toEqual([...byName].sort());

    const timestamps = await page
      .locator('#recipes-grid [data-type="recipe"]')
      .evaluateAll((els) => els.map((el) => Number(el.getAttribute('data-modified'))));
    const descending = [...timestamps].sort((a, b) => b - a);
    expect(timestamps).toEqual(descending);
  });

  test('directories stay grouped above recipes in both directions', async ({ page }) => {
    for (const _ of [0, 1]) {
      const entries = await allNames(page);
      const lastDir = entries.map((e) => e.startsWith('directory:')).lastIndexOf(true);
      const firstRecipe = entries.findIndex((e) => e.startsWith('recipe:'));
      if (lastDir !== -1 && firstRecipe !== -1) {
        expect(lastDir).toBeLessThan(firstRecipe);
      }
      await page.locator('#sort-dir').click();
    }
  });

  test('sort choice survives a reload', async ({ page }) => {
    await page.locator('#sort-dir').click();
    const before = await recipeNames(page);

    await page.reload();

    await expect(page.locator('#sort-dir')).toHaveText('↓');
    await expect(page.locator('#sort-field')).toHaveValue('name');
    expect(await recipeNames(page)).toEqual(before);
  });

  test('corrupt saved state falls back to defaults instead of throwing', async ({ page }) => {
    await page.evaluate(() => sessionStorage.setItem('recipes-sort', '{not json'));
    await page.reload();

    await expect(page.locator('#sort-controls')).toBeVisible();
    await expect(page.locator('#sort-field')).toHaveValue('name');
    await expect(page.locator('#sort-dir')).toHaveText('↑');
  });

  test('controls stay hidden when there is nothing to sort', async ({ page }) => {
    await helpers.navigateTo('/directory/Salads');
    const recipes = await page.locator('#recipes-grid [data-type="recipe"]').count();
    expect(recipes).toBeLessThan(2);
    await expect(page.locator('#sort-controls')).toBeHidden();
  });

  test('direction toggle is labelled for assistive tech', async ({ page }) => {
    const label = await page.locator('#sort-dir').getAttribute('aria-label');
    expect(label).toBeTruthy();
  });
});
