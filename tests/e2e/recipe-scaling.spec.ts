import { test, expect, type Page } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';

// A recipe without `servings` keeps the plain multiplier.
const NO_SERVINGS = '/recipe/Breakfast/Chocolate%20Toast%20Delight';
// `servings: 6`, and `@mozzarella cheese{100%grams}`.
const SERVES_SIX = '/recipe/Neapolitan%20Pizza';

test.describe('Recipe Scaling', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    await helpers.navigateTo(NO_SERVINGS);
  });

  test('should display scale input', async ({ page }) => {
    const scaleInput = page.locator('#scale');
    await expect(scaleInput).toBeVisible();
    await expect(scaleInput).toHaveValue('1');
    await expect(page.locator('#servings')).toHaveCount(0);
  });

  test('should scale recipe by changing input', async ({ page }) => {
    await helpers.scaleRecipe(2);

    await expect(page.locator('#scale')).toHaveValue('2');
    expect(page.url()).toContain('scale=2');
  });

  test('should scale recipe via URL parameter', async ({ page }) => {
    await page.goto(NO_SERVINGS + '?scale=3');
    await page.waitForLoadState('networkidle');

    await expect(page.locator('#scale')).toHaveValue('3');
  });

  test('should ignore a servings parameter', async ({ page }) => {
    await page.goto(NO_SERVINGS + '?scale=2&servings=6');
    await page.waitForLoadState('networkidle');

    await expect(page.locator('#scale')).toHaveValue('2');
  });

  test('should handle decimal scaling', async ({ page }) => {
    const scaleInput = page.locator('#scale');

    await helpers.scaleRecipe(0.5);
    await expect(scaleInput).toHaveValue('0.5');

    await helpers.scaleRecipe(1.5);
    await expect(scaleInput).toHaveValue('1.5');
  });

  test('should reset scaling to 1', async ({ page }) => {
    const scaleInput = page.locator('#scale');

    await helpers.scaleRecipe(2);
    await expect(scaleInput).toHaveValue('2');

    await helpers.scaleRecipe(1);
    await expect(scaleInput).toHaveValue('1');
  });

  test('should validate scale input', async ({ page }) => {
    const scaleInput = page.locator('#scale');
    await expect(scaleInput).toHaveAttribute('max', '200');

    // Below min: clamped to 0.5
    await helpers.scaleRecipe(0.1);
    await expect(scaleInput).toHaveValue('0.5');

    await helpers.scaleRecipe(2);
    await expect(scaleInput).toHaveValue('2');
  });

  test('should preserve scaling on page refresh', async ({ page }) => {
    await page.goto(NO_SERVINGS + '?scale=2');
    await page.waitForLoadState('networkidle');

    const scaleInput = page.locator('#scale');
    await expect(scaleInput).toHaveValue('2');

    await page.reload();
    await page.waitForLoadState('networkidle');

    expect(page.url()).toContain('scale=2');
    await expect(scaleInput).toHaveValue('2');
  });

  test('should send the scale when adding to shopping list', async ({ page }) => {
    await helpers.scaleRecipe(2);

    let payload: { scale?: number } = {};
    await page.route('**/api/shopping_list/add', async route => {
      payload = route.request().postDataJSON();
      await route.fulfill({ status: 200, body: '{}' });
    });
    await page.getByRole('button', { name: /Add to Shopping List/i }).click();

    await expect.poll(() => payload.scale).toBe(2);
  });

  // Regression coverage for goToScale()'s guard against non-numeric input
  // and for building the scale URL from a JS string constant instead of an
  // HTML-escaped template literal (issue: recipe names with & or ' broke).
  test('should not navigate when the scale input is cleared', async ({ page }) => {
    const scaleInput = page.locator('#scale');
    await scaleInput.fill('');
    await scaleInput.press('Tab');
    await page.waitForTimeout(300);

    expect(page.url()).not.toContain('scale=');
    expect(page.url()).toContain('/recipe/Breakfast/Chocolate');
    await expect(scaleInput).toHaveValue('1');
  });

  test('should navigate to a safely encoded scale URL from the stepper button', async ({ page }) => {
    await page.getByRole('button', { name: 'Increase scale' }).click();
    await page.waitForLoadState('networkidle');

    expect(page.url()).toMatch(/\?scale=1\.5$/);
  });
});

