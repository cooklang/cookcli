import { test, expect, Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// Top-level `let` in templates/edit.html: a global binding, not a window property.
declare const editorView: any;
declare const CooklangEditor: any;

// Documents in these tests mark the selection with « » and a bare cursor
// with |, as in editor-toolbar.spec.ts.

async function setDoc(page: Page, marked: string) {
  let text = marked;
  let anchor: number;
  let head: number;
  if (text.includes('«')) {
    anchor = text.indexOf('«');
    text = text.replace('«', '');
    head = text.indexOf('»');
    text = text.replace('»', '');
  } else {
    anchor = head = text.indexOf('|');
    text = text.replace('|', '');
  }
  await page.evaluate(([doc, from, to]) => {
    const view = editorView;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: doc },
      selection: { anchor: from, head: to },
    });
  }, [text, anchor, head] as const);
}

async function readDoc(page: Page): Promise<string> {
  return page.evaluate(() => {
    const view = editorView;
    const doc: string = view.state.doc.toString();
    const { from, to } = view.state.selection.main;
    if (from === to) return doc.slice(0, from) + '|' + doc.slice(from);
    return doc.slice(0, from) + '«' + doc.slice(from, to) + '»' + doc.slice(to);
  });
}

function toolbar(page: Page) {
  return page.getByRole('toolbar', { name: 'Cooklang formatting' });
}

function button(page: Page, name: string) {
  return toolbar(page).getByRole('button', { name, exact: true });
}

function picker(page: Page) {
  return page.getByRole('dialog', { name: 'Choose a recipe' });
}

// Never let the editor's autosave rewrite the seed fixtures.
async function interceptSaves(page: Page) {
  await page.route('**/api/recipes/**', route =>
    route.request().method() === 'PUT' ? route.fulfill({ status: 200, body: '' }) : route.continue()
  );
}

async function openEditor(page: Page, path: string) {
  await page.goto(`/edit/${path}`);
  await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
}

// Search the picker and wait for `option` to be the highlighted result. Until
// the debounced search answers, the picker still shows the previous list.
async function searchPicker(page: Page, query: string, option: string) {
  const searched = page.waitForResponse(response =>
    response.url().includes(`/api/search?q=${encodeURIComponent(query)}`)
  );
  await picker(page).getByRole('combobox').fill(query);
  await searched;
  await expect(picker(page).getByRole('option', { selected: true })).toContainText(option);
}

