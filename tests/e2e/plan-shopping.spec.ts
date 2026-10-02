import { test, expect, APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// Some days of a meal plan to the shopping list (#385). These tests share the
// seed's `.shopping-list` with the other shopping list specs: run them with
// one worker.

function isoDate(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

function addDays(date: Date, n: number): Date {
  const copy = new Date(date);
  copy.setDate(copy.getDate() + n);
  return copy;
}

async function writePlan(request: APIRequestContext, name: string, start: Date, days: number) {
  const dates = Array.from({ length: days }, (_, i) => isoDate(addDays(start, i)));
  const body = `---\nservings: 2\nplan:\n  start: ${dates[0]}\n  days: ${days}\n---\n` +
    `\n== One (${dates[0]}) ==\n\nDinner: \\\n- @./Risotto{} \\\n- @almonds{50%g}\n` +
    `\n== Two (${dates[1]}) ==\n\nDinner: \\\n- @./Thai Green Curry{} \\\n- @bread{1%loaf}\n` +
    `\n== Three (${dates[2]}) ==\n\nSnacks: \\\n- @almonds{50%g}\n`;
  const response = await request.put(`/api/recipes/${encodeURIComponent(name)}.menu`, {
    data: body,
    headers: { 'content-type': 'text/plain' },
  });
  expect(response.ok()).toBeTruthy();
  return dates;
}

test.describe('Meal plan days to the shopping list', () => {
  let name: string;

  test.beforeEach(async ({}, testInfo) => {
    name = `E2E Days ${testInfo.workerIndex}-${Date.now()}`;
  });

  // Takes off only what these tests put on: other specs use the same list,
  // possibly at the same time, so it is never cleared.
  test.afterEach(async ({ request }) => {
    const items = await (await request.get('/api/shopping_list/items')).json();
    for (const item of items) {
      if (['Risotto', 'Thai Green Curry', `${name}.menu`].includes(item.path)) {
        await request.post('/api/shopping_list/remove', { data: { path: item.path } });
      }
    }
    const extra = await (await request.get('/api/shopping_list/extra_items')).json();
    for (const item of extra) {
      if (['almonds', 'bread'].includes(item.name)) {
        await request.post('/api/shopping_list/remove_extra_item', { data: item });
      }
    }
    await request.delete(`/api/recipes/${encodeURIComponent(name)}.menu`);
  });

  test('adds only the ticked days, their extra items included', async ({ page, request }) => {
    const dates = await writePlan(request, name, new Date(2026, 9, 5), 3);
    await page.goto(`/recipe/${encodeURIComponent(name)}.menu`);

    const add = page.getByRole('button', { name: /Add days to shopping list/ });
    await expect(add).toBeDisabled();
    await page.locator(`.plan-day-pick[value="${dates[0]}"]`).check();
    // The day's label names its checkbox.
    await page.getByRole('checkbox', { name: 'Wed 7 Oct' }).check();
    await expect(page.locator('#plan-pick-count')).toHaveText('2');
    await expect(add).toBeEnabled();

    const added = page.waitForResponse(r => r.url().endsWith('/api/shopping_list/add_menu'));
    await add.click();
    const response = await added;
    expect(response.ok()).toBeTruthy();
    expect(JSON.parse(response.request().postData() ?? '{}')).toMatchObject({
      dates: [dates[0], dates[2]],
      scale: 1,
    });

    await page.goto('/shopping-list');
    const sidebar = page.locator('#selected-recipes');
    await expect(sidebar.getByRole('link', { name: 'Risotto' })).toBeVisible();
    await expect(sidebar.getByRole('link', { name: 'Thai Green Curry' })).toHaveCount(0);
    await expect(page.locator('#extra-items')).toContainText('almonds 100 g');

    const names = page.locator('#list-content .item-name');
    await expect(names.filter({ hasText: /^almonds$/ })).toHaveCount(1);
    await expect(names.filter({ hasText: /^bread$/ })).toHaveCount(0);
    await expect(names.filter({ hasText: /^saffron threads$/ })).not.toHaveCount(0);

    // An extra item goes on its own.
    await page.locator('#extra-items').getByRole('button', { name: 'Remove' }).click();
    await expect(page.locator('#extra-items')).toHaveCount(0);
    await expect(names.filter({ hasText: /^almonds$/ })).toHaveCount(0);
    await expect(sidebar.getByRole('link', { name: 'Risotto' })).toBeVisible();
  });

  test('the shortcuts tick all, none, or the next seven days', async ({ page, request }) => {
    await writePlan(request, name, addDays(new Date(), -2), 12);
    await page.goto(`/recipe/${encodeURIComponent(name)}.menu`);
    const count = page.locator('#plan-pick-count');

    await page.getByRole('button', { name: 'All', exact: true }).click();
    await expect(count).toHaveText('12');
    await page.getByRole('button', { name: 'None', exact: true }).click();
    await expect(count).toHaveText('0');
    await page.getByRole('button', { name: 'Next 7 days' }).click();
    await expect(count).toHaveText('7');
    const today = isoDate(new Date());
    await expect(page.locator(`.plan-day-pick[value="${today}"]`)).toBeChecked();
    await expect(page.locator(`.plan-day-pick[value="${isoDate(addDays(new Date(), -1))}"]`)).not.toBeChecked();
    await expect(page.locator(`.plan-day-pick[value="${isoDate(addDays(new Date(), 7))}"]`)).not.toBeChecked();
  });

  test('the whole plan still goes on as one entry', async ({ page, request }) => {
    await writePlan(request, name, new Date(2026, 9, 5), 3);
    await page.goto(`/recipe/${encodeURIComponent(name)}.menu`);

    await page.getByRole('button', { name: 'Add All to Shopping List' }).click();
    await expect(page.getByRole('button', { name: /Added/ })).toBeVisible();

    const items = await (await request.get('/api/shopping_list/items')).json();
    const plan = items.filter((item: { path: string }) => item.path === `${name}.menu`);
    expect(plan).toHaveLength(1);
    expect(plan[0].recipes.map((r: { path: string }) => r.path)).toEqual(['Risotto', 'Thai Green Curry']);
    const extra = await (await request.get('/api/shopping_list/extra_items')).json();
    expect(extra.filter((item: { name: string }) => item.name === 'almonds')).toEqual([]);
  });

  test('has no accessibility violations', async ({ page, request }) => {
    await writePlan(request, name, new Date(2026, 9, 5), 3);
    await page.goto(`/recipe/${encodeURIComponent(name)}.menu`);
    await page.locator('.plan-day-pick').first().check();

    const results = await new AxeBuilder({ page })
      .include('#plan')
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(results.violations).toEqual([]);
  });
});
