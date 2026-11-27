import { test, expect, Page } from '@playwright/test';

// Typing items into the shopping list. Shares the seed's `.shopping-list`
// with the other shopping list specs: every item here starts with "E2E ",
// and only those are taken off afterwards.

async function removeOurs(request: import('@playwright/test').APIRequestContext) {
  const extra = await (await request.get('/api/shopping_list/extra_items')).json();
  for (const item of extra) {
    if (String(item.name).startsWith('E2E ')) {
      await request.post('/api/shopping_list/remove_extra_item', {
        data: { name: item.name, quantity: item.quantity ?? null },
      });
    }
  }
}

async function paste(page: Page, text: string) {
  await page.locator('#manual-add-input').evaluate((input, value) => {
    const data = new DataTransfer();
    data.setData('text/plain', value);
    input.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
  }, text);
}

function row(page: Page, name: string) {
  return page.locator(`#list-content li:has([data-ingredient-name="${name}"])`);
}

test.describe('Typed items on the shopping list', () => {
  test.beforeEach(async ({ request }) => removeOurs(request));
  test.afterEach(async ({ request }) => removeOurs(request));

  test('typing shows the amount and Enter adds the item', async ({ page }) => {
    await page.goto('/shopping-list');
    const input = page.locator('#manual-add-input');
    await input.fill('E2E oat drink 1.5l');
    await expect(page.locator('#manual-add-chip')).toHaveText('1.5 l');
    await input.press('Enter');
    await expect(input).toHaveValue('');
    await expect(input).toBeFocused();
    await expect(row(page, 'E2E oat drink')).toContainText('1.5');
    await expect(row(page, 'E2E oat drink')).toHaveClass(/row-flash/);
    await expect(page.locator('#extra-items')).toContainText('E2E oat drink');
  });

  test('pasting a list previews it, and removed lines stay out', async ({ page, request }) => {
    await page.goto('/shopping-list');
    await paste(page, '- E2E rye bread\n- E2E lemons 3\n- E2E basil');
    const rows = page.locator('#manual-paste-rows li');
    await expect(rows).toHaveCount(3);
    await rows.nth(2).getByRole('button').click();
    await page.locator('#manual-paste-add').click();
    await expect(page.locator('#manual-paste')).toBeHidden();
    await expect(row(page, 'E2E rye bread')).toBeVisible();
    await expect(row(page, 'E2E lemons')).toBeVisible();
    const extra = await (await request.get('/api/shopping_list/extra_items')).json();
    const names = extra.map((item: { name: string }) => item.name);
    expect(names).toContain('E2E lemons');
    expect(names).not.toContain('E2E basil');
  });

  test('an item is edited from Other items', async ({ page, request }) => {
    await request.post('/api/shopping_list/add_items', { data: { lines: ['E2E rye flour 1 kg'] } });
    await page.goto('/shopping-list');
    await page.locator('[data-action="edit-extra"][data-name="E2E rye flour"]').click();
    const input = page.locator('#manual-add-input');
    await expect(input).toHaveValue('E2E rye flour 1 kg');
    await input.fill('E2E rye flour 2 kg');
    await input.press('Enter');
    await expect(page.locator('#manual-add-cancel')).toBeHidden();
    await expect.poll(async () => {
      const extra = await (await request.get('/api/shopping_list/extra_items')).json();
      return extra.find((item: { name: string }) => item.name === 'E2E rye flour')?.quantity;
    }).toBe('2%kg');
  });

  test('an item is removed from Other items', async ({ page, request }) => {
    await request.post('/api/shopping_list/add_items', { data: { lines: ['E2E capers'] } });
    await page.goto('/shopping-list');
    await page.locator('[data-action="remove-extra"][data-name="E2E capers"]').click();
    // The box goes away with its last item, so count the row rather than
    // read the box's text.
    await expect(page.locator('[data-name="E2E capers"]')).toHaveCount(0);
  });
});