test.describe('Menu editor toolbar', () => {
  test.beforeEach(async ({ page }) => {
    await interceptSaves(page);
    await openEditor(page, '2 Day Plan.menu');
  });

  test('is in menu mode', async ({ page }) => {
    await expect(toolbar(page)).toHaveAttribute('data-mode', 'menu');
    await expect(toolbar(page).getByRole('group', { name: 'Menu elements' })).toBeVisible();
    await expect(button(page, 'Recipe reference')).toHaveCount(0);
    await expect(toolbar(page).locator('[tabindex="0"]')).toHaveCount(1);
  });

  test('Day numbers the new day after the existing ones', async ({ page }) => {
    await setDoc(page, '---\nservings: 2\n---\n\n== Day 1 ==\n\nBreakfast: \\\n- @eggs{2}|');
    await button(page, 'Day').click();
    expect(await readDoc(page)).toBe(
      '---\nservings: 2\n---\n\n== Day 1 ==\n\nBreakfast: \\\n- @eggs{2}\n\n== «Day 2» =='
    );
    await expect(page.locator('.cm-content')).toBeFocused();
  });

  test('Day uses the picked date, then moves it on a day', async ({ page }) => {
    const date = toolbar(page).getByLabel('Date of the next day');
    await date.fill('2026-03-07');
    await setDoc(page, '|');
    await button(page, 'Day').click();
    expect(await readDoc(page)).toBe('== «Saturday (2026-03-07)» ==');
    await expect(date).toHaveValue('2026-03-08');
  });

  test('Meal opens a menu of meals and inserts the picked one', async ({ page }) => {
    await setDoc(page, '== Day 1 ==|');
    const meal = button(page, 'Meal');
    await expect(meal).toHaveAttribute('aria-expanded', 'false');
    await meal.click();
    await expect(meal).toHaveAttribute('aria-expanded', 'true');
    const menu = page.getByRole('menu');
    await expect(menu.getByRole('menuitem')).toHaveText(['Breakfast', 'Lunch', 'Dinner', 'Snacks']);
    await expect(menu.getByRole('menuitem', { name: 'Breakfast' })).toBeFocused();

    await menu.getByRole('menuitem', { name: 'Lunch' }).click();
    await expect(menu).toBeHidden();
    expect(await readDoc(page)).toBe('== Day 1 ==\n\nLunch: \\\n- |');
    await expect(page.locator('.cm-content')).toBeFocused();
  });

  test('Meal menu works from the keyboard', async ({ page }) => {
    await setDoc(page, '|');
    const meal = button(page, 'Meal');
    await meal.focus();
    await page.keyboard.press('ArrowDown');
    const menu = page.getByRole('menu');
    await expect(menu.getByRole('menuitem', { name: 'Breakfast' })).toBeFocused();
    await page.keyboard.press('ArrowUp');
    await expect(menu.getByRole('menuitem', { name: 'Snacks' })).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(menu).toBeHidden();
    await expect(meal).toBeFocused();
    expect(await readDoc(page)).toBe('|');

    await page.keyboard.press('Enter');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await expect(menu.getByRole('menuitem', { name: 'Dinner' })).toBeFocused();
    await page.keyboard.press('Enter');
    expect(await readDoc(page)).toBe('Dinner: \\\n- |');
  });

  // Where Add recipe puts the bullet, and the ` \` that keeps it in the meal.
  const items: Array<[string, string, string]> = [
    ['fills an empty bullet', 'Breakfast: \\\n- |', 'Breakfast: \\\n- @./Risotto{}|'],
    ['fills an empty bullet under a header without a continuation', 'Dinner:\n- |', 'Dinner: \\\n- @./Risotto{}|'],
    ['keeps the continuation of an empty bullet', 'Dinner: \\\n-| \\\n- @bread{}', 'Dinner: \\\n- @./Risotto{}| \\\n- @bread{}'],
    ['adds under a header', 'Dinner:|', 'Dinner: \\\n- @./Risotto{}|'],
    ['adds after the last bullet', 'Dinner: \\\n- @soup{}|', 'Dinner: \\\n- @soup{} \\\n- @./Risotto{}|'],
    ['adds between two bullets', 'Dinner: \\\n- @soup{}| \\\n- @bread{}', 'Dinner: \\\n- @soup{} \\\n- @./Risotto{}| \\\n- @bread{}'],
    ['adds to the meal above a blank line', 'Dinner: \\\n- @soup{}\n|\nLunch: \\\n- @salad{}', 'Dinner: \\\n- @soup{} \\\n- @./Risotto{}|\n\nLunch: \\\n- @salad{}'],
    ['starts a block of its own under a day', '== Day 1 ==\n\n|', '== Day 1 ==\n\n- @./Risotto{}|'],
    ['leaves a comment line alone', '-- leftovers|', '-- leftovers\n- @./Risotto{}|'],
  ];

  for (const [name, before, after] of items) {
    test(`Add recipe ${name}`, async ({ page }) => {
      await setDoc(page, before);
      await page.evaluate(() => CooklangEditor.insertMenuItem(editorView, '@./Risotto{}'));
      expect(await readDoc(page)).toBe(after);
    });
  }

  test('Add recipe picks a recipe with the menu servings', async ({ page }) => {
    await setDoc(page, '---\nservings: 2\n---\n\nDinner: \\\n- |');
    await button(page, 'Add recipe').click();
    const dialog = picker(page);
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole('combobox')).toBeFocused();
    await expect(dialog.getByLabel('Servings')).toHaveValue('2');

    await searchPicker(page, 'risotto', 'Classic Risotto alla Milanese');
    await page.keyboard.press('Enter');
    await expect(dialog).toBeHidden();
    expect(await readDoc(page)).toBe('---\nservings: 2\n---\n\nDinner: \\\n- @./Risotto{2%servings}|');
    await expect(page.locator('.cm-content')).toBeFocused();
  });

  test('Add recipe inserts {} when servings is left empty', async ({ page }) => {
    await setDoc(page, 'Dinner: \\\n- |');
    await button(page, 'Add recipe').click();
    const dialog = picker(page);
    await dialog.getByLabel('Servings').fill('');
    await dialog.getByRole('combobox').focus();
    await searchPicker(page, 'caprese', 'Caprese');
    await dialog.getByRole('option', { name: /Caprese/ }).click();
    expect(await readDoc(page)).toBe('Dinner: \\\n- @./Salads/Caprese{}|');
  });

  test('the picker lists recipes but never menus', async ({ page }) => {
    await button(page, 'Add recipe').click();
    const dialog = picker(page);
    const options = dialog.getByRole('option');

    // No query: the whole collection, from the recipe tree.
    await expect(options.filter({ hasText: './Breakfast/Easy Pancakes' })).toHaveCount(1);
    await expect(options.filter({ hasText: 'Plan' })).toHaveCount(0);

    // `pla` matches both menus as well as recipes.
    await searchPicker(page, 'pla', 'Lamb Chops');
    await expect(options.filter({ hasText: 'Easy Pancakes' })).toHaveCount(0);
    for (const text of await options.allTextContents()) {
      expect(text).not.toContain('Plan');
    }
  });

  test('the picker is driven from the keyboard', async ({ page }) => {
    await setDoc(page, 'Dinner:|');
    await button(page, 'Add recipe').click();
    const dialog = picker(page);
    const search = dialog.getByRole('combobox');
    await expect(dialog.getByRole('option').first()).toHaveAttribute('aria-selected', 'true');

    await page.keyboard.press('ArrowDown');
    const second = dialog.getByRole('option').nth(1);
    await expect(second).toHaveAttribute('aria-selected', 'true');
    await expect(search).toHaveAttribute('aria-activedescendant', (await second.getAttribute('id'))!);
    await page.keyboard.press('ArrowUp');
    await expect(dialog.getByRole('option').first()).toHaveAttribute('aria-selected', 'true');

    // Focus stays in the dialog.
    for (let i = 0; i < 6; i++) {
      await page.keyboard.press('Tab');
      expect(await dialog.evaluate(el => el.contains(document.activeElement))).toBe(true);
    }

    // Escape closes without inserting and goes back to the button.
    await page.keyboard.press('Escape');
    await expect(dialog).toBeHidden();
    await expect(button(page, 'Add recipe')).toBeFocused();
    expect(await readDoc(page)).toBe('Dinner:|');
  });

  for (const theme of ['light', 'dark']) {
    test(`has no accessibility violations in the ${theme} theme`, async ({ page }) => {
      if (theme === 'dark') {
        await page.evaluate(() => document.documentElement.classList.add('dark'));
      }
      await button(page, 'Meal').click();
      const toolbarResults = await new AxeBuilder({ page })
        .include('#editor-toolbar')
        .withTags(['wcag2a', 'wcag2aa'])
        .analyze();
      expect(toolbarResults.violations).toEqual([]);
      await page.keyboard.press('Escape');

      await button(page, 'Add recipe').click();
      await expect(picker(page).getByRole('option').first()).toBeVisible();
      const pickerResults = await new AxeBuilder({ page })
        .include('#recipe-picker')
        .withTags(['wcag2a', 'wcag2aa'])
        .analyze();
      expect(pickerResults.violations).toEqual([]);
    });
  }

  test('wraps at phone width without horizontal scroll', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 800 });
    const bar = toolbar(page);
    const overflow = await bar.evaluate(el => el.scrollWidth - el.clientWidth);
    expect(overflow).toBeLessThanOrEqual(0);
    for (const control of await bar.locator('button:visible, input').all()) {
      const box = await control.boundingBox();
      expect(box!.x + box!.width).toBeLessThanOrEqual(375);
    }
  });
});

