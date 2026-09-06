import { test, expect } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';

test.describe('Search Functionality', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    await helpers.navigateTo('/');
  });

  test('should display search input', async ({ page }) => {
    const searchInput = page.getByPlaceholder('Search recipes...');
    await expect(searchInput).toBeVisible();
    await expect(searchInput).toBeEnabled();
  });

  test('should display search results', async ({ page }) => {
    // Perform search
    await helpers.searchRecipe('recipe');

    // Check that we're on search results page
    const heading = page.locator('h1');
    const headingText = await heading.textContent();

    // Should show search results or filtered recipes
    expect(headingText).toMatch(/Search|Results|Recipes/i);
  });

  test('should clear search', async ({ page }) => {
    const searchInput = page.getByPlaceholder('Search recipes...');

    // Perform search
    await helpers.searchRecipe('pasta');
    await expect(searchInput).toHaveValue('pasta');

    // Clear search
    await searchInput.clear();
    await searchInput.press('Enter');
    await page.waitForLoadState('networkidle');

    // Should return to main recipes page
    const url = page.url();
    expect(url).not.toContain('q=');
    await expect(searchInput).toHaveValue('');
  });

  test('should handle special characters in search', async ({ page }) => {
    const specialQueries = [
      'pasta & sauce',
      'chicken "wings"',
      "grandma's recipe",
      '50% whole wheat',
      'recipe #1'
    ];

    for (const query of specialQueries) {
      await helpers.searchRecipe(query);

      // Check that search completes without error
      await expect(page.locator('h1')).toBeVisible();

      // Clear for next search
      const searchInput = page.getByPlaceholder('Search recipes...');
      await searchInput.clear();
    }
  });

  test('should render inline result names as text, not markup', async ({ page }) => {
    // Fabricate a result whose name is markup; a raw innerHTML interpolation
    // would run the onerror handler and set the marker.
    await page.route('**/api/search?*', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([{ path: 'x', name: '<img src=x onerror=window.__pwned=1>' }]),
      })
    );

    const searchInput = page.getByPlaceholder('Search recipes...');
    await searchInput.fill('pizza');

    const result = page.locator('#search-results a.search-result');
    await expect(result).toHaveCount(1);
    await expect(result).toContainText('<img');
    await expect(result.locator('img')).toHaveCount(0);

    const pwned = await page.evaluate(() => (window as any).__pwned);
    expect(pwned).toBeUndefined();
  });
});
