import { test, expect, Page, APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// Changing a meal plan from its calendar (#385): the recipe picker adds to a
// day's meal, each line's menu removes, moves or copies it. Every change is
// written to the `.menu` file, which these tests read back.

const PLAN = '---\nservings: 2\n---\n\n' +
  '== Wednesday (2026-10-07) ==\n\n-- a note to keep\n\nDinner: \\\n- @./Risotto{} \\\n- @salad{1%bowl}\n\n' +
  '== Friday (2026-10-09) ==\n\nBreakfast: \\\n- \n\nDinner: \\\n- \n';

function day(page: Page, date: string) {
  return page.locator(`#plan .plan-day[data-date="${date}"]`);
}

function meal(page: Page, date: string, name: string) {
  return day(page, date).locator(`.plan-meal[data-meal="${name}"]`);
}

async function readPlan(request: APIRequestContext, name: string): Promise<string> {
  const response = await request.get(`/api/recipes/raw/${encodeURIComponent(name)}.menu`);
  expect(response.ok()).toBeTruthy();
  return response.text();
}

test.describe('Editing a meal plan from its calendar', () => {
  // One file a test: they run side by side.
  let name: string;

  test.beforeEach(async ({ page, request }, testInfo) => {
    name = `E2E Planner ${testInfo.workerIndex}-${Date.now()}`;
    const response = await request.put(`/api/recipes/${encodeURIComponent(name)}.menu`, {
      data: PLAN,
      headers: { 'content-type': 'text/plain' },
    });
    expect(response.ok()).toBeTruthy();
    await page.goto(`/recipe/${encodeURIComponent(name)}.menu`);
  });

  test.afterEach(async ({ request }) => {
    await request.delete(`/api/recipes/${encodeURIComponent(name)}.menu`);
  });

  test('adds a recipe to a meal with the picker', async ({ page, request }) => {
    await meal(page, '2026-10-08', 'Dinner').getByRole('button', { name: /Add to Dinner/ }).click();
    const picker = page.getByRole('dialog', { name: 'Choose a recipe' });
    await expect(picker).toBeVisible();
    await expect(picker.getByLabel('Servings')).toHaveValue('2');
    const searched = page.waitForResponse(r => r.url().includes('/api/search?q=caprese'));
    await picker.getByRole('combobox').fill('caprese');
    await searched;
    await expect(picker.getByRole('option', { selected: true })).toContainText('Caprese');
    await page.keyboard.press('Enter');

    await expect(meal(page, '2026-10-08', 'Dinner').getByRole('link', { name: 'Caprese' })).toBeVisible();
    // Focus comes back to the meal the recipe went to.
    await expect(meal(page, '2026-10-08', 'Dinner').getByRole('button', { name: /Add to Dinner/ })).toBeFocused();
    expect(await readPlan(request, name)).toBe(
      PLAN.replace('== Friday', '== Thursday (2026-10-08) ==\n\nDinner: \\\n- @./Salads/Caprese{2%servings}\n\n== Friday')
    );
  });

  test('removes a line from its menu', async ({ page, request }) => {
    const salad = meal(page, '2026-10-07', 'Dinner').locator('.plan-line').nth(1);
    await salad.getByRole('button', { name: /Change/ }).click();
    await page.getByRole('menuitem', { name: 'Remove' }).click();

    await expect(meal(page, '2026-10-07', 'Dinner').locator('.plan-line')).toHaveCount(1);
    expect(await readPlan(request, name)).toBe(
      PLAN.replace('- @./Risotto{} \\\n- @salad{1%bowl}\n', '- @./Risotto{}\n')
    );
  });

  test('moves a line to another day with the keyboard', async ({ page, request }) => {
    const risotto = meal(page, '2026-10-07', 'Dinner').locator('.plan-line').first();
    const button = risotto.getByRole('button', { name: /Change/ });
    await button.focus();
    await page.keyboard.press('Enter');
    await expect(button).toHaveAttribute('aria-expanded', 'true');
    await expect(page.getByRole('menuitem', { name: 'Move to…' })).toBeFocused();
    await page.keyboard.press('Enter');

    const dialog = page.getByRole('dialog', { name: 'Move to another day' });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByLabel('Day')).toBeFocused();
    await dialog.getByLabel('Day').selectOption('2026-10-09');
    await dialog.getByLabel('Meal').selectOption('Breakfast');
    await dialog.getByLabel('Meal').focus();
    // Meal → Cancel → Move.
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await expect(dialog.getByRole('button', { name: 'Move', exact: true })).toBeFocused();
    await page.keyboard.press('Enter');

    await expect(meal(page, '2026-10-09', 'Breakfast').getByRole('link', { name: 'Risotto' })).toBeVisible();
    await expect(meal(page, '2026-10-07', 'Dinner').getByRole('link', { name: 'Risotto' })).toHaveCount(0);
    expect(await readPlan(request, name)).toBe(
      // Friday's empty breakfast bullet is filled in.
      PLAN.replace('- @./Risotto{} \\\n', '').replace('Breakfast: \\\n- \n', 'Breakfast: \\\n- @./Risotto{}\n')
    );
  });

  test('Escape closes the line menu and gives focus back', async ({ page }) => {
    const button = meal(page, '2026-10-07', 'Dinner').locator('.plan-line').first().getByRole('button', { name: /Change/ });
    await button.click();
    await expect(page.getByRole('menu')).toBeVisible();
    await page.keyboard.press('ArrowDown');
    await expect(page.getByRole('menuitem', { name: 'Copy to…' })).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('menu')).toBeHidden();
    await expect(button).toBeFocused();
  });

  test('a change to a plan edited elsewhere is refused', async ({ page, request }) => {
    await request.put(`/api/recipes/${encodeURIComponent(name)}.menu`, {
      data: PLAN + '\n-- changed elsewhere\n',
      headers: { 'content-type': 'text/plain' },
    });

    await meal(page, '2026-10-07', 'Dinner').locator('.plan-line').first().getByRole('button', { name: /Change/ }).click();
    await page.getByRole('menuitem', { name: 'Remove' }).click();

    await expect(page.getByRole('alert')).toContainText('The plan changed');
    expect(await readPlan(request, name)).toBe(PLAN + '\n-- changed elsewhere\n');
  });

  test('has no accessibility violations', async ({ page }) => {
    const results = await new AxeBuilder({ page })
      .include('#plan')
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(results.violations).toEqual([]);

    await meal(page, '2026-10-07', 'Dinner').locator('.plan-line').first().getByRole('button', { name: /Change/ }).click();
    await page.getByRole('menuitem', { name: 'Copy to…' }).click();
    await expect(page.getByRole('dialog', { name: 'Copy to another day' })).toBeVisible();
    const dialog = await new AxeBuilder({ page })
      .include('#plan-target')
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(dialog.violations).toEqual([]);
  });
});