test.describe('Recipe Scaling by servings', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    await helpers.navigateTo(SERVES_SIX);
  });

  const mozzarella = (page: Page) => page.locator('.ingredient-row', { hasText: 'mozzarella' });

  test('should start at the recipe servings', async ({ page }) => {
    const servingsInput = page.locator('#servings');
    await expect(servingsInput).toBeVisible();
    await expect(servingsInput).toHaveValue('6');
    await expect(servingsInput).not.toHaveAttribute('max');
    await expect(page.locator('#scale')).toHaveCount(0);
    await expect(page.locator('.metadata-servings')).toContainText('6');
    await expect(mozzarella(page).first()).toContainText('100');
  });

  test('should scale quantities to the chosen servings', async ({ page }) => {
    await helpers.setServings(3);

    expect(page.url()).toMatch(/\?servings=3$/);
    await expect(page.locator('#servings')).toHaveValue('3');
    await expect(page.locator('.metadata-servings')).toContainText('3');
    await expect(mozzarella(page).first()).toContainText('50');
  });

  test('should say what a scaled recipe was written for', async ({ page }) => {
    const original = page.locator('.metadata-original-servings');
    await expect(original).toHaveCount(0);

    await helpers.setServings(3);
    await expect(original).toHaveText(/Written for 6 servings/);

    // It links back to the recipe's own servings.
    await original.click();
    await page.waitForLoadState('networkidle');
    expect(page.url()).not.toContain('?');
    await expect(page.locator('#servings')).toHaveValue('6');
    await expect(original).toHaveCount(0);
  });

  test('should accept half a serving and clamp below it', async ({ page }) => {
    await helpers.setServings(0.5);
    await expect(page.locator('#servings')).toHaveValue('0.5');
    await expect(page.locator('.metadata-servings')).toContainText('0.5');

    await helpers.setServings(0.1);
    await expect(page.locator('#servings')).toHaveValue('0.5');
  });

  test('should have no upper limit', async ({ page }) => {
    await helpers.setServings(600);

    expect(page.url()).toMatch(/\?servings=600$/);
    await expect(page.locator('#servings')).toHaveValue('600');
    await expect(mozzarella(page).first()).toContainText('10');
  });

  test('should step by half a serving from the buttons', async ({ page }) => {
    await page.getByRole('button', { name: 'Increase servings' }).click();
    await page.waitForLoadState('networkidle');
    expect(page.url()).toMatch(/\?servings=6\.5$/);

    await page.getByRole('button', { name: 'Decrease servings' }).click();
    await page.waitForLoadState('networkidle');
    expect(page.url()).toMatch(/\?servings=6$/);
  });

  test('should show a scale link as servings', async ({ page }) => {
    await page.goto(SERVES_SIX + '?scale=0.5');
    await page.waitForLoadState('networkidle');

    await expect(page.locator('#servings')).toHaveValue('3');
    await expect(mozzarella(page).first()).toContainText('50');
  });

  test('should put the recipe servings back when cleared', async ({ page }) => {
    const servingsInput = page.locator('#servings');
    await servingsInput.fill('');
    await servingsInput.press('Tab');
    await page.waitForTimeout(300);

    expect(page.url()).not.toContain('servings=');
    await expect(servingsInput).toHaveValue('6');
  });

  test('should send the factor when adding to shopping list', async ({ page }) => {
    await helpers.setServings(3);

    let payload: { scale?: number } = {};
    await page.route('**/api/shopping_list/add', async route => {
      payload = route.request().postDataJSON();
      await route.fulfill({ status: 200, body: '{}' });
    });
    await page.getByRole('button', { name: /Add to Shopping List/i }).click();

    await expect.poll(() => payload.scale).toBe(0.5);
  });
});
