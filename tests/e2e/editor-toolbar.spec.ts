import { test, expect, Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// Top-level `let` in templates/edit.html: a global binding, not a window property.
declare const editorView: any;

// Documents in these tests mark the selection with « » and a bare cursor
// with |. `setDoc` parses the markers out; `readDoc` puts them back.

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

async function click(page: Page, name: string) {
  await toolbar(page).getByRole('button', { name, exact: true }).click();
}

test.describe('Editor toolbar', () => {
  test.beforeEach(async ({ page }) => {
    // The editor autosaves one second after a change; never let that rewrite
    // the seed fixtures.
    await page.route('**/api/recipes/**', route =>
      route.request().method() === 'PUT' ? route.fulfill({ status: 200, body: '' }) : route.continue()
    );
    await page.goto('/edit/Neapolitan Pizza.cook');
    await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
  });

  const cases: Array<[string, string, string]> = [
    ['Ingredient', 'Add | now', 'Add @|{} now'],
    ['Ingredient', 'Add «salt» now', 'Add @salt{|} now'],
    ['Ingredient', 'Add «sea salt »now', 'Add @sea salt{|} now'],
    ['Cookware', 'Use a |', 'Use a #|{}'],
    ['Cookware', 'Use a «frying pan»', 'Use a #frying pan{|}'],
    ['Timer', 'Bake for |', 'Bake for ~{|%minutes}'],
    ['Timer', 'Let it «rest»', 'Let it ~rest{|%minutes}'],
    ['Section', 'Mix well.|', 'Mix well.\n\n== «Section» =='],
    ['Section', 'Step one.\n|\nStep two.', 'Step one.\n\n== «Section» ==\n\nStep two.'],
    ['Section', '«Dough»\nMix flour.', '== «Dough» ==\n\nMix flour.'],
    ['Section', 'Mix «Topping» now', 'Mix\n\n== «Topping» ==\n\nnow'],
    ['Note', 'Line |one', '> Line |one'],
    ['Note', '> Line |one', 'Line |one'],
    ['Note', '«a\nb»', '> «a\n> b»'],
    ['Note', '>> legacy: |meta', '> >> legacy: |meta'],
    ['Comment', '|Mix', '-- |Mix'],
    ['Comment', '-- Mix|', 'Mix|'],
    ['Comment', '«Mix\nStir»', '-- «Mix\n-- Stir»'],
    ['Comment', 'Mix «well» now', 'Mix «[- well -]» now'],
    ['Comment', 'Mix «[- well -]» now', 'Mix «well» now'],
    ['Comment', '«Mix well now»', '-- «Mix well now»'],
    ['Metadata', 'Mix.|', '---\ntitle: |\n---\n\nMix.'],
    ['Metadata', '', '---\ntitle: |\n---\n'],
    ['Metadata', '---\nservings: 2\n---\n\nMix.|', '---\nservings: 2\n|\n---\n\nMix.'],
  ];

  for (const [button, before, after] of cases) {
    test(`${button}: ${JSON.stringify(before)}`, async ({ page }) => {
      await setDoc(page, before || '|');
      await click(page, button);
      expect(await readDoc(page)).toBe(after);
      // Focus goes back to the editor so typing continues right away.
      await expect(page.locator('.cm-content')).toBeFocused();
    });
  }

  test('adds a metadata line to the recipe frontmatter', async ({ page }) => {
    const original = await page.evaluate(() => editorView.state.doc.toString());
    await click(page, 'Metadata');
    const doc = await readDoc(page);
    expect(doc).toBe(original.replace('difficulty: advanced\ntime: 20 min\n', 'difficulty: advanced\ntime: 20 min\n|\n'));
  });

  test('each action is a single undo step', async ({ page }) => {
    await setDoc(page, 'Add «salt»');
    await click(page, 'Ingredient');
    await click(page, 'Comment');
    expect(await readDoc(page)).toBe('-- Add @salt{|}');

    await page.keyboard.press('ControlOrMeta+z');
    expect((await readDoc(page)).replace(/[|«»]/g, '')).toBe('Add @salt{}');
    await page.keyboard.press('ControlOrMeta+z');
    expect((await readDoc(page)).replace(/[|«»]/g, '')).toBe('Add salt');
  });

  test('is a labelled ARIA toolbar with a single tab stop', async ({ page }) => {
    const bar = toolbar(page);
    await expect(bar).toHaveAttribute('data-mode', 'recipe');

    const buttons = bar.getByRole('button');
    await expect(buttons).toHaveCount(7);
    for (const button of await buttons.all()) {
      await expect(button).toHaveAttribute('aria-label', /.+/);
      await expect(button).toHaveAttribute('title', /.+/);
    }
    await expect(bar.locator('[tabindex="0"]')).toHaveCount(1);
  });

  test('can be driven from the keyboard', async ({ page }) => {
    const bar = toolbar(page);
    const button = (name: string) => bar.getByRole('button', { name, exact: true });

    // The toolbar sits right before the editor in the tab order.
    await page.locator('.cm-content').click();
    await page.keyboard.press('Shift+Tab');
    await expect(button('Ingredient')).toBeFocused();

    await page.keyboard.press('ArrowRight');
    await expect(button('Cookware')).toBeFocused();
    await page.keyboard.press('End');
    await expect(button('Metadata')).toBeFocused();
    await page.keyboard.press('ArrowRight');
    await expect(button('Ingredient')).toBeFocused();
    await page.keyboard.press('ArrowLeft');
    await expect(button('Metadata')).toBeFocused();
    await page.keyboard.press('Home');
    await expect(button('Ingredient')).toBeFocused();

    // The roving tab stop follows the focused button.
    await page.keyboard.press('ArrowRight');
    await expect(button('Cookware')).toHaveAttribute('tabindex', '0');
    await expect(button('Ingredient')).toHaveAttribute('tabindex', '-1');

    await setDoc(page, 'Use a «pan»');
    await button('Cookware').focus();
    await page.keyboard.press('Enter');
    expect(await readDoc(page)).toBe('Use a #pan{|}');
  });

  for (const theme of ['light', 'dark']) {
    test(`has no accessibility violations in the ${theme} theme`, async ({ page }) => {
      if (theme === 'dark') {
        await page.evaluate(() => document.documentElement.classList.add('dark'));
      }
      const results = await new AxeBuilder({ page })
        .include('#editor-toolbar')
        .withTags(['wcag2a', 'wcag2aa'])
        .analyze();
      expect(results.violations).toEqual([]);
    });
  }

  test('wraps at phone width without horizontal scroll', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 800 });
    const bar = toolbar(page);
    const box = await bar.boundingBox();
    expect(box!.x + box!.width).toBeLessThanOrEqual(375);
    const overflow = await bar.evaluate(el => el.scrollWidth - el.clientWidth);
    expect(overflow).toBeLessThanOrEqual(0);
    for (const button of await bar.getByRole('button').all()) {
      const b = await button.boundingBox();
      expect(b!.x + b!.width).toBeLessThanOrEqual(375);
    }
  });
});
