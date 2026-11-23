import { test, expect, type Page } from '@playwright/test';
import fs from 'node:fs/promises';
import path from 'node:path';

const name = `Cooking notes acceptance ${process.pid}`;
const file = path.join(__dirname, '../../seed', `${name}.cook`);
const recipe = '= Sear\n\n> A\n\nSalt @chicken thighs{4}.\n\n> B\n\n> C\n\nFry in a #pan{} for ~{12%minutes}.\n\n> D\n\n= Rest\n\n> E\n';
const current = (page: Page) => page.locator('.cooking-card.current');

async function openRecipe(page: Page) {
  await page.goto(`/recipe/${encodeURIComponent(name)}.cook`);
  await page.locator('#start-cooking-btn').click();
  await expect(page.locator('#cooking-overlay')).toBeVisible();
}

async function next(page: Page) {
  await page.keyboard.press('ArrowRight');
}

async function swipe(page: Page, from: number, to: number) {
  for (const [type, y] of [['touchstart', from], ['touchmove', to], ['touchend', to]] as const) {
    await page.locator('#cooking-carousel').evaluate((el, { type, y }) => {
      el.dispatchEvent(new TouchEvent(type, {
        bubbles: true,
        touches: type === 'touchend' ? [] : [new Touch({ identifier: 1, target: el, clientX: 100, clientY: y })],
      }));
    }, { type, y });
  }
}

test.beforeAll(async () => { await fs.writeFile(file, recipe, { flag: 'wx' }); });
test.afterAll(async () => { await fs.rm(file, { force: true }); });

test('notes follow file order, group adjacent paragraphs and preserve step HTML', async ({ page }) => {
  await openRecipe(page);
  await expect(page.locator('.cooking-card')).toHaveCount(8);
  await expect(current(page)).toHaveClass(/cooking-card-section/);
  await next(page);
  await expect(current(page).locator('.cooking-note-text p')).toHaveText(['A']);
  await expect(page.locator('#cooking-status')).toHaveText('Note: Sear');
  const progress = await page.locator('#cooking-progress-bar').evaluate(el => parseFloat(el.style.width));
  expect(progress).toBeCloseTo(100 / 7);
  await next(page);
  await expect(current(page)).toHaveClass(/cooking-card-step/);
  await expect(current(page).locator('.cooking-step-text')).toContainText(/Salt\s+chicken thighs\s*\./);
  await expect(current(page).locator('.cooking-step-number')).toHaveText('1');
  await next(page);
  await expect(current(page).locator('.cooking-note-text p')).toHaveText(['B', 'C']);
  await expect(current(page).locator('.timer-badge, .cooking-step-ingredients, img, .cooking-step-number')).toHaveCount(0);
  await next(page);
  await expect(current(page).locator('.cooking-step-text')).toContainText(/Fry in a\s+pan/);
  await expect(current(page).locator('.cooking-step-number')).toHaveText('2');
  await expect(page.locator('#cooking-status')).toHaveText('Step 2 of 2');
  await expect(current(page).locator('[data-timer]')).toHaveCount(1);
  await next(page);
  await expect(current(page).locator('.cooking-note-text p')).toHaveText(['D']);
  await next(page);
  await expect(current(page).locator('.cooking-note-text p')).toHaveText(['E']);
  await expect(current(page)).toContainText('Rest');
  await expect(page.locator('.cooking-section-pill.active')).toHaveText('Rest');
  await next(page);
  await expect(current(page)).toHaveClass(/cooking-card-done/);
});

