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
    await expect(page.locator('#scale')).toHaveValue('5');
  });

  test('keep a factor below the stepper steps', async ({ page }) => {
    // The whole menu at x0.25 makes `@./lamb-chops{}` x0.25.
    await page.goto('/recipe/2 Day Plan.menu?scale=0.25');
    await page.getByRole('link', { name: 'lamb-chops' }).first().click();
    await expect(page).toHaveURL(/\/recipe\/lamb-chops\?scale=0\.25$/);

    const scale = page.locator('#scale');
    await expect(scale).toHaveValue('0.25');
    expect(await scale.evaluate((input: HTMLInputElement) => input.checkValidity())).toBe(true);

    // − stops at the scale the page was opened at, rather than jumping up to 0.5.
    await page.getByRole('button', { name: /decrease/i }).click();
    await expect(scale).toHaveValue('0.25');
    await expect(page).toHaveURL(/\?scale=0\.25$/);
  });
});
