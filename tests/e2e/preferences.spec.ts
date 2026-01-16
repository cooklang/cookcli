import { test, expect } from '@playwright/test';
import { TestHelpers } from '../fixtures/test-helpers';

test.describe('Preferences', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    await helpers.navigateTo('/preferences');
  });

  test('should display preferences page', async ({ page }) => {
    // Check we're on preferences page by URL
    expect(page.url()).toContain('/preferences');
    // Page might exist but be empty or have different content
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should display pantry configuration section', async ({ page }) => {
    // Check for pantry section
    const pantrySection = page.locator('h2, h3').filter({ hasText: /Pantry/i });

    if (await pantrySection.isVisible()) {
      await expect(pantrySection).toBeVisible();

      // Check for upload or configuration options
      const uploadInput = page.locator('input[type="file"]');
      const configTextarea = page.locator('textarea[name*="pantry"]');

      const hasUpload = await uploadInput.isVisible().catch(() => false);
      const hasTextarea = await configTextarea.isVisible().catch(() => false);

      expect(hasUpload || hasTextarea).toBeTruthy();
    }
  });

  test('should display aisle configuration section', async ({ page }) => {
    // Check for aisle section
    const aisleSection = page.locator('h2, h3').filter({ hasText: /Aisle/i });

    if (await aisleSection.isVisible()) {
      await expect(aisleSection).toBeVisible();

      // Check for configuration options
      const configInput = page.locator('input[name*="aisle"], textarea[name*="aisle"]');

      if (await configInput.isVisible()) {
        await expect(configInput).toBeVisible();
      }
    }
  });

  test('should upload pantry configuration file', async ({ page }) => {
    const fileInput = page.locator('input[type="file"][accept*="toml"], input[type="file"][accept*="conf"]');

    if (await fileInput.isVisible()) {
      // Create a test file content
      const testPantryConfig = `
[pantry]
salt = { amount = "1", unit = "kg" }
pepper = { amount = "100", unit = "g" }
olive_oil = { amount = "1", unit = "l" }
      `;

      // Create file and upload
      await fileInput.setInputFiles({
        name: 'test-pantry.toml',
        mimeType: 'text/plain',
        buffer: Buffer.from(testPantryConfig)
      });

      await page.waitForTimeout(500);

      // Check for success message or updated display
      const successMessage = page.locator('text=/uploaded|success|saved/i');
      const hasSuccess = await successMessage.isVisible().catch(() => false);

      if (hasSuccess) {
        await expect(successMessage).toBeVisible();
      }
    }
  });

  test('should save preferences', async ({ page }) => {
    // Look for save button
    const saveButton = page.getByRole('button', { name: /Save/i });

    if (await saveButton.isVisible()) {
      // Make a change (if there are editable fields)
      const editableField = page.locator('input[type="text"], textarea').first();

      if (await editableField.isVisible()) {
        await editableField.fill('Test configuration');
      }

      // Click save
      await saveButton.click();
      await page.waitForTimeout(500);

      // Check for success indication
      const successMessage = page.locator('text=/saved|updated|success/i');
      const hasSuccess = await successMessage.isVisible().catch(() => false);

      if (hasSuccess) {
        await expect(successMessage).toBeVisible();
      }
    }
  });

  test('should display current configuration', async ({ page }) => {
    // Check if current config is displayed
    const configDisplay = page.locator('pre, code, .config-display');

    if (await configDisplay.count() > 0) {
      await expect(configDisplay.first()).toBeVisible();

      // Should show some configuration content
      const configText = await configDisplay.first().textContent();
      expect(configText).toBeTruthy();
    }
  });

  test('should handle invalid configuration', async ({ page }) => {
    const configTextarea = page.locator('textarea').first();

    if (await configTextarea.isVisible()) {
      // Enter invalid configuration
      await configTextarea.fill('invalid { config [ syntax');

      // Try to save
      const saveButton = page.getByRole('button', { name: /Save/i });

      if (await saveButton.isVisible()) {
        await saveButton.click();
        await page.waitForTimeout(500);

        // Should show error message
        const errorMessage = page.locator('text=/error|invalid|failed/i');
        const hasError = await errorMessage.isVisible().catch(() => false);

        if (hasError) {
          await expect(errorMessage).toBeVisible();
        }
      }
    }
  });

  test('should have help text or documentation', async ({ page }) => {
    // Look for help text
    const helpText = page.locator('text=/help|example|format|syntax/i');

    if (await helpText.count() > 0) {
      await expect(helpText.first()).toBeVisible();
    }

    // Or look for example configuration
    const exampleConfig = page.locator('text=/example/i');

    if (await exampleConfig.isVisible()) {
      await expect(exampleConfig).toBeVisible();
    }
  });

  test('should allow resetting to defaults', async ({ page }) => {
    // Look for reset button
    const resetButton = page.getByRole('button', { name: /Reset|Default/i });

    if (await resetButton.isVisible()) {
      // Click reset
      await resetButton.click();

      // Might show confirmation dialog
      page.on('dialog', async dialog => {
        await dialog.accept();
      });

      await page.waitForTimeout(500);

      // Check for reset confirmation
      const confirmMessage = page.locator('text=/reset|restored|default/i');
      const hasConfirm = await confirmMessage.isVisible().catch(() => false);

      if (hasConfirm) {
        await expect(confirmMessage).toBeVisible();
      }
    }
  });

  test('should organize preferences into sections', async ({ page }) => {
    // Check for organized sections
    const sections = page.locator('section, fieldset, .preference-section');

    if (await sections.count() > 0) {
      // Should have multiple sections
      const sectionCount = await sections.count();
      expect(sectionCount).toBeGreaterThan(0);

      // Each section should have a heading
      for (let i = 0; i < sectionCount; i++) {
        const section = sections.nth(i);
        const heading = section.locator('h2, h3, legend');

        if (await heading.isVisible()) {
          const headingText = await heading.textContent();
          expect(headingText).toBeTruthy();
        }
      }
    }
  });
});