test('section navigation and starting at a step skip the right number of note cards', async ({ page }) => {
  await openRecipe(page);
  await page.locator('.cooking-section-pill', { hasText: 'Rest' }).click();
  await expect(current(page).locator('.cooking-note-text p')).toHaveText(['E']);
  await page.locator('.cooking-section-pill', { hasText: 'Sear' }).click();
  await expect(current(page)).toHaveClass(/cooking-card-section/);
  await page.keyboard.press('Escape');
  await page.evaluate(() => (window as unknown as { startCookingMode: (step: number) => void }).startCookingMode(2));
  await expect(current(page).locator('.cooking-step-number')).toHaveText('2');
  await expect(current(page).locator('.cooking-step-text')).toContainText(/Fry in a\s+pan/);
  await page.keyboard.press('ArrowLeft');
  await expect(current(page).locator('.cooking-note-text p')).toHaveText(['B', 'C']);
});

test('swipes cross note cards and a running timer survives navigation onto notes', async ({ page }) => {
  await openRecipe(page);
  await swipe(page, 400, 200);
  await expect(current(page).locator('.cooking-note-text p')).toHaveText(['A']);
  await next(page); await next(page); await next(page);
  await expect(current(page).locator('[data-timer]')).toBeVisible();
  await current(page).locator('[data-timer]').click();
  await expect(page.locator('#cooking-timer-strip [data-timer-chip]')).toHaveCount(1);
  await next(page);
  await expect(current(page).locator('.cooking-note-text p')).toHaveText(['D']);
  await expect(page.locator('#cooking-timer-strip [data-timer-chip]')).toHaveCount(1);
});

test('note text is escaped, safe links open separately, and long notes scroll', async ({ page }) => {
  await page.route(url => decodeURIComponent(url.pathname) === `/recipe/${name}.cook`, async route => {
    const response = await route.fetch();
    const html = (await response.text()).replace(
      /(<script id="cooking-mode-data" type="application\/json">)([\s\S]*?)(<\/script>)/,
      (_match, open: string, json: string, close: string) => {
        const data = JSON.parse(json);
        data.sections[0].items[0].text = '<img src=x onerror="window.__noteXss=1"> javascript:alert(1) https://example.com/guide?q=1&lang=en.\n' + 'Long note\n'.repeat(120);
        return open + JSON.stringify(data).replaceAll('</', '<\\/') + close;
      },
    );
    await route.fulfill({ response, body: html });
  });
  await openRecipe(page); await next(page);
  await expect(current(page)).toContainText('<img src=x onerror="window.__noteXss=1">');
  await expect(current(page).locator('img, script')).toHaveCount(0);
  const link = current(page).getByRole('link');
  await expect(link).toHaveCount(1);
  await expect(link).toHaveAttribute('href', 'https://example.com/guide?q=1&lang=en');
  await expect(link).toHaveAttribute('target', '_blank');
  await expect(link).toHaveAttribute('rel', 'noopener noreferrer');
  expect(await page.evaluate(() => (window as unknown as { __noteXss?: number }).__noteXss)).toBeUndefined();
  expect(await current(page).evaluate(el => el.scrollHeight > el.clientHeight)).toBe(true);
  await current(page).evaluate(el => { el.scrollTop = 50; });
  await swipe(page, 400, 300);
  await expect(current(page)).toHaveClass(/cooking-card-note/);
});

// Older static sites provide only the step list, with no ordered items.
test('legacy step-only data still starts at the ingredient card', async ({ page }) => {
  await page.route(url => decodeURIComponent(url.pathname) === `/recipe/${name}.cook`, async route => {
    const response = await route.fetch();
    const html = (await response.text()).replace(
      /(<script id="cooking-mode-data" type="application\/json">)([\s\S]*?)(<\/script>)/,
      (_match, open: string, json: string, close: string) => {
        const data = JSON.parse(json);
        for (const section of data.sections) {
          delete section.items;
          for (const step of section.steps) delete step.globalNumber;
        }
        return open + JSON.stringify(data) + close;
      },
    );
    await route.fulfill({ response, body: html });
  });
  await openRecipe(page);
  await expect(current(page)).toHaveClass(/cooking-card-section/);
  await expect(page.locator('.cooking-card-note')).toHaveCount(0);
  await next(page);
  await expect(current(page).locator('.cooking-step-text')).toContainText(/Salt\s+chicken thighs/);
});
