import { test, expect } from '@playwright/test';

// Browsing by course is a cookie each viewer sets on the preferences page;
// these tests start without it, as every new browser context does.
test.describe('Browse by course and tag', () => {
  test('the preference switches the front page to courses and back', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('a[href="/directory/Breakfast"]')).toBeVisible();

    await page.goto('/preferences');
    await page.locator('#browse-by-course').click();
    await expect(page.locator('#browse-by-course')).toHaveAttribute('aria-pressed', 'true');

    await page.goto('/');
    await expect(page.locator('a[href="/directory/Breakfast"]')).toHaveCount(0);
    const breakfast = page.locator('a[href="/course/breakfast"]');
    await expect(breakfast).toContainText('Breakfast');
    await expect(breakfast).toContainText('2 recipes');
    await expect(page.locator('a[href="/no-course"]')).toBeVisible();

    await page.goto('/preferences');
    await page.locator('#browse-by-folder').click();
    await expect(page.locator('#browse-by-folder')).toHaveAttribute('aria-pressed', 'true');
    await page.goto('/');
    await expect(page.locator('a[href="/directory/Breakfast"]')).toBeVisible();
  });

  test('a course lists its recipes from every folder', async ({ page }) => {
    await page.goto('/course/dinner');
    await expect(page.locator('h1')).toHaveText('Dinner');
    const titles = page.locator('#recipe-list-grid .recipe-card-link');
    await expect(titles).toHaveCount(4);
    await expect(titles.filter({ hasText: 'Classic Risotto alla Milanese' })).toBeVisible();

    await titles.filter({ hasText: 'Classic Risotto alla Milanese' }).click();
    await expect(page).toHaveURL(/\/recipe\/Risotto\.cook$/);
  });

  test('a recipe links its course', async ({ page }) => {
    await page.goto('/recipe/Breakfast/Easy%20Pancakes');
    await page.locator('a.metadata-course').click();
    await expect(page).toHaveURL(/\/course\/breakfast$/);
    await expect(page.locator('#recipe-list-grid .recipe-card-link', { hasText: 'Easy Pancakes' })).toBeVisible();
  });

  test('a tag on a card opens the recipes with that tag', async ({ page }) => {
    await page.goto('/course/salad');
    const card = page.locator('#recipe-list-grid .recipe-card', { hasText: 'Caprese' });
    await card.locator('a.tag-link', { hasText: 'italian' }).click();

    await expect(page).toHaveURL(/\/tag\/italian$/);
    await expect(page.locator('h1')).toHaveText('#italian');
    const titles = page.locator('#recipe-list-grid .recipe-card-link');
    await expect(titles).toHaveCount(3);
  });

  test('clicking a card outside its tags opens the recipe', async ({ page }) => {
    await page.goto('/tag/mexican');
    const card = page.locator('#recipe-list-grid .recipe-card', { hasText: 'Guacamole' });
    await card.click({ position: { x: 20, y: 20 } });
    await expect(page).toHaveURL(/\/recipe\/Shared\/Guacamole\.cook$/);
  });
});
