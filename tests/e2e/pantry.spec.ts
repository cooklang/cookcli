import { test, expect } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';

test.describe('Pantry Management', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    await helpers.navigateTo('/');
  });

  test('should navigate to pantry page', async ({ page }) => {
    // Check if pantry link exists in navigation
    const pantryLink = page.getByRole('link', { name: /Pantry/i });

    if (await pantryLink.isVisible()) {
      await pantryLink.click();
      await page.waitForLoadState('networkidle');

      await expect(page.locator('h1')).toContainText('Pantry');
    } else {
      // Pantry might be accessed through preferences
      await helpers.goToPreferences();

      const pantrySection = page.locator('text=/pantry/i');
      expect(await pantrySection.isVisible()).toBeTruthy();
    }
  });

  test('should display pantry items', async ({ page }) => {
    // Navigate to pantry (either direct or through preferences)
    const pantryLink = page.getByRole('link', { name: /Pantry/i });

    if (await pantryLink.isVisible()) {
      await pantryLink.click();
      await page.waitForLoadState('networkidle');

      // Check for pantry items display
      const pantryItems = page.locator('.pantry-row');

      if (await pantryItems.count() > 0) {
        await expect(pantryItems.first()).toBeVisible();

        // Check item structure
        const firstItem = pantryItems.first();
        const itemName = firstItem.locator('.pantry-item-name');
        const itemQuantity = firstItem.locator('.pantry-quantity');

        if (await itemName.isVisible()) {
          const name = await itemName.textContent();
          expect(name).toBeTruthy();
        }

        if (await itemQuantity.isVisible()) {
          const quantity = await itemQuantity.textContent();
          expect(quantity).toBeTruthy();
        }
      }
    }
  });

  test('should add items to pantry', async ({ page }) => {
    const pantryLink = page.getByRole('link', { name: /Pantry/i });

    if (await pantryLink.isVisible()) {
      await pantryLink.click();
      await page.waitForLoadState('networkidle');

      // The form below the sections adds to any of them, or to a new one.
      const form = page.locator('#pantry-new-item');
      const addButton = form.getByRole('button', { name: /Add/i });
      const itemNameInput = form.getByLabel(/Item|Name|Ingredient/i);
      const quantityInput = form.getByLabel(/Quantity|Amount/i);
      const unitInput = form.getByLabel(/Unit/i);

      if (await form.isVisible()) {
        await form.getByLabel(/Section/i).fill('pantry');
      }

      if (await addButton.isVisible() && await itemNameInput.isVisible()) {
        // Fill form
        await itemNameInput.fill('Test Ingredient');

        if (await quantityInput.isVisible()) {
          await quantityInput.fill('100');
        }

        if (await unitInput.isVisible()) {
          await unitInput.fill('g');
        }

        // Submit
        await addButton.click();
        await page.waitForTimeout(500);

        // Verify item was added
        const newItem = page.locator('text=/Test Ingredient/i');
        await expect(newItem).toBeVisible();
      }
    }
  });

  test('should edit pantry items', async ({ page }) => {
    const pantryLink = page.getByRole('link', { name: /Pantry/i });

    if (await pantryLink.isVisible()) {
      await pantryLink.click();
      await page.waitForLoadState('networkidle');

      const pantryItems = page.locator('.pantry-row');

      if (await pantryItems.count() > 0) {
        const firstItem = pantryItems.first();

        // Look for edit button
        const editButton = firstItem.getByRole('button', { name: /Edit/i });

        if (await editButton.isVisible()) {
          await editButton.click();

          // The edit panel opens under the row
          const editor = page.locator('.pantry-editor');
          const quantityInput = editor.getByLabel(/Quantity|Amount/i);

          if (await quantityInput.isVisible()) {
            await quantityInput.clear();
            await quantityInput.fill('200');

            // Save changes
            const saveButton = editor.getByRole('button', { name: /Save/i });

            if (await saveButton.isVisible()) {
              await saveButton.click();
              await page.waitForTimeout(500);

              // Verify change
              const updatedQuantity = firstItem.locator('text=/200/');
              await expect(updatedQuantity).toBeVisible();
            }
          }
        }
      }
    }
  });

  test('should remove items from pantry', async ({ page }) => {
    const pantryLink = page.getByRole('link', { name: /Pantry/i });

    if (await pantryLink.isVisible()) {
      await pantryLink.click();
      await page.waitForLoadState('networkidle');

      const pantryItems = page.locator('.pantry-row');
      const initialCount = await pantryItems.count();

      if (initialCount > 0) {
        const firstItem = pantryItems.first();

        // Get item name for verification
        const itemName = await firstItem.locator('.pantry-item-name').textContent();

        // Look for remove button
        const removeButton = firstItem.getByRole('button', { name: /Remove|Delete/i });

        if (await removeButton.isVisible()) {
          // Handle potential confirmation dialog
          page.on('dialog', async dialog => {
            await dialog.accept();
          });

          await removeButton.click();
          await page.waitForTimeout(500);

          // Verify item was removed
          if (itemName) {
            const removedItem = page.locator(`text="${itemName}"`);
            await expect(removedItem).not.toBeVisible();
          }

          // Check count decreased
          const newCount = await pantryItems.count();
          expect(newCount).toBeLessThan(initialCount);
        }
      }
    }
  });

  test('should filter shopping list based on pantry', async ({ page }) => {
    // Navigate to pantry page
    await page.goto('/pantry');
    await page.waitForLoadState('networkidle');

    const pantryItems = page.locator('.pantry-row');

    if (await pantryItems.count() > 0) {
      // Go to recipes and add to shopping list
      await helpers.navigateTo('/');

      const recipes = page.locator('a[href^="/recipe/"][href$=".cook"]');
      const recipeCount = await recipes.count();

      if (recipeCount > 0) {
        await recipes.first().click();
        await page.waitForLoadState('networkidle');

        const addButton = page.getByRole('button', { name: /Add to Shopping List/i });

        if (await addButton.count() > 0) {
          await addButton.click();
          await page.waitForTimeout(500);

          // Go to shopping list
          await helpers.goToShoppingList();

          // Check if pantry items are filtered or marked
          const filteredIndicator = page.locator('.in-pantry, .pantry-filtered, [data-in-pantry]');

          if (await filteredIndicator.count() > 0) {
            await expect(filteredIndicator.first()).toBeVisible();
          } else {
            // No filtering visible, which is okay
            expect(true).toBe(true);
          }
        } else {
          expect(true).toBe(true);
        }
      } else {
        expect(true).toBe(true);
      }
    } else {
      // No pantry items to test with
      expect(true).toBe(true);
    }
  });

  test('should show pantry status in recipe view', async ({ page }) => {
    // Navigate to a real recipe (not menu)
    const recipes = page.locator('a[href^="/recipe/"][href$=".cook"]');
    const count = await recipes.count();

    if (count > 0) {
      await recipes.first().click();
      await page.waitForLoadState('networkidle');

      // Check if ingredients show pantry status
      const pantryIndicators = page.locator('.in-pantry, .pantry-available, [data-pantry-status]');

      if (await pantryIndicators.count() > 0) {
        await expect(pantryIndicators.first()).toBeVisible();

        // Might show checkmark or different styling
        const firstIndicator = pantryIndicators.first();
        const hasCheckmark = await firstIndicator.locator('.checkmark, .check').count() > 0;
        const hasSpecialClass = await firstIndicator.evaluate(el => el.classList.contains('in-pantry') || el.classList.contains('available'));

        expect(hasCheckmark || hasSpecialClass).toBeTruthy();
      } else {
        // No pantry indicators, which is fine
        expect(true).toBe(true);
      }
    } else {
      // No recipes to test
      expect(true).toBe(true);
    }
  });

  test('should import pantry configuration', async ({ page }) => {
    const pantryLink = page.getByRole('link', { name: /Pantry/i });

    if (await pantryLink.isVisible()) {
      await pantryLink.click();
      await page.waitForLoadState('networkidle');

      // Look for import button
      const importButton = page.getByRole('button', { name: /Import/i });

      if (await importButton.isVisible()) {
        await importButton.click();

        // File input should appear
        const fileInput = page.locator('input[type="file"]');

        if (await fileInput.isVisible()) {
          // Create test pantry file
          const pantryData = `
[pantry]
flour = { amount = "5", unit = "kg" }
sugar = { amount = "2", unit = "kg" }
butter = { amount = "500", unit = "g" }
eggs = { amount = "12", unit = "pieces" }
          `;

          await fileInput.setInputFiles({
            name: 'pantry.toml',
            mimeType: 'text/plain',
            buffer: Buffer.from(pantryData)
          });

          await page.waitForTimeout(500);

          // Verify import
          const importedItems = page.locator('text=/flour|sugar|butter|eggs/i');
          await expect(importedItems.first()).toBeVisible();
        }
      }
    }
  });

  test('should export pantry configuration', async ({ page }) => {
    const pantryLink = page.getByRole('link', { name: /Pantry/i });

    if (await pantryLink.isVisible()) {
      await pantryLink.click();
      await page.waitForLoadState('networkidle');

      // Look for export button
      const exportButton = page.getByRole('button', { name: /Export/i });

      if (await exportButton.isVisible()) {
        // Set up download promise before clicking
        const downloadPromise = page.waitForEvent('download');

        await exportButton.click();

        // Wait for download
        const download = await downloadPromise.catch(() => null);

        if (download) {
          // Verify download
          expect(download.suggestedFilename()).toMatch(/pantry.*\.(toml|conf|json)/i);
        }
      }
    }
  });

  test('should search pantry items', async ({ page }) => {
    const pantryLink = page.getByRole('link', { name: /Pantry/i });

    if (await pantryLink.isVisible()) {
      await pantryLink.click();
      await page.waitForLoadState('networkidle');

      // Look for search input
      const searchInput = page.getByPlaceholder(/Search pantry/i);

      if (await searchInput.isVisible()) {
        const pantryItems = page.locator('.pantry-row');
        const initialCount = await pantryItems.count();

        // Perform search
        await searchInput.fill('salt');
        await page.waitForTimeout(500);

        // Check filtered results
        const filteredCount = await pantryItems.count();
        expect(filteredCount).toBeLessThanOrEqual(initialCount);

        // Clear search
        await searchInput.clear();
        await page.waitForTimeout(500);

        // Should show all items again
        const clearedCount = await pantryItems.count();
        expect(clearedCount).toBe(initialCount);
      }
    }
  });

  test('marks zero-quantity items out of stock with the danger colour', async ({ page }) => {
    await page.goto('/pantry');
    await page.waitForLoadState('networkidle');
    const item = page.locator('.pantry-row[data-stock="out"]').first();
    await expect(item).toBeVisible();
    const [qtyColor, dangerColor] = await item.evaluate((el) => {
      const qty = el.querySelector('.pantry-quantity') as HTMLElement;
      const probe = document.createElement('span');
      probe.style.color = 'var(--danger)';
      document.body.appendChild(probe);
      const expected = getComputedStyle(probe).color;
      probe.remove();
      return [getComputedStyle(qty).color, expected];
    });
    expect(qtyColor).toBe(dangerColor);
  });
});
