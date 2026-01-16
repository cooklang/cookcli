import { Page, expect } from '@playwright/test';

export class TestHelpers {
  constructor(private page: Page) {}

  /**
   * Navigate to a page and wait for it to load
   */
  async navigateTo(path: string) {
    await this.page.goto(path);
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * Search for a recipe
   */
  async searchRecipe(query: string) {
    const searchInput = this.page.getByPlaceholder('Search recipes...');
    await searchInput.fill(query);
    await searchInput.press('Enter');
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * Click on a recipe card
   */
  async clickRecipeCard(recipeName: string) {
    await this.page.locator('.recipe-card').filter({ hasText: recipeName }).click();
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * Navigate using breadcrumb
   */
  async clickBreadcrumb(text: string) {
    await this.page.locator('.breadcrumb').filter({ hasText: text }).click();
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * Scale a recipe
   */
  async scaleRecipe(scale: number) {
    const scaleInput = this.page.locator('#scale');
    await scaleInput.fill(scale.toString());
    // Trigger change event to update URL
    await scaleInput.evaluate((input: HTMLInputElement, val: string) => {
      input.value = val;
      input.dispatchEvent(new Event('change', { bubbles: true }));
    }, scale.toString());
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * Add ingredients to shopping list
   */
  async addToShoppingList() {
    await this.page.getByRole('button', { name: /Add to Shopping List/i }).click();
    await this.page.waitForTimeout(500); // Wait for the action to complete
  }

  /**
   * Toggle ingredient checkbox in shopping list
   */
  async toggleShoppingListItem(itemText: string) {
    const item = this.page.locator('li').filter({ hasText: itemText });
    const checkbox = item.getByRole('checkbox');
    await checkbox.click();
  }

  /**
   * Clear shopping list
   */
  async clearShoppingList() {
    await this.page.getByRole('button', { name: /Clear List/i }).click();
    await this.page.waitForTimeout(500);
  }

  /**
   * Navigate to shopping list
   */
  async goToShoppingList() {
    await this.page.goto('/shopping-list');
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * Navigate to preferences
   */
  async goToPreferences() {
    await this.page.goto('/preferences');
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * Check if an ingredient badge exists
   */
  async checkIngredientBadge(text: string) {
    return this.page.locator('.ingredient-badge').filter({ hasText: text }).isVisible();
  }

  /**
   * Check if a cookware badge exists
   */
  async checkCookwareBadge(text: string) {
    return this.page.locator('.cookware-badge').filter({ hasText: text }).isVisible();
  }

  /**
   * Check if a timer badge exists
   */
  async checkTimerBadge(text: string) {
    return this.page.locator('.timer-badge').filter({ hasText: text }).isVisible();
  }

  /**
   * Get all recipe cards on the page
   */
  getRecipeCards() {
    return this.page.locator('.recipe-card');
  }

  /**
   * Check if directory navigation item exists
   */
  async checkDirectoryItem(name: string) {
    return this.page.locator('.directory-item').filter({ hasText: name }).isVisible();
  }

  /**
   * Click on a directory to navigate
   */
  async navigateToDirectory(name: string) {
    await this.page.locator('.directory-item').filter({ hasText: name }).click();
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * Get current breadcrumb path
   */
  async getBreadcrumbPath() {
    const breadcrumbs = await this.page.getByRole('navigation').locator('a, span').allTextContents();
    return breadcrumbs.filter(text => text.trim() !== '').join(' > ');
  }

  /**
   * Check recipe metadata
   */
  async checkMetadata(key: string, value: string) {
    return this.page.locator('.metadata-pill').filter({ hasText: `${key}: ${value}` }).isVisible();
  }

  /**
   * Get shopping list items count
   */
  async getShoppingListCount() {
    const items = await this.page.locator('#list-content li').count();
    return items;
  }

  /**
   * Check if shopping list is empty
   */
  async isShoppingListEmpty() {
    const emptyMessage = await this.page.locator('text=Your shopping list is empty').isVisible();
    return emptyMessage;
  }

  /**
   * Upload a pantry configuration file
   */
  async uploadPantryConfig(filePath: string) {
    const fileInput = this.page.locator('input[type="file"]');
    await fileInput.setInputFiles(filePath);
    await this.page.waitForTimeout(500);
  }

  /**
   * Save preferences
   */
  async savePreferences() {
    await this.page.getByRole('button', { name: /Save/i }).click();
    await this.page.waitForTimeout(500);
  }

  /**
   * Check for error messages
   */
  async checkErrorMessage(message: string) {
    return this.page.locator('.error-message').filter({ hasText: message }).isVisible();
  }

  /**
   * Wait for and dismiss alerts
   */
  async handleAlert(expectedMessage?: string) {
    return new Promise<void>((resolve) => {
      this.page.once('dialog', async dialog => {
        if (expectedMessage) {
          expect(dialog.message()).toContain(expectedMessage);
        }
        await dialog.accept();
        resolve();
      });
    });
  }
}

export class RecipePage {
  constructor(private page: Page, private helpers: TestHelpers) {}

  async getTitle() {
    return this.page.locator('h1').textContent();
  }

  async getDescription() {
    return this.page.locator('.recipe-description').textContent();
  }

  async getIngredients() {
    return this.page.locator('.ingredients-list li').allTextContents();
  }

  async getCookware() {
    return this.page.locator('.cookware-list li').allTextContents();
  }

  async getSteps() {
    return this.page.locator('.recipe-step').allTextContents();
  }

  async getTotalTime() {
    return this.page.locator('.total-time').textContent();
  }

  async getServings() {
    return this.page.locator('.servings').textContent();
  }

  async checkIngredientHighlight(ingredient: string) {
    return this.page.locator('.ingredient-ref').filter({ hasText: ingredient }).isVisible();
  }

  async checkCookwareHighlight(cookware: string) {
    return this.page.locator('.cookware-ref').filter({ hasText: cookware }).isVisible();
  }

  async checkTimerHighlight(time: string) {
    return this.page.locator('.timer-ref').filter({ hasText: time }).isVisible();
  }
}

export class ShoppingListPage {
  constructor(private page: Page, private helpers: TestHelpers) {}

  async getItems() {
    return this.page.locator('#list-content li label .item-name').allTextContents();
  }

  async getCheckedItems() {
    return this.page.locator('#list-content li input:checked + label .item-name').allTextContents();
  }

  async getUncheckedItems() {
    return this.page.locator('#list-content li input:not(:checked) + label .item-name').allTextContents();
  }

  async toggleItem(itemText: string) {
    await this.helpers.toggleShoppingListItem(itemText);
  }

  async clearList() {
    await this.helpers.clearShoppingList();
  }

  async isEmpty() {
    return this.helpers.isShoppingListEmpty();
  }

  async getAisleSection(aisleName: string) {
    return this.page.locator('.aisle-section').filter({ hasText: aisleName });
  }

  async getItemsInAisle(aisleName: string) {
    const section = await this.getAisleSection(aisleName);
    return section.locator('li label').allTextContents();
  }
}