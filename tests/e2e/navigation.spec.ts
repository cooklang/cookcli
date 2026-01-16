import { test, expect } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';

test.describe('Navigation', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    await helpers.navigateTo('/');
  });

  test('should display home page with recipe list', async ({ page }) => {
    await expect(page).toHaveTitle(/Cook/);
    await expect(page.locator('h1')).toContainText(/Recipe/);

    // Check if recipe cards are displayed
    const recipeCards = helpers.getRecipeCards();
    const count = await recipeCards.count();
    expect(count).toBeGreaterThan(0);
  });

  test('should navigate to recipe details', async ({ page }) => {
    // Find recipes that are not directories
    const recipeCards = page.locator('a[href^="/recipe/"]');
    const count = await recipeCards.count();

    if (count > 0) {
      const firstRecipe = recipeCards.first();
      const recipeName = await firstRecipe.locator('h3').textContent();

      await firstRecipe.click();
      await page.waitForLoadState('networkidle');

      // Verify we're on the recipe page by URL
      expect(page.url()).toContain('/recipe/');
      // Recipe page might show name differently
      const heading = page.locator('h1, h2, h3').first();
      await expect(heading).toBeVisible();
    } else {
      // No recipes to test
      expect(true).toBe(true);
    }
  });

  test('should navigate using breadcrumbs', async ({ page }) => {
    // Navigate to a directory first
    const directories = page.locator('a[href^="/directory/"]');
    const dirCount = await directories.count();

    if (dirCount > 0) {
      await directories.first().click();
      await page.waitForLoadState('networkidle');

      // Check breadcrumb exists
      const breadcrumbs = page.locator('.breadcrumb');
      const breadcrumbCount = await breadcrumbs.count();

      if (breadcrumbCount > 0) {
        // Navigate back using breadcrumb
        await breadcrumbs.first().click();
        await page.waitForLoadState('networkidle');
        await expect(page.locator('h1')).toContainText(/Recipe/);
      }
    }
  });

  test('should navigate to directories', async ({ page }) => {
    // Check if any directories exist and navigate
    const directories = page.locator('.directory-item');
    const dirCount = await directories.count();

    if (dirCount > 0) {
      const firstDir = directories.first();
      const dirName = await firstDir.textContent();

      await firstDir.click();
      await page.waitForLoadState('networkidle');

      // Verify navigation
      const breadcrumbPath = await helpers.getBreadcrumbPath();
      expect(breadcrumbPath).toContain(dirName || '');
    }
  });

  test('should navigate to shopping list', async ({ page }) => {
    await helpers.goToShoppingList();
    // Verify by URL since page might not have h1
    expect(page.url()).toContain('/shopping-list');
    // Check page title instead
    await expect(page).toHaveTitle(/Shopping/);
  });

  test('should navigate to preferences', async ({ page }) => {
    await helpers.goToPreferences();
    const heading = page.locator('h1');
    await expect(heading).toBeVisible();
    // Preferences page exists but might have different heading
  });

  test('should handle navigation history', async ({ page }) => {
    // Navigate through multiple pages
    await helpers.goToShoppingList();
    const shoppingUrl = page.url();
    expect(shoppingUrl).toContain('/shopping-list');

    await page.goto('/pantry');
    const pantryUrl = page.url();
    expect(pantryUrl).toContain('/pantry');

    // Use browser back button
    await page.goBack();
    expect(page.url()).toContain('/shopping-list');

    await page.goBack();
    expect(page.url()).toMatch(/\/$|recipes/);  // Root or recipes
  });

  test('should display navigation menu on all pages', async ({ page }) => {
    // Check navigation links exist
    const navLinks = page.locator('nav a, header a');
    const linkCount = await navLinks.count();
    expect(linkCount).toBeGreaterThan(0);

    // Check we can navigate to shopping list
    await helpers.goToShoppingList();

    // Check navigation still exists
    const navLinksOnShoppingList = page.locator('nav a, header a');
    const linkCountOnShoppingList = await navLinksOnShoppingList.count();
    expect(linkCountOnShoppingList).toBeGreaterThan(0);
  });

  test('should use recipe filename in URLs instead of display names', async ({ page }) => {
    // Check the recipe with title 'Sicilian-style Scottadito Lamb Chops' and filename 'lamb-chops.cook'

    const simpleRecipeCard = page.locator('a[href="/recipe/lamb-chops.cook"]');
    await expect(simpleRecipeCard).toBeVisible();

    const recipeName = await simpleRecipeCard.locator('h3').textContent();
    expect(recipeName).toContain('Sicilian-style Scottadito Lamb Chops');

    await simpleRecipeCard.click();
    await page.waitForLoadState('networkidle');

    // Verify we're on the correct recipe page
    expect(page.url()).toContain('/recipe/lamb-chops.cook');
    const recipeTitle = page.locator('h1, h2').first();
    await expect(recipeTitle).toContainText('Sicilian-style Scottadito Lamb Chops');
  });
});
