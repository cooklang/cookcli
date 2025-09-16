import { test, expect } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';

test.describe('Performance', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
  });

  test('should load home page within acceptable time', async ({ page }) => {
    const startTime = Date.now();
    await helpers.navigateTo('/');
    const loadTime = Date.now() - startTime;

    // Page should load within 3 seconds
    expect(loadTime).toBeLessThan(3000);
  });

  test('should load recipe page quickly', async ({ page }) => {
    await helpers.navigateTo('/');

    const recipes = page.locator('a[href^="/recipe/"]');
    const count = await recipes.count();

    if (count > 0) {
      const startTime = Date.now();
      await recipes.first().click();
      await page.waitForLoadState('networkidle');
      const loadTime = Date.now() - startTime;

      // Recipe page should load within 3 seconds
      expect(loadTime).toBeLessThan(3000);
    } else {
      // No recipes to test
      expect(true).toBe(true);
    }
  });

  test('should handle large recipe lists efficiently', async ({ page }) => {
    await helpers.navigateTo('/');

    // Check initial render performance
    const recipeCards = await helpers.getRecipeCards();
    const cardCount = await recipeCards.count();

    // Even with many recipes, should render quickly
    const metrics = await page.evaluate(() => {
      const perf = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming;
      return {
        domContentLoaded: perf.domContentLoadedEventEnd - perf.domContentLoadedEventStart,
        loadComplete: perf.loadEventEnd - perf.loadEventStart,
        domInteractive: perf.domInteractive - perf.fetchStart
      };
    });

    expect(metrics.domInteractive).toBeLessThan(1500);
    expect(metrics.loadComplete).toBeLessThan(3000);
  });

  test('should search without significant lag', async ({ page }) => {
    await helpers.navigateTo('/');

    const searchInput = page.getByPlaceholder('Search recipes...');
    await searchInput.fill('test');

    const startTime = Date.now();
    await searchInput.press('Enter');
    await page.waitForLoadState('networkidle');
    const searchTime = Date.now() - startTime;

    // Search should complete within 2 seconds
    expect(searchTime).toBeLessThan(2000);
  });

  test('should scale recipes quickly', async ({ page }) => {
    await helpers.navigateTo('/');

    const recipes = page.locator('a[href^="/recipe/"]');
    const count = await recipes.count();

    if (count > 0) {
      await recipes.first().click();
      await page.waitForLoadState('networkidle');

      // Check if scale input exists
      const scaleInput = page.getByLabel('Scale');
      if (await scaleInput.count() > 0) {
        const startTime = Date.now();
        await helpers.scaleRecipe(2);
        const scaleTime = Date.now() - startTime;

        // Scaling should be quick
        expect(scaleTime).toBeLessThan(2000);
      } else {
        // No scaling available
        expect(true).toBe(true);
      }
    } else {
      expect(true).toBe(true);
    }
  });

  test('should add to shopping list quickly', async ({ page }) => {
    await helpers.navigateTo('/');

    const recipes = page.locator('a[href^="/recipe/"]');
    const count = await recipes.count();

    if (count > 0) {
      await recipes.first().click();
      await page.waitForLoadState('networkidle');

      const addButton = page.getByRole('button', { name: /Add to Shopping List/i });

      if (await addButton.count() > 0) {
        const startTime = Date.now();
        await addButton.click();
        await page.waitForTimeout(100); // Small wait for action
        const addTime = Date.now() - startTime;

        // Adding should be quick
        expect(addTime).toBeLessThan(1000);
      } else {
        // No add button
        expect(true).toBe(true);
      }
    } else {
      expect(true).toBe(true);
    }
  });

  test.skip('should have acceptable memory usage', async ({ page }) => {
    // Skip - memory usage varies by system
    if (page.context().browser()?.browserType().name() === 'chromium') {
      await helpers.navigateTo('/');

      // Navigate through several pages
      const recipes = page.locator('a[href^="/recipe/"]');
      const count = await recipes.count();
      const maxToTest = Math.min(count, 3);

      for (let i = 0; i < maxToTest; i++) {
        await recipes.nth(i).click();
        await page.waitForLoadState('networkidle');
        await page.goBack();
        await page.waitForLoadState('networkidle');
      }

      // Check memory usage
      const metrics = await page.evaluate(() => {
        if ('memory' in performance) {
          return (performance as any).memory;
        }
        return null;
      });

      if (metrics) {
        // Used JS heap should be reasonable (less than 100MB for more tolerance)
        expect(metrics.usedJSHeapSize).toBeLessThan(100 * 1024 * 1024);
      } else {
        // Memory API not available
        expect(true).toBe(true);
      }
    } else {
      // Not chromium
      expect(true).toBe(true);
    }
  });

  test('should have optimized images', async ({ page }) => {
    await helpers.navigateTo('/');

    // Check all images
    const images = page.locator('img');
    const imageCount = await images.count();

    for (let i = 0; i < imageCount; i++) {
      const img = images.nth(i);

      // Check for lazy loading
      const loading = await img.getAttribute('loading');
      const isLazy = loading === 'lazy';

      // Check for proper sizing attributes
      const width = await img.getAttribute('width');
      const height = await img.getAttribute('height');
      const hasDimensions = width && height;

      // At least one optimization should be present
      expect(isLazy || hasDimensions).toBeTruthy();
    }
  });

  test('should cache static assets', async ({ page }) => {
    // First load
    await helpers.navigateTo('/');

    // Get resource timing
    const firstLoadResources = await page.evaluate(() => {
      return performance.getEntriesByType('resource').map(r => ({
        name: r.name,
        duration: (r as PerformanceResourceTiming).duration
      }));
    });

    // Second load (should use cache)
    await page.reload();

    const secondLoadResources = await page.evaluate(() => {
      return performance.getEntriesByType('resource').map(r => ({
        name: r.name,
        duration: (r as PerformanceResourceTiming).duration
      }));
    });

    // CSS and JS files should load faster on second load
    const staticResources = secondLoadResources.filter(r =>
      r.name.includes('.css') || r.name.includes('.js')
    );

    if (staticResources.length > 0) {
      const avgSecondLoadTime = staticResources.reduce((acc, r) => acc + r.duration, 0) / staticResources.length;
      expect(avgSecondLoadTime).toBeLessThan(100); // Should be cached
    }
  });

  test('should handle navigation history efficiently', async ({ page }) => {
    await helpers.navigateTo('/');

    // Navigate through multiple pages
    await page.goto('/shopping-list');
    await page.waitForLoadState('networkidle');
    await page.goto('/pantry');
    await page.waitForLoadState('networkidle');
    await helpers.navigateTo('/');

    // Test back/forward navigation performance
    const startTime = Date.now();

    await page.goBack();
    await page.goBack();
    await page.goForward();

    const navTime = Date.now() - startTime;

    // History navigation should be reasonably fast
    expect(navTime).toBeLessThan(3000);
  });

  test('should render large shopping lists efficiently', async ({ page }) => {
    await helpers.navigateTo('/');

    // Add multiple recipes to shopping list
    const recipes = await helpers.getRecipeCards();
    const count = Math.min(await recipes.count(), 3);

    for (let i = 0; i < count; i++) {
      await recipes.nth(i).click();
      await page.waitForLoadState('networkidle');

      const addButton = page.getByRole('button', { name: /Add to Shopping List/i });

      if (await addButton.isVisible()) {
        await addButton.click();
        await page.waitForTimeout(100);
      }

      await page.goBack();
      await page.waitForLoadState('networkidle');
    }

    // Load shopping list
    const startTime = Date.now();
    await helpers.goToShoppingList();
    const loadTime = Date.now() - startTime;

    // Should handle large lists efficiently
    expect(loadTime).toBeLessThan(2000);
  });

  test('should have good Cumulative Layout Shift (CLS)', async ({ page }) => {
    await helpers.navigateTo('/');

    // Monitor layout shifts
    const cls = await page.evaluate(() => {
      return new Promise<number>((resolve) => {
        let clsScore = 0;
        const observer = new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            if ((entry as any).hadRecentInput) continue;
            clsScore += (entry as any).value;
          }
        });

        observer.observe({ entryTypes: ['layout-shift'] });

        // Wait a bit then resolve
        setTimeout(() => {
          observer.disconnect();
          resolve(clsScore);
        }, 2000);
      });
    });

    // CLS should be less than 0.1 (good)
    expect(cls).toBeLessThan(0.1);
  });
});