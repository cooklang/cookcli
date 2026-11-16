import { test, expect, type APIRequestContext, type Page } from '@playwright/test';
import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import * as fs from 'node:fs';
import * as net from 'node:net';
import * as os from 'node:os';
import * as path from 'node:path';

// The shared dev server has no users file, so nothing there asks to sign in.
// These tests start a server of their own with one, on a recipe directory and
// a configuration directory of their own.
const COOK = path.resolve(__dirname, '../../target/debug/cook');
const PASSWORD = 'secret';

let server: ChildProcess | undefined;
let baseURL = '';
let usersFile = '';
let hash = '';

function writeUsers(role: string) {
  fs.writeFileSync(usersFile, `[users]\nsam = { hash = "${hash}", role = "${role}" }\n`);
}

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
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'cook-sign-in-notice-'));
  const recipes = path.join(root, 'recipes');
  const config = path.join(root, 'config');
  fs.mkdirSync(recipes);
  fs.mkdirSync(config);
  fs.writeFileSync(path.join(recipes, 'Bread.cook'), 'Mix @flour{500%g} with @water{300%ml}.\n');
  const env = { ...process.env, COOK_CONFIG_DIR: config };
  hash = execFileSync(COOK, ['server', 'hash-password'], { input: `${PASSWORD}\n`, env })
    .toString()
    .trim();
  usersFile = path.join(config, 'users.toml');
  writeUsers('shopper');

  const port = await freePort();
  baseURL = `http://127.0.0.1:${port}`;
  server = spawn(COOK, ['server', recipes, '--port', String(port)], { env, stdio: 'ignore' });
  for (let tries = 0; ; tries++) {
    try {
      if ((await fetch(baseURL)).ok) break;
    } catch {
      // Not listening yet.
    }
    if (tries > 100) throw new Error('cook server did not start');
    await new Promise(resolve => setTimeout(resolve, 200));
  }
});

test.afterAll(() => {
  server?.kill();
});

// Signs in, puts Bread on the shopping list and opens it.
async function openShoppingList(page: Page) {
  await page.goto(`${baseURL}/login?next=/shopping-list`);
  await page.fill('input[name="username"]', 'sam');
  await page.fill('input[name="password"]', PASSWORD);
  await Promise.all([page.waitForURL(`${baseURL}/shopping-list`), page.click('button[type="submit"]')]);
  const add = await page.request.post(`${baseURL}/api/shopping_list/add`, {
    data: { path: 'Bread.cook', scale: 1 },
  });
  expect(add.ok()).toBeTruthy();
  await page.reload();
  return page.locator('[data-action="toggle-item"]').first();
}

async function roleApplies(request: APIRequestContext) {
  await expect
    .poll(async () => {
      const resp = await request.post(`${baseURL}/api/shopping_list/uncheck`, {
        data: { name: 'nothing' },
      });
      return resp.status();
    })
    .toBe(403);
}

test('a tick refused after signing out says so and offers to sign in', async ({ page }) => {
  writeUsers('shopper');
  const tick = await openShoppingList(page);
  const notice = page.locator('#refused-notice');
  await expect(notice).toBeHidden();

  // Signed out elsewhere: this page still shows the ticks.
  const out = await page.request.post(`${baseURL}/logout`);
  expect(out.status()).toBeLessThan(400);
  await tick.check();

  await expect(notice).toBeVisible();
  await expect(notice).toContainText('no longer signed in');
  const signIn = notice.getByRole('link', { name: 'Sign in' });
  await expect(signIn).toHaveAttribute('href', '/login?next=%2Fshopping-list');
  // Nothing was written, so the tick is taken back.
  await expect(tick).not.toBeChecked();

  await notice.getByRole('button', { name: 'Close' }).click();
  await expect(notice).toBeHidden();
});

test('a change refused for a lowered role says so without a sign-in link', async ({ page }) => {
  writeUsers('shopper');
  const tick = await openShoppingList(page);

  writeUsers('reader');
  await roleApplies(page.request);
  await tick.check();

  const notice = page.locator('#refused-notice');
  await expect(notice).toBeVisible();
  await expect(notice).toContainText("Your account can't do this");
  await expect(notice.getByRole('link', { name: 'Sign in' })).toBeHidden();
  await expect(tick).not.toBeChecked();
});
