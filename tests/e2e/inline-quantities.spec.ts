import { test, expect, type ChildProcess } from '@playwright/test';
import { spawn } from 'node:child_process';
import * as fs from 'node:fs';
import * as net from 'node:net';
import * as os from 'node:os';
import * as path from 'node:path';

// The shared dev server runs with the default, names only. This test starts
// a server of its own with `--inline-quantities`, on a recipe directory and a
// configuration directory of its own.
const COOK = path.resolve(__dirname, '../../target/debug/cook');

let server: ChildProcess | undefined;
let baseURL = '';

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

test.beforeAll(async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'cook-inline-quantities-'));
  const recipes = path.join(root, 'recipes');
  const config = path.join(root, 'config');
  fs.mkdirSync(recipes);
  fs.mkdirSync(config);
  fs.writeFileSync(
    path.join(recipes, 'Soup.cook'),
    '---\nservings: 2\n---\n\nBring @water{4%cups} to a boil with @garlic{1%clove}(lightly smashed) and @salt.\n',
  );
  const env = { ...process.env, COOK_CONFIG_DIR: config };
  const port = await freePort();
  baseURL = `http://127.0.0.1:${port}`;
  server = spawn(COOK, ['server', recipes, '--port', String(port), '--inline-quantities'], {
    env,
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
});

test.afterAll(() => {
  server?.kill();
});

test.describe('Inline quantities', () => {
  test('the step shows each scaled amount and the summary keeps only the notes', async ({ page }) => {
    await page.goto(`${baseURL}/recipe/Soup?servings=4`);
    const step = page.locator('.step-box').first();
    await expect(step.locator('.ingredient-badge')).toHaveText(['8 cups water', '2 clove garlic', 'salt']);
    await expect(step.locator('.step-refs')).toHaveText(/^\s*garlic\s*\(lightly smashed\)\s*$/);
    await expect(step.locator('.step-refs')).not.toContainText('cups');
  });
});
