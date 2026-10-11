import { test, expect, type Page } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import * as fs from 'node:fs';
import * as net from 'node:net';
import * as os from 'node:os';
import * as path from 'node:path';

// A pantry file the shopping list cannot read has to be broken on purpose,
// which the shared dev server's ./seed must not be: these tests start a server
// of their own, on a recipe directory and a configuration directory of their
// own.
const COOK = path.resolve(__dirname, '../../target/debug/cook');

let server: ChildProcess | undefined;
let baseURL = '';
let pantryFile = '';

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const probe = net.createServer();
    probe.once('error', reject);
    probe.listen(0, '127.0.0.1', () => {
      const { port } = probe.address() as net.AddressInfo;
      probe.close(() => resolve(port));
    });
  });
}

test.describe.configure({ mode: 'serial' });

test.beforeAll(async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'cook-shopping-list-pantry-'));
  const recipes = path.join(root, 'recipes');
  const config = path.join(root, 'config');
  fs.mkdirSync(path.join(recipes, 'config'), { recursive: true });
  fs.mkdirSync(config);
  fs.writeFileSync(path.join(recipes, 'Pancakes.cook'), 'Mix @milk{1%l}, @flour{200%g} and @eggs{2}.\n');
  pantryFile = path.join(recipes, 'config', 'pantry.conf');
  fs.writeFileSync(pantryFile, '[dairy]\nmilk = "1%l"\n');

  const port = await freePort();
  baseURL = `http://127.0.0.1:${port}`;
  server = spawn(COOK, ['server', recipes, '--port', String(port)], {
    env: { ...process.env, COOK_CONFIG_DIR: config },
    stdio: 'ignore',
  });
  for (let tries = 0; ; tries++) {
    try {
      if ((await fetch(baseURL)).ok) break;
    } catch {
      // Not listening yet.
    }
    if (tries > 100) throw new Error('cook server did not start');
    await new Promise(resolve => setTimeout(resolve, 200));
  }

  const add = await fetch(`${baseURL}/api/shopping_list/add`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path: 'Pancakes.cook', scale: 1 }),
  });
  if (!add.ok) throw new Error(`adding the recipe answered ${add.status}`);
});

test.afterAll(() => {
  server?.kill();
});

async function openShoppingList(page: Page) {
  await page.goto(`${baseURL}/shopping-list`);
  await expect(page.locator('#list-content')).toContainText('flour');
}

test('a pantry file that does not parse is pointed out', async ({ page }) => {
  // An aisle.conf-style comment, which TOML does not take.
  fs.writeFileSync(pantryFile, '// my pantry\n[dairy]\nmilk = "1%l"\n');
  await openShoppingList(page);

  const box = page.locator('#pantry-unused');
  await expect(box).toBeVisible();
  await expect(box).toContainText('nothing in your pantry was taken off this list');
  await expect(box).toContainText('line 1');
  await expect(box).toContainText('// my pantry');
  await expect(box.getByRole('link', { name: 'Open the pantry to fix it' })).toHaveAttribute('href', '/pantry');
  // Made without the pantry, so milk is still to buy.
  await expect(page.locator('#list-content')).toContainText('milk');
});

test('the box goes once the pantry file reads', async ({ page }) => {
  fs.writeFileSync(pantryFile, '# my pantry\n[dairy]\nmilk = "1%l"\n');
  await openShoppingList(page);

  await expect(page.locator('#pantry-unused')).toBeHidden();
  await expect(page.locator('#pantry-items')).toContainText('milk');
});
