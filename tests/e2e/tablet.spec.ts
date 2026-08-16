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

// The bar has almost zero horizontal slack at the md breakpoint: at exactly
// 768px the last icon's right edge lands on 752px, plus 16px padding. Longer
// locale strings (nl-NL "Boodschappenlijst", de-DE) eat the remaining room via
// the search box's flex-shrink. Guard the whole band, not just one width.
test.describe('App bar fits without overflow', () => {
  for (const width of [768, 800, 820, 900, 1024, 1280]) {
    test(`no horizontal overflow at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.goto('/');
      await page.waitForLoadState('networkidle');

      const { scrollWidth, clientWidth, scrollHeight, clientHeight } =
        await page.locator('nav.appbar').evaluate((el) => ({
          scrollWidth: el.scrollWidth,
          clientWidth: el.clientWidth,
          scrollHeight: el.scrollHeight,
          clientHeight: el.clientHeight,
        }));

      expect(scrollWidth, `app bar overflows horizontally at ${width}px`)
        .toBeLessThanOrEqual(clientWidth);
      expect(scrollHeight, `app bar wrapped to a second line at ${width}px`)
        .toBeLessThanOrEqual(clientHeight);
    });
  }
});
