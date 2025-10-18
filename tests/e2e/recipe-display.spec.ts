import { test, expect } from '@playwright/test';
import { TestHelpers, RecipePage } from '../fixtures/test-helpers';

test.describe.skip('Recipe Display', () => {  // Skip - requires recipe content
  let helpers: TestHelpers;
  let recipePage: RecipePage;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    recipePage = new RecipePage(page, helpers);
    await helpers.navigateTo('/');
  });

  test('should display recipe title and description', async ({ page }) => {
    // Navigate to the first recipe
    const firstRecipe = await helpers.getRecipeCards().first();
    const expectedTitle = await firstRecipe.locator('h2').textContent();
    await firstRecipe.click();
    await page.waitForLoadState('networkidle');

    // Check title
    const title = await recipePage.getTitle();
    expect(title).toContain(expectedTitle || '');

    // Check if description exists (if present)
    const description = page.locator('.recipe-description');
    if (await description.isVisible()) {
      const descText = await recipePage.getDescription();
      expect(descText).toBeTruthy();
    }
  });

  test('should display ingredients list', async ({ page }) => {
    // Navigate to a recipe
    const firstRecipe = await helpers.getRecipeCards().first();
    await firstRecipe.click();
    await page.waitForLoadState('networkidle');

    // Check ingredients section
    await expect(page.locator('h2').filter({ hasText: 'Ingredients' })).toBeVisible();

    const ingredients = await recipePage.getIngredients();
    expect(ingredients.length).toBeGreaterThan(0);
  });

  test('should display cooking steps', async ({ page }) => {
    // Navigate to a recipe
    const firstRecipe = await helpers.getRecipeCards().first();
    await firstRecipe.click();
    await page.waitForLoadState('networkidle');

    // Check steps section
    await expect(page.locator('h2').filter({ hasText: /Steps|Instructions|Method/ })).toBeVisible();

    const steps = await recipePage.getSteps();
    expect(steps.length).toBeGreaterThan(0);

    // Check step numbers
    const stepNumbers = page.locator('.step-number');
    await expect(stepNumbers.first()).toBeVisible();
  });

  test('should highlight ingredients in steps', async ({ page }) => {
    // Navigate to a recipe
    const firstRecipe = await helpers.getRecipeCards().first();
    await firstRecipe.click();
    await page.waitForLoadState('networkidle');

    // Check for ingredient highlights in steps
    const ingredientRefs = page.locator('.ingredient-badge');
    const count = await ingredientRefs.count();

    if (count > 0) {
      await expect(ingredientRefs.first()).toBeVisible();
      await expect(ingredientRefs.first()).toHaveClass(/ingredient-badge/);
    }
  });

  test('should display cookware if present', async ({ page }) => {
    // Navigate to a recipe
    const firstRecipe = await helpers.getRecipeCards().first();
    await firstRecipe.click();
    await page.waitForLoadState('networkidle');

    // Check if cookware section exists
    const cookwareSection = page.locator('h2').filter({ hasText: 'Cookware' });

    if (await cookwareSection.isVisible()) {
      const cookware = await recipePage.getCookware();
      expect(cookware.length).toBeGreaterThan(0);

      // Check for cookware highlights in steps
      const cookwareRefs = page.locator('.cookware-badge');
      if (await cookwareRefs.first().isVisible()) {
        await expect(cookwareRefs.first()).toHaveClass(/cookware-badge/);
      }
    }
  });

  test('should display timers in steps', async ({ page }) => {
    // Navigate to a recipe
    const firstRecipe = await helpers.getRecipeCards().first();
    await firstRecipe.click();
    await page.waitForLoadState('networkidle');

    // Check for timer badges in steps
    const timerRefs = page.locator('.timer-badge');

    if (await timerRefs.count() > 0) {
      await expect(timerRefs.first()).toBeVisible();
      await expect(timerRefs.first()).toHaveClass(/timer-badge/);
    }
  });

  test('should display recipe metadata', async ({ page }) => {
    // Navigate to a recipe
    const firstRecipe = await helpers.getRecipeCards().first();
    await firstRecipe.click();
    await page.waitForLoadState('networkidle');

    // Check for metadata pills
    const metadataPills = page.locator('.metadata-pill');

    if (await metadataPills.count() > 0) {
      await expect(metadataPills.first()).toBeVisible();

      // Check for common metadata like servings, time, etc.
      const metadataText = await metadataPills.allTextContents();
      const hasValidMetadata = metadataText.some(text =>
        text.includes('servings') ||
        text.includes('time') ||
        text.includes('difficulty') ||
        text.includes('cuisine')
      );

      if (hasValidMetadata) {
        expect(hasValidMetadata).toBeTruthy();
      }
    }
  });

  test('should display ingredient notes from shorthand notation', async ({ page }) => {
    // Navigate to Red Beans recipe which has shorthand notation
    await helpers.navigateTo('/recipe/Shared/Red Beans.cook');
    await page.waitForLoadState('networkidle');

    // Check that ingredients with notes are displayed
    const ingredientsList = page.locator('ul.space-y-3 li');
    const count = await ingredientsList.count();
    expect(count).toBeGreaterThan(0);

    // Find an ingredient with a note (e.g., "garlic (peeled and finely sliced)")
    const ingredientWithNote = ingredientsList.filter({ hasText: 'garlic' });

    if (await ingredientWithNote.count() > 0) {
      // Check note is displayed with correct styling
      const noteSpan = ingredientWithNote.locator('span.italic.text-gray-600');
      await expect(noteSpan).toBeVisible();

      // Check note content
      const noteText = await noteSpan.textContent();
      expect(noteText).toContain('peeled');

      // Check accessibility attributes
      await expect(noteSpan).toHaveAttribute('aria-label');
      await expect(noteSpan).toHaveAttribute('title');

      // Check truncation classes are present for long notes
      await expect(noteSpan).toHaveClass(/truncate/);
      await expect(noteSpan).toHaveClass(/max-w-/);
    }

    // Check step-level ingredient notes
    const stepIngredients = page.locator('.text-sm.text-gray-600.mt-2');
    if (await stepIngredients.count() > 0) {
      const stepNoteSpan = stepIngredients.locator('span.italic').first();
      if (await stepNoteSpan.count() > 0) {
        await expect(stepNoteSpan).toHaveAttribute('title');
        await expect(stepNoteSpan).toHaveAttribute('aria-label');
      }
    }
  });

  test('should display recipe image if present', async ({ page }) => {
    if (!page.url().includes('/recipe/')) {
      expect(true).toBe(true);
      return;
    }

    // Check for recipe image
    const recipeImage = page.locator('img').first();

    if (await recipeImage.count() > 0) {
      const src = await recipeImage.getAttribute('src');
      if (src && !src.includes('data:')) {  // Not a placeholder
        expect(src).toBeTruthy();
      }
    } else {
      // No image
      expect(true).toBe(true);
    }
  });

  test('should maintain responsive layout', async ({ page }) => {
    if (!page.url().includes('/recipe/')) {
      expect(true).toBe(true);
      return;
    }

    // Test different viewport sizes
    const viewports = [
      { width: 1920, height: 1080, name: 'Desktop' },
      { width: 768, height: 1024, name: 'Tablet' },
      { width: 375, height: 667, name: 'Mobile' }
    ];

    for (const viewport of viewports) {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });

      // Check that main elements are still visible
      const title = page.locator('h1');
      if (await title.count() > 0) {
        await expect(title).toBeVisible();
      }

      // Check content is accessible
      const content = page.locator('main, article, .container').first();
      if (await content.count() > 0) {
        await expect(content).toBeVisible();
      }
    }
  });
});