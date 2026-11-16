import { test, expect, APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// Meal plans (#385): a menu with sections on two days or more, shown as a
// calendar. The server under test answers in en-US, where weeks start
// on Sunday.

function isoDate(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

function addDays(date: Date, n: number): Date {
  const copy = new Date(date);
  copy.setDate(copy.getDate() + n);
  return copy;
}

/** Writes a plan straight to disk through the recipe API: a section a day,
 * with Risotto for dinner on the first. */
async function writePlan(request: APIRequestContext, name: string, start: string, days: number) {
  const first = new Date(`${start}T00:00:00`);
  const sections = Array.from({ length: days }, (_, i) =>
    `== Day (${isoDate(addDays(first, i))}) ==\n\nBreakfast: \\\n- \n\nDinner: \\\n- ${i === 0 ? '@./Risotto{}' : ''}\n`);
  const response = await request.put(`/api/recipes/${encodeURIComponent(name)}.menu`, {
    data: sections.join('\n'),
    headers: { 'content-type': 'text/plain' },
  });
  expect(response.ok()).toBeTruthy();
}

test.describe('Meal plan', () => {
  // One file a test: they run side by side.
  let name: string;

  test.beforeEach(async ({}, testInfo) => {
    name = `E2E Plan ${testInfo.workerIndex}-${Date.now()}`;
  });

  test.afterEach(async ({ request }) => {
    await request.delete(`/api/recipes/${encodeURIComponent(name)}.menu`);
  });

  test('is created from the listing and laid out as a calendar', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: 'New Meal Plan' }).click();
    await expect(page).toHaveURL(/\/new\?kind=plan$/);
    await expect(page.getByRole('heading', { name: 'New Meal Plan' })).toBeVisible();

    // The shortcuts fill the fields, and the preview follows them.
    await page.getByRole('button', { name: '2 weeks' }).click();
    await expect(page.getByLabel('Number of days')).toHaveValue('14');
    await page.getByRole('button', { name: 'Today' }).click();
    await expect(page.getByLabel('First day')).toHaveValue(isoDate(new Date()));

    await page.getByLabel('Meal plan path').fill(name);
    await page.getByLabel('First day').fill('2026-10-07');
    await page.getByLabel('Number of days').fill('10');
    await expect(page.locator('#plan-preview')).toHaveText('Wed, Oct 7 → Fri, Oct 16');
    await expect(page.getByLabel('Breakfast')).toBeChecked();
    await expect(page.getByLabel('Snacks')).not.toBeChecked();
    await page.getByLabel('Lunch').uncheck();

    await page.getByRole('button', { name: 'Create Meal Plan' }).click();
    await expect(page).toHaveURL(new RegExp(`/edit/${encodeURIComponent(name)}\\.menu$`));

    await page.goto(`/recipe/${encodeURIComponent(name)}.menu`);
    await expect(page.locator('.tag', { hasText: 'Meal Plan' })).toBeVisible();
    const days = page.locator('#plan .plan-day');
    await expect(days).toHaveCount(10);
    await expect(days.first().locator('time')).toHaveText('Wed 7 Oct');
    await expect(days.last().locator('time')).toHaveText('Fri 16 Oct');
    await expect(days.first().getByRole('heading', { level: 3 })).toHaveText(['Breakfast', 'Dinner']);

    // Sunday-first columns: Wednesday sits under the fourth heading.
    const headings = page.locator('#plan > div[aria-hidden="true"] > div');
    await expect(headings).toHaveText(['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']);
    const wednesday = await headings.nth(3).boundingBox();
    const first = await days.first().boundingBox();
    expect(Math.abs(first!.x - wednesday!.x)).toBeLessThan(2);
  });

  test('marks today and greys the days gone by', async ({ page, request }) => {
    const today = new Date();
    await writePlan(request, name, isoDate(addDays(today, -1)), 3);

    await page.goto(`/recipe/${encodeURIComponent(name)}.menu`);

    const yesterday = page.locator(`#plan .plan-day[data-date="${isoDate(addDays(today, -1))}"]`);
    const current = page.locator(`#plan .plan-day[data-date="${isoDate(today)}"]`);
    await expect(yesterday).toHaveClass(/plan-day-past/);
    await expect(yesterday.getByRole('link', { name: 'Risotto' })).toBeVisible();
    await expect(current).toHaveClass(/plan-day-today/);
    await expect(current).toHaveAttribute('aria-current', 'date');
    await expect(page.locator('#plan .plan-day-past')).toHaveCount(1);
  });

  test('stacks the days on a phone', async ({ page, request }) => {
    await writePlan(request, name, '2026-10-07', 3);
    await page.setViewportSize({ width: 390, height: 844 });

    await page.goto(`/recipe/${encodeURIComponent(name)}.menu`);

    await expect(page.locator('#plan li[aria-hidden="true"]').first()).toBeHidden();
    const boxes = await page.locator('#plan .plan-day').evaluateAll(cells =>
      cells.map(cell => cell.getBoundingClientRect()).map(({ x, y }) => ({ x, y }))
    );
    expect(boxes).toHaveLength(3);
    expect(boxes[1].x).toBe(boxes[0].x);
    expect(boxes[1].y).toBeGreaterThan(boxes[0].y);
  });

  test('has no accessibility violations', async ({ page, request }) => {
    await writePlan(request, name, '2026-10-07', 10);

    await page.goto(`/recipe/${encodeURIComponent(name)}.menu`);
    const planResults = await new AxeBuilder({ page })
      .include('#plan')
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(planResults.violations).toEqual([]);

    await page.goto('/new?kind=plan');
    const formResults = await new AxeBuilder({ page })
      .include('form')
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(formResults.violations).toEqual([]);
  });
});
