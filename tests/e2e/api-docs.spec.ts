import { test, expect } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';

const SECTIONS = [
  'recipes',
  'menus',
  'shopping-list',
  'pantry',
  'search-stats',
  'realtime',
  'sync',
];

test.describe('API documentation page', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
  });

  test('renders the page with its ground rules', async ({ page }) => {
    await helpers.navigateTo('/api-docs');
    await expect(page.getByRole('heading', { name: 'Server API', level: 1 })).toBeVisible();
    await expect(page.getByText('Base URL:')).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Errors' })).toBeVisible();
  });

  test('every section anchor resolves to a section', async ({ page }) => {
    await helpers.navigateTo('/api-docs');
    for (const id of SECTIONS) {
      await expect(page.locator(`#${id}`)).toHaveCount(1);
      await expect(page.locator(`a[href="#${id}"]`)).toHaveCount(1);
    }
  });

  test('documents endpoints with method badges and paths', async ({ page }) => {
    await helpers.navigateTo('/api-docs');
    await expect(page.getByText('/api/shopping_list/items').first()).toBeVisible();
    await expect(page.locator('#pantry').getByText('/api/pantry/:section/:name').first()).toBeVisible();

    // Every verb the API uses should appear as a badge somewhere on the page.
    for (const method of ['GET', 'POST', 'PUT', 'DELETE']) {
      await expect(page.getByText(method, { exact: true }).first()).toBeVisible();
    }
  });

  test('is reachable from the preferences page', async ({ page }) => {
    await helpers.navigateTo('/preferences');
    const link = page.getByRole('link', { name: /Server API/ });
    await expect(link).toBeVisible();
    await link.click();
    await expect(page).toHaveURL(/\/api-docs$/);
    await expect(page.getByRole('heading', { name: 'Server API', level: 1 })).toBeVisible();
  });
});
