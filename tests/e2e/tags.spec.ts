import { test, expect } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';

// The seed tags Risotto, Caprese (in Salads/) and Neapolitan Pizza `italian`.
test.describe('Tag pages', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
  });

  test('a tag on a recipe opens every recipe carrying it', async ({ page }) => {
    await helpers.navigateTo('/recipe/Risotto');
    await page.getByRole('link', { name: '#italian', exact: true }).click();

    await expect(page).toHaveURL(/\/tag\/italian$/);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText('#italian');
    const cards = page.locator('#recipes-grid [data-type="recipe"]');
    await expect(cards).toHaveCount(3);
    const caprese = cards.filter({ hasText: 'Caprese' });
    await expect(caprese).toContainText('Salads/');

    await caprese.click();
    await expect(page).toHaveURL(/\/recipe\/Salads\/Caprese$/);
  });

  test('the listing links the index of every tag', async ({ page }) => {
    await helpers.navigateTo('/');
    await page.locator('#all-tags').click();

    await expect(page).toHaveURL(/\/tags$/);
    const italian = page.locator('#tags-list a', { hasText: '#italian' });
    await expect(italian).toContainText('3');
    await italian.click();
    await expect(page).toHaveURL(/\/tag\/italian$/);
  });
});
