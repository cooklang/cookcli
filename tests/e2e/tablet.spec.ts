import { test, expect } from '@playwright/test';

// iPad portrait. The band between md (768) and lg (1024) is where the
// old layout was worst: desktop density at three-quarters the width.
const TABLET = { width: 820, height: 1180 };

test.describe('Tablet layout', () => {
  test.use({ viewport: TABLET });

  test('app bar is compact', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const nav = page.locator('nav').first();
    const box = await nav.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.height).toBeLessThanOrEqual(56);
  });

  test('nav items are visible at tablet width', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    await expect(page.locator('nav a.nav-pill', { hasText: /Recipes/i })).toBeVisible();
  });
});
