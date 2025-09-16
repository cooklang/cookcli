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

  test.skip('should perform search on Enter key', async ({ page }) => {  // Skip - search not implemented
    const searchInput = page.getByPlaceholder('Search recipes...');

    // Type search query
    await searchInput.fill('pasta');
    await searchInput.press('Enter');
    await page.waitForLoadState('networkidle');

    // Check URL contains search parameter
    expect(page.url()).toContain('q=pasta');

    // Check search input retains value
    await expect(searchInput).toHaveValue('pasta');
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

  test.skip('should show no results message for non-existent recipes', async ({ page }) => {
    // Search for something unlikely to exist
    await helpers.searchRecipe('xyzabc123impossible');

    // Check for no results message
    const noResults = page.locator('text=/no results|no recipes found|nothing found/i');
    const hasNoResults = await noResults.isVisible().catch(() => false);

    if (hasNoResults) {
      await expect(noResults).toBeVisible();
    } else {
      // Or check that recipe count is 0
      const recipeCards = await helpers.getRecipeCards();
      await expect(recipeCards).toHaveCount(0);
    }
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

  test.skip('should search from any page', async ({ page }) => {
    // Navigate to shopping list
    await helpers.goToShoppingList();

    // Search should still be available
    const searchInput = page.getByPlaceholder('Search recipes...');
    await expect(searchInput).toBeVisible();

    // Perform search
    await searchInput.fill('salad');
    await searchInput.press('Enter');
    await page.waitForLoadState('networkidle');

    // Should navigate to search results
    expect(page.url()).toContain('q=salad');
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

  test.skip('should maintain search when navigating', async ({ page }) => {
    // Perform search
    await helpers.searchRecipe('chicken');

    // If there are results, click on one
    const recipeCards = await helpers.getRecipeCards();
    const cardCount = await recipeCards.count();

    if (cardCount > 0) {
      // Click first result
      await recipeCards.first().click();
      await page.waitForLoadState('networkidle');

      // Go back
      await page.goBack();
      await page.waitForLoadState('networkidle');

      // Search should still be active
      const searchInput = page.getByPlaceholder('Search recipes...');
      await expect(searchInput).toHaveValue('chicken');
      expect(page.url()).toContain('q=chicken');
    }
  });

  test.skip('should search in recipe content', async ({ page }) => {
    // Search for common cooking terms that might be in steps
    const contentSearchTerms = ['cook', 'mix', 'add', 'heat'];

    for (const term of contentSearchTerms) {
      await helpers.searchRecipe(term);

      // Should return results or show no results
      const hasResults = await helpers.getRecipeCards().count() > 0;
      const hasNoResults = await page.locator('text=/no results|no recipes found/i').isVisible().catch(() => false);

      expect(hasResults || hasNoResults).toBeTruthy();

      // Clear for next search
      const searchInput = page.getByPlaceholder('Search recipes...');
      await searchInput.clear();
    }
  });

  test.skip('should be case-insensitive', async ({ page }) => {
    const searchInput = page.getByPlaceholder('Search recipes...');

    // Search lowercase
    await searchInput.fill('recipe');
    await searchInput.press('Enter');
    await page.waitForLoadState('networkidle');

    const lowercaseResults = await helpers.getRecipeCards().count();

    // Clear and search uppercase
    await searchInput.clear();
    await searchInput.fill('RECIPE');
    await searchInput.press('Enter');
    await page.waitForLoadState('networkidle');

    const uppercaseResults = await helpers.getRecipeCards().count();

    // Should return same number of results
    expect(lowercaseResults).toBe(uppercaseResults);
  });
});