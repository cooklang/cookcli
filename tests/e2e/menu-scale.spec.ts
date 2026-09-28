import { test, expect } from '@playwright/test';

// A menu links each recipe at the factor it shows next to the link (#560).

test.describe('Recipes opened from a menu', () => {
  test('open at the scale the menu asks for', async ({ page }) => {
    await page.goto('/recipe/2 Day Plan.menu');
    // `@./Breakfast/Easy Pancakes{10%servings}` of a 2-serving recipe.
    const pancakes = page.getByRole('link', { name: 'Breakfast › Easy Pancakes' }).first();
    await expect(pancakes).toHaveAttribute('href', /\/recipe\/Breakfast\/Easy Pancakes\?scale=5$/);
    // `@./lamb-chops{}` is x1: no query string.
    await expect(page.getByRole('link', { name: 'lamb-chops' }).first())
      .toHaveAttribute('href', /\/recipe\/lamb-chops$/);

    await pancakes.click();
    await expect(page).toHaveURL(/\/recipe\/Breakfast\/Easy%20Pancakes\?scale=5$/);
    // Easy Pancakes declares its servings, so x5 shows as 10 servings.
    await expect(page.locator('#servings')).toHaveValue('10');
  });

  test('keep a factor below the stepper steps', async ({ page }) => {
    // The whole menu at x0.1 makes `@./lamb-chops{}` x0.1: 0.4 of its 4
    // servings, below the stepper's half a serving.
    await page.goto('/recipe/2 Day Plan.menu?scale=0.1');
    await page.getByRole('link', { name: 'lamb-chops' }).first().click();
    await expect(page).toHaveURL(/\/recipe\/lamb-chops\?scale=0\.1$/);

    const servings = page.locator('#servings');
    await expect(servings).toHaveValue('0.4');
    expect(await servings.evaluate((input: HTMLInputElement) => input.checkValidity())).toBe(true);

    // − stops at the servings the page was opened at, rather than jumping up to 0.5.
    await page.getByRole('button', { name: /decrease/i }).click();
    await expect(servings).toHaveValue('0.4');
    await expect(page).toHaveURL(/\?scale=0\.1$/);

    // + goes to the first step.
    await page.getByRole('button', { name: /increase/i }).click();
    await expect(page).toHaveURL(/\?servings=0\.5$/);
  });
});

test.describe('Recipe scale steps', () => {
  // No `servings`, so the stepper is the plain multiplier.
  const RECIPE = '/recipe/Breakfast/Chocolate%20Toast%20Delight';

  // A scale between steps, such as 1.667 from a menu, moves to the nearest
  // multiple of 0.5 in that direction; one on a step moves by the full step.
  const cases: Array<[string, string, RegExp | string, string]> = [
    ['+ from between steps', '1.667', /increase/i, '2'],
    ['− from between steps', '1.667', /decrease/i, '1.5'],
    ['+ from a step', '1.5', /increase/i, '2'],
    ['− from a step', '1.5', /decrease/i, '1'],
  ];

  for (const [name, from, button, to] of cases) {
    test(name, async ({ page }) => {
      await page.goto(`${RECIPE}?scale=${from}`);
      await page.getByRole('button', { name: button }).click();
      await expect(page).toHaveURL(new RegExp(`\\?scale=${to.replace('.', '\\.')}$`));
      await expect(page.locator('#scale')).toHaveValue(to);
    });
  }

  test('] from between steps keeps to the grid', async ({ page }) => {
    await page.goto(`${RECIPE}?scale=1.667`);
    await page.locator('body').press(']');
    await expect(page).toHaveURL(/\?scale=2\.5$/);
  });

  // The same grid on a recipe that declares servings: lamb-chops serves 4.
  test('servings between steps move onto the grid', async ({ page }) => {
    // x1.667 of 4 servings is 6.67.
    await page.goto('/recipe/lamb-chops?scale=1.667');
    await expect(page.locator('#servings')).toHaveValue('6.67');
    await page.getByRole('button', { name: /increase/i }).click();
    await expect(page).toHaveURL(/\?servings=7$/);
    await expect(page.locator('#servings')).toHaveValue('7');
  });
});