test.describe('Recipe reference in the recipe editor', () => {
  test.beforeEach(async ({ page }) => {
    await interceptSaves(page);
    await openEditor(page, 'Neapolitan Pizza.cook');
  });

  test('inserts a reference to the picked recipe', async ({ page }) => {
    await setDoc(page, 'Stretch the |');
    await button(page, 'Recipe reference').click();
    const dialog = picker(page);
    await expect(dialog).toBeVisible();
    // A sub-recipe takes no servings here.
    await expect(dialog.getByLabel('Servings')).toBeHidden();

    await searchPicker(page, 'dough', 'Pizza Dough');
    await page.keyboard.press('Enter');
    expect(await readDoc(page)).toBe('Stretch the @./Shared/Pizza Dough{|}');
  });

  test('replaces the selection', async ({ page }) => {
    await setDoc(page, 'Make «the dough» first');
    await button(page, 'Recipe reference').click();
    await searchPicker(page, 'dough', 'Pizza Dough');
    await page.keyboard.press('Enter');
    expect(await readDoc(page)).toBe('Make @./Shared/Pizza Dough{|} first');
  });

  test('never offers the recipe being edited', async ({ page }) => {
    await button(page, 'Recipe reference').click();
    const options = picker(page).getByRole('option');
    await expect(options.first()).toBeVisible();
    await expect(options.filter({ hasText: './Neapolitan Pizza' })).toHaveCount(0);
  });
});

