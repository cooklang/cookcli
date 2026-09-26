import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';

// Seed directory used by the dev server started by Playwright's `webServer`.
const SEED_DIR = path.resolve(__dirname, '../../seed');
const RECIPE_FILE = path.join(SEED_DIR, 'Neapolitan Pizza.cook');

test.describe('Recipe editor', () => {
  test('does not autosave when the page is merely opened', async ({ page }) => {
    const before = fs.statSync(RECIPE_FILE);
    const originalContent = fs.readFileSync(RECIPE_FILE, 'utf8');

    const saveRequests: string[] = [];
    page.on('request', request => {
      if (request.method() === 'PUT' && request.url().includes('/api/recipes/')) {
        saveRequests.push(request.url());
      }
    });

    await page.goto('/edit/Neapolitan Pizza.cook');
    await page.waitForLoadState('networkidle');

    // Wait past the autosave debounce so a spurious change would have fired.
    await expect(page.locator('#editor-container .cm-editor')).toBeVisible();
    await page.waitForTimeout(2000);

    await expect(page.locator('#save-status')).toHaveText('');
    expect(saveRequests).toEqual([]);

    const after = fs.statSync(RECIPE_FILE);
    expect(after.mtimeMs).toBe(before.mtimeMs);
    expect(fs.readFileSync(RECIPE_FILE, 'utf8')).toBe(originalContent);
  });
});

// A throwaway recipe in its own folder, so the pictures written here never
// touch a seed recipe another spec reads.
const PICTURE_DIR = path.join(SEED_DIR, 'E2E Picture Upload');
const PICTURE_RECIPE = path.join(PICTURE_DIR, 'Picture Test.cook');
const PICTURE_FILE = path.join(PICTURE_DIR, 'Picture Test.jpg');
const PICTURE_EDIT_URL = '/edit/E2E Picture Upload/Picture Test.cook';

// A 1×1 PNG with an alpha channel.
const PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==',
  'base64',
);

test.describe('Recipe editor title picture', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeAll(() => {
    fs.mkdirSync(PICTURE_DIR, { recursive: true });
    fs.writeFileSync(PICTURE_RECIPE, '---\ntitle: Picture Test\n---\n\nMix @flour{100%g}.\n');
  });

  test.afterAll(() => {
    fs.rmSync(PICTURE_DIR, { recursive: true, force: true });
  });

  test('uploads a picture, shows it, and removes it', async ({ page }) => {
    await page.goto(PICTURE_EDIT_URL);
    await page.getByRole('button', { name: 'Picture', exact: true }).click();

    const dialog = page.locator('#picture-modal');
    await expect(dialog.getByRole('dialog')).toBeVisible();
    await expect(dialog.locator('#picture-empty')).toBeVisible();
    await expect(dialog.locator('#picture-remove')).toBeHidden();

    await dialog.locator('#picture-input').setInputFiles({
      name: 'photo.png',
      mimeType: 'image/png',
      buffer: PNG,
    });

    const preview = dialog.locator('#picture-preview');
    await expect(preview).toBeVisible();
    await expect.poll(() => preview.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(1);
    await expect(dialog.locator('#picture-remove')).toBeVisible();

    // Stored as JPEG whatever was sent.
    expect(fs.readFileSync(PICTURE_FILE).subarray(0, 3)).toEqual(Buffer.from([0xff, 0xd8, 0xff]));

    // The recipe page picks it up. Each path segment is percent-encoded and
    // they are joined with `/` on every platform, Windows included (#548).
    const recipePage = await page.request.get('/recipe/E2E Picture Upload/Picture Test.cook');
    expect(await recipePage.text()).toContain('/api/static/E2E%20Picture%20Upload/Picture%20Test.jpg');

    await dialog.locator('#picture-remove').click();
    await expect(dialog.locator('#picture-confirm')).toBeVisible();
    await dialog.locator('#picture-confirm .btn-danger').click();

    await expect(dialog.locator('#picture-empty')).toBeVisible();
    await expect(preview).toBeHidden();
    expect(fs.existsSync(PICTURE_FILE)).toBe(false);
  });

  test('explains that a HEIC photo cannot be read', async ({ page }) => {
    await page.goto(PICTURE_EDIT_URL);
    await page.getByRole('button', { name: 'Picture', exact: true }).click();

    // What a HEIC file opens with; the server refuses it on these bytes alone.
    const heic = Buffer.concat([
      Buffer.from('\0\0\0\x18ftypheic\0\0\0\0mif1heic', 'latin1'),
      Buffer.alloc(64),
    ]);
    await page.locator('#picture-input').setInputFiles({
      name: 'IMG_0001.HEIC',
      mimeType: 'image/heic',
      buffer: heic,
    });

    await expect(page.getByText('Most Compatible')).toBeVisible();
    expect(fs.existsSync(PICTURE_FILE)).toBe(false);
  });

  test('scales a large picture down in the browser before sending it', async ({ page }) => {
    await page.goto(PICTURE_EDIT_URL);
    await page.getByRole('button', { name: 'Picture', exact: true }).click();

    // A 3000 × 1500 PNG, drawn by the browser: Node has no encoder at hand.
    const png = Buffer.from(
      await page.evaluate(async () => {
        const canvas = document.createElement('canvas');
        canvas.width = 3000;
        canvas.height = 1500;
        const context = canvas.getContext('2d')!;
        const gradient = context.createLinearGradient(0, 0, 3000, 1500);
        gradient.addColorStop(0, '#c0392b');
        gradient.addColorStop(1, '#2980b9');
        context.fillStyle = gradient;
        context.fillRect(0, 0, 3000, 1500);
        const blob: Blob = await new Promise(resolve => canvas.toBlob(b => resolve(b!), 'image/png'));
        return Array.from(new Uint8Array(await blob.arrayBuffer()));
      }),
    );

    // Playwright does not hand back a request body sent as a Blob, so note
    // what the page gives `fetch` instead.
    await page.evaluate(() => {
      const w = window as any;
      const original = window.fetch;
      w.sentPictures = [];
      window.fetch = async (input, init) => {
        if (init?.method === 'PUT' && init.body instanceof Blob) {
          const bitmap = await createImageBitmap(init.body);
          const head = new Uint8Array(await init.body.slice(0, 3).arrayBuffer());
          w.sentPictures.push({
            type: init.body.type,
            size: init.body.size,
            head: Array.from(head),
            width: bitmap.width,
            height: bitmap.height,
          });
        }
        return original(input, init);
      };
    });

    await page.locator('#picture-input').setInputFiles({
      name: 'large.png',
      mimeType: 'image/png',
      buffer: png,
    });

    // What crossed the network is already a JPEG, at the stored size.
    await expect.poll(() => page.evaluate(() => (window as any).sentPictures.length)).toBe(1);
    const sent = await page.evaluate(() => (window as any).sentPictures[0]);
    expect(sent.type).toBe('image/jpeg');
    expect(sent.head).toEqual([0xff, 0xd8, 0xff]);
    expect([sent.width, sent.height]).toEqual([2048, 1024]);

    const preview = page.locator('#picture-preview');
    await expect.poll(() => preview.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(2048);
  });
});
