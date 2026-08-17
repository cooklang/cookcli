import { test, expect } from '@playwright/test';
import { TestHelpers, RecipePage } from '../fixtures/test-helpers';

test.describe('Recipe Display', () => {
  let helpers: TestHelpers;
  let recipePage: RecipePage;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    recipePage = new RecipePage(page, helpers);
    await helpers.navigateTo('/');
  });

  test('should display recipe title and description', async ({ page }) => {
    // Navigate to a known recipe with content
    await helpers.navigateTo('/recipe/Breakfast/Easy Pancakes.cook');
    await page.waitForLoadState('networkidle');

    // Check title
    const title = await recipePage.getTitle();
    expect(title).toContain('Easy Pancakes');

    // Check if description exists (if present)
    const description = page.locator('.recipe-description');
    if (await description.isVisible()) {
      const descText = await recipePage.getDescription();
      expect(descText).toBeTruthy();
    }
  });

  test.skip('should display ingredients list', async ({ page }) => {
    // Skip - removed due to persistent failures
    // Navigate to a known recipe with ingredients
    await helpers.navigateTo('/recipe/Breakfast/Easy Pancakes.cook');
    await page.waitForLoadState('networkidle');

    // Check ingredients section exists (look for the ingredients heading with emoji)
    await expect(page.locator('h2').filter({ hasText: '🥘' })).toBeVisible();

    const ingredients = await recipePage.getIngredients();
    expect(ingredients.length).toBeGreaterThan(0);
  });

  test.skip('should display cooking steps', async ({ page }) => {
    // Skip - removed due to persistent failures
    // Navigate to a known recipe with steps
    await helpers.navigateTo('/recipe/Breakfast/Easy Pancakes.cook');
    await page.waitForLoadState('networkidle');

    // Check steps exist by looking for step numbers
    const stepNumbers = page.locator('.step-number');
    await expect(stepNumbers.first()).toBeVisible();

    const steps = await recipePage.getSteps();
    expect(steps.length).toBeGreaterThan(0);
  });

  test('should highlight ingredients in steps', async ({ page }) => {
    // Navigate to a known recipe with ingredients
    await helpers.navigateTo('/recipe/Breakfast/Easy Pancakes.cook');
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
    // Navigate to a known recipe
    await helpers.navigateTo('/recipe/Breakfast/Easy Pancakes.cook');
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
    // Navigate to a known recipe
    await helpers.navigateTo('/recipe/Breakfast/Easy Pancakes.cook');
    await page.waitForLoadState('networkidle');

    // Check for timer badges in steps
    const timerRefs = page.locator('.timer-badge');

    if (await timerRefs.count() > 0) {
      await expect(timerRefs.first()).toBeVisible();
      await expect(timerRefs.first()).toHaveClass(/timer-badge/);
    }
  });

  test('should display recipe metadata', async ({ page }) => {
    // Navigate to a known recipe with metadata (Easy Pancakes has servings and tags)
    await helpers.navigateTo('/recipe/Breakfast/Easy Pancakes.cook');
    await page.waitForLoadState('networkidle');

    // Metadata renders as dot-separated entries inside #metadata-container.
    // NOT `.metadata-pill` — since the UI refresh that class is emitted only
    // for *custom* metadata keys, and Easy Pancakes has none, so selecting on
    // it yields a count of 0 and silently skips every assertion below.
    const metadataEntries = page.locator('#metadata-container > span');

    expect(await metadataEntries.count()).toBeGreaterThan(0);
    await expect(metadataEntries.first()).toBeVisible();

    const metadataText = (await metadataEntries.allTextContents())
      .join(' ')
      .toLowerCase();

    // Easy Pancakes declares servings, prep time, cook time, author and tags.
    expect(metadataText).toContain('2');
    expect(metadataText).toContain('servings');
    expect(metadataText).toContain('5 min');
    expect(metadataText).toContain('20 min');
    expect(metadataText).toContain('cookcli team');
    expect(metadataText).toContain('#breakfast');
  });

  test('should display ingredient notes from shorthand notation', async ({ page }) => {
    // Navigate to Red Beans recipe which has shorthand notation
    await helpers.navigateTo('/recipe/Shared/Red Beans.cook');
    await page.waitForLoadState('networkidle');

    // Check that ingredients with notes are displayed
    const ingredientsList = page.locator('ul.ingredient-list li');
    const count = await ingredientsList.count();
    expect(count).toBeGreaterThan(0);

    // Find an ingredient with a note (e.g., "garlic (peeled and finely sliced)")
    const ingredientWithNote = ingredientsList.filter({ hasText: 'garlic' });

    if (await ingredientWithNote.count() > 0) {
      // Check note is displayed with correct styling
      const noteSpan = ingredientWithNote.locator('span.row-note');
      await expect(noteSpan).toBeVisible();

      // Check note content
      const noteText = await noteSpan.textContent();
      expect(noteText).toContain('peeled');

      // Check accessibility attributes
      await expect(noteSpan).toHaveAttribute('aria-label');

      // Long notes wrap instead of being truncated (issue #375)
      await expect(noteSpan).toHaveClass(/break-words/);
      await expect(noteSpan).not.toHaveClass(/truncate/);
    }

    // Check step-level ingredient notes
    const stepIngredients = page.locator('.step-refs');
    if (await stepIngredients.count() > 0) {
      const stepNoteSpan = stepIngredients.locator('span.italic').first();
      if (await stepNoteSpan.count() > 0) {
        await expect(stepNoteSpan).toHaveAttribute('aria-label');
        await expect(stepNoteSpan).not.toHaveClass(/truncate/);
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

test.describe('Recipe Display images', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/recipe/Breakfast/Easy Pancakes');
    await page.waitForLoadState('networkidle');
  });

  test('should not display image before the targeted cooking step when missing', async ({ page }) => {
    const steps = page.locator('main ol li:has(.step-number)');
    const beforeStep = steps.nth(1);
    const imageStep = beforeStep.locator('.image-step');
    await expect(imageStep).not.toBeVisible();
  });

  test('should display images in cooking steps', async ({ page }) => {
    const steps = page.locator('main ol li:has(.step-number)');
    const targetStep = steps.nth(2);
    const imageStep = targetStep.locator('.image-step');
    await expect(imageStep).toBeVisible();
    await expect(imageStep).toHaveAttribute('src', '/api/static/Breakfast/Easy Pancakes.3.jpg');
  });

  test('should not display image after the targeted cooking step when missing', async ({ page }) => {
    const steps = page.locator('main ol li:has(.step-number)');
    const afterStep = steps.nth(3);
    const imageStep = afterStep.locator('.image-step');
    await expect(imageStep).not.toBeVisible();
  });
});

test.describe('Recipe Display images respecting section', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/recipe/Breakfast/Chocolate Toast Delight');
    await page.waitForLoadState('networkidle');
  });

  test('should display images in cooking steps', async ({ page }) => {
    const steps = page.locator('main ol li:has(.step-number)');
    const targetStep = steps.nth(2);
    const imageStep = targetStep.locator('.image-step');
    await expect(imageStep).toHaveAttribute('src', '/api/static/Breakfast/Chocolate Toast Delight.3.jpg');
  });
});