test.describe('New menu', () => {
  const name = `E2E Menu ${Date.now()}`;

  test.afterEach(async ({ request }) => {
    await request.delete(`/api/recipes/${encodeURIComponent(name)}.menu`);
  });

  test('is created from the listing, filled in with the toolbar and rendered as a menu', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: 'New Menu' }).click();
    await expect(page).toHaveURL(/\/new\?kind=menu$/);
    await expect(page.getByRole('heading', { name: 'New Menu' })).toBeVisible();
    await expect(page.getByText('.menu', { exact: true })).toBeVisible();

    await page.getByLabel('Menu path').fill(name);
    await page.getByRole('button', { name: 'Create Menu' }).click();
    await expect(page).toHaveURL(new RegExp(`/edit/${encodeURIComponent(name)}\\.menu$`));
    await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
    await expect(toolbar(page)).toHaveAttribute('data-mode', 'menu');
    expect(await page.evaluate(() => editorView.state.doc.toString())).toBe(
      `---\ntitle: ${name}\nservings: 2\n---\n\n== Day 1 ==\n\nBreakfast: \\\n- \n`
    );

    // Fill the starter bullet, then a dated day with a dinner.
    await page.evaluate(() => {
      const line = editorView.state.doc.line(editorView.state.doc.lines - 1);
      editorView.dispatch({ selection: { anchor: line.to } });
    });
    await button(page, 'Add recipe').click();
    await searchPicker(page, 'pancakes', 'Easy Pancakes');
    await page.keyboard.press('Enter');

    await page.evaluate(() => editorView.dispatch({ selection: { anchor: editorView.state.doc.length } }));
    await toolbar(page).getByLabel('Date of the next day').fill('2026-03-07');
    await button(page, 'Day').click();
    await page.evaluate(() => editorView.dispatch({ selection: { anchor: editorView.state.doc.length } }));
    await button(page, 'Meal').click();
    await page.getByRole('menuitem', { name: 'Dinner' }).click();
    await button(page, 'Add recipe').click();
    const dialog = picker(page);
    await dialog.getByLabel('Servings').fill('');
    await dialog.getByRole('combobox').focus();
    await searchPicker(page, 'risotto', 'Classic Risotto alla Milanese');

    // An earlier autosave may still be on its way; wait for the one with it all.
    const saved = page.waitForResponse(response =>
      response.request().method() === 'PUT' &&
      (response.request().postData() ?? '').includes('@./Risotto{}') &&
      response.ok()
    );
    await page.keyboard.press('Enter');
    expect(await page.evaluate(() => editorView.state.doc.toString())).toBe(
      `---\ntitle: ${name}\nservings: 2\n---\n\n== Day 1 ==\n\nBreakfast: \\\n- @./Breakfast/Easy Pancakes{2%servings}\n\n` +
      '== Saturday (2026-03-07) ==\n\nDinner: \\\n- @./Risotto{}'
    );
    await saved;

    await page.goto(`/recipe/${encodeURIComponent(name)}.menu`);
    const sections = page.locator('.menu-section');
    await expect(sections).toHaveCount(2);
    await expect(sections.nth(0).getByRole('heading', { level: 2 })).toHaveText('Day 1');
    await expect(sections.nth(0).getByRole('heading', { level: 3 })).toHaveText('Breakfast:');
    await expect(sections.nth(0).getByRole('link', { name: /Easy Pancakes/ })).toBeVisible();
    await expect(sections.nth(1).getByRole('heading', { level: 2 })).toHaveText('Saturday (2026-03-07)');
    await expect(sections.nth(1).getByRole('heading', { level: 3 })).toHaveText('Dinner:');
    await expect(sections.nth(1).getByRole('link', { name: 'Risotto' })).toBeVisible();
  });
});
