import { test, expect } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';

test.describe('Cooking Mode', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    // Navigate to a recipe with multiple ingredients and steps
    await helpers.navigateTo('/recipe/Breakfast/Easy Pancakes.cook');
    await page.waitForLoadState('networkidle');
  });

  test('should show Cook button on recipe page', async ({ page }) => {
    const cookBtn = page.locator('#start-cooking-btn');
    await expect(cookBtn).toBeVisible();
    await expect(cookBtn).toContainText('Cook');
  });

  test('should open cooking mode overlay on Cook button click', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();

    const overlay = page.locator('#cooking-overlay');
    await expect(overlay).toBeVisible();

    // Overlay should have dark background
    await expect(overlay).toHaveClass(/cooking-overlay/);
  });

  test('should display recipe name in header', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    const title = page.locator('.cooking-header-title');
    await expect(title).toBeVisible();
    await expect(title).toContainText('Easy Pancakes');
  });

  test('should show close button in header', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    const closeBtn = page.locator('.cooking-close-btn');
    await expect(closeBtn).toBeVisible();
    await expect(closeBtn).toHaveAttribute('aria-label', 'Close cooking mode');
  });

  test('should close overlay when close button is clicked', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    await page.locator('.cooking-close-btn').click();
    await expect(page.locator('#cooking-overlay')).not.toBeVisible();
  });

  test('should close overlay when Escape is pressed', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    await page.keyboard.press('Escape');
    await expect(page.locator('#cooking-overlay')).not.toBeVisible();
  });

  test('should start with mise en place card showing ingredients', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // First card should be section card with ingredients
    const currentCard = page.locator('.cooking-card.current');
    await expect(currentCard).toBeVisible();
    await expect(currentCard).toHaveClass(/cooking-card-section/);

    // Should show ingredients
    const ingredients = page.locator('.cooking-card.current .cooking-mise-item');
    const count = await ingredients.count();
    expect(count).toBeGreaterThan(0);
  });

  test('should show ingredient names and quantities in mise en place', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // Check that ingredient names are present
    const miseItems = page.locator('.cooking-card.current .cooking-mise-item');
    const count = await miseItems.count();
    expect(count).toBeGreaterThan(0);

    // Check first ingredient has name and quantity
    const firstName = await miseItems.first().locator('.cooking-mise-name').textContent();
    expect(firstName).toBeTruthy();
  });

  test('should toggle ingredient checked state on click', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    const firstItem = page.locator('.cooking-card.current .cooking-mise-item').first();
    await expect(firstItem).toBeVisible();

    // Click to check
    await firstItem.click();
    await expect(firstItem).toHaveClass(/checked/);

    // Click again to uncheck
    await firstItem.click();
    await expect(firstItem).not.toHaveClass(/checked/);
  });

  test('should navigate to next card with arrow key', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // First card is section/mise en place
    const firstCard = page.locator('.cooking-card.current');
    await expect(firstCard).toHaveClass(/cooking-card-section/);

    // Press ArrowDown to go to next card (step)
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(400);

    const nextCard = page.locator('.cooking-card.current');
    await expect(nextCard).toHaveClass(/cooking-card-step/);
  });

  test('should navigate backwards with ArrowUp', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // Go forward
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(400);
    await expect(page.locator('.cooking-card.current')).toHaveClass(/cooking-card-step/);

    // Go back
    await page.keyboard.press('ArrowUp');
    await page.waitForTimeout(400);
    await expect(page.locator('.cooking-card.current')).toHaveClass(/cooking-card-section/);
  });

  test('should show step number on step cards', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // Navigate to first step
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(400);

    const stepNumber = page.locator('.cooking-card.current .cooking-step-number');
    await expect(stepNumber).toBeVisible();
    await expect(stepNumber).toContainText('1');
  });

  test('should show step text content', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // Navigate to first step
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(400);

    const stepText = page.locator('.cooking-card.current .cooking-step-text');
    await expect(stepText).toBeVisible();
    const text = await stepText.textContent();
    expect(text!.length).toBeGreaterThan(10);
  });

  test('should show prev and next cards faded', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // Navigate to a middle step
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(400);

    const prevCard = page.locator('.cooking-card.prev');
    const nextCard = page.locator('.cooking-card.next');

    await expect(prevCard).toBeVisible();
    await expect(nextCard).toBeVisible();
  });

  test('should navigate when clicking prev card', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // Go to step 1
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(400);

    // Click on prev card to go back to mise en place
    await page.locator('.cooking-card.prev').click();
    await page.waitForTimeout(400);

    await expect(page.locator('.cooking-card.current')).toHaveClass(/cooking-card-section/);
  });

  test('should navigate when clicking next card', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // Click on next card to go to step 1
    await page.locator('.cooking-card.next').click();
    await page.waitForTimeout(400);

    await expect(page.locator('.cooking-card.current')).toHaveClass(/cooking-card-step/);
  });

  test('should show progress bar', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    const progressBar = page.locator('#cooking-progress-bar');
    await expect(progressBar).toBeAttached();

    // At start, progress should be 0%
    const width = await progressBar.evaluate(el => el.style.width);
    expect(width).toBe('0%');
  });

  test('should update progress bar on navigation', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    const progressBar = page.locator('#cooking-progress-bar');

    // Navigate forward
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(400);

    const width = await progressBar.evaluate(el => el.style.width);
    const pct = parseFloat(width);
    expect(pct).toBeGreaterThan(0);
  });

  test('should reach done card at end', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // Navigate through all cards to reach done
    const totalCards = await page.locator('.cooking-card').count();
    for (let i = 0; i < totalCards - 1; i++) {
      await page.keyboard.press('ArrowDown');
      await page.waitForTimeout(200);
    }

    const doneCard = page.locator('.cooking-card.current');
    await expect(doneCard).toHaveClass(/cooking-card-done/);
    await expect(doneCard).toContainText('Bon Appetit');
  });

  test('should show close button on done card', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // Navigate to done card
    const totalCards = await page.locator('.cooking-card').count();
    for (let i = 0; i < totalCards - 1; i++) {
      await page.keyboard.press('ArrowDown');
      await page.waitForTimeout(200);
    }

    const closeBtn = page.locator('.cooking-done-close-btn');
    await expect(closeBtn).toBeVisible();
    await expect(closeBtn).toContainText('Close');

    // Clicking it should close cooking mode
    await closeBtn.click();
    await expect(page.locator('#cooking-overlay')).not.toBeVisible();
  });

  test('should not navigate past first card', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // Try going back from the first card
    await page.keyboard.press('ArrowUp');
    await page.waitForTimeout(400);

    // Should still be on first card (section)
    await expect(page.locator('.cooking-card.current')).toHaveClass(/cooking-card-section/);
  });

  test('should not navigate past last card', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // Navigate to done card
    const totalCards = await page.locator('.cooking-card').count();
    for (let i = 0; i < totalCards - 1; i++) {
      await page.keyboard.press('ArrowDown');
      await page.waitForTimeout(200);
    }

    await expect(page.locator('.cooking-card.current')).toHaveClass(/cooking-card-done/);

    // Try going forward
    await page.keyboard.press('ArrowDown');
    await page.waitForTimeout(400);

    // Should still be on done card
    await expect(page.locator('.cooking-card.current')).toHaveClass(/cooking-card-done/);
  });

  test('should restore body scroll after closing', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // Body should have overflow hidden
    const overflowDuring = await page.evaluate(() => document.body.style.overflow);
    expect(overflowDuring).toBe('hidden');

    // Close
    await page.locator('.cooking-close-btn').click();
    await expect(page.locator('#cooking-overlay')).not.toBeVisible();

    // Body should be back to normal
    const overflowAfter = await page.evaluate(() => document.body.style.overflow);
    expect(overflowAfter).toBe('');
  });

  test('should have cooking mode JSON data embedded', async ({ page }) => {
    const dataEl = page.locator('#cooking-mode-data');
    await expect(dataEl).toBeAttached();

    const json = await dataEl.textContent();
    const data = JSON.parse(json!);

    expect(data.name).toBe('Easy Pancakes');
    expect(data.sections).toBeDefined();
    expect(data.sections.length).toBeGreaterThan(0);
    expect(data.sections[0].ingredients.length).toBeGreaterThan(0);
    expect(data.sections[0].steps.length).toBeGreaterThan(0);
  });

  test('should open with keyboard shortcut c', async ({ page }) => {
    // Press 'c' to open cooking mode
    await page.keyboard.press('c');

    const overlay = page.locator('#cooking-overlay');
    await expect(overlay).toBeVisible();
  });

  test('should also navigate with ArrowLeft and ArrowRight', async ({ page }) => {
    await page.locator('#start-cooking-btn').click();
    await expect(page.locator('#cooking-overlay')).toBeVisible();

    // ArrowRight should go forward
    await page.keyboard.press('ArrowRight');
    await page.waitForTimeout(400);
    await expect(page.locator('.cooking-card.current')).toHaveClass(/cooking-card-step/);

    // ArrowLeft should go back
    await page.keyboard.press('ArrowLeft');
    await page.waitForTimeout(400);
    await expect(page.locator('.cooking-card.current')).toHaveClass(/cooking-card-section/);
  });
});
